use crate::backend::TimeslotBackend;
use crate::configuration::Configuration;
use crate::types::Timeslot;
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::sse::{Event, Sse};
use axum::response::{Html, Response};
use axum::routing::delete;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::{
    routing::{get, post},
    Router,
};
use axum_valid::Valid;
use chrono::{DateTime, Utc};
use futures::stream::{self, Stream};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use tokio::fs;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error};
use uuid::Uuid;
use validator::Validate;

// TODO_SD: Add validation to frontend
const VALID_NAMES: &str = r"^[\p{L}0-9 .!?-@_]+$";
const VALID_NOTES: &str = r"^[\p{L}0-9 .!?@_#%*\-()+=:~\n£€¥$¢]+$";

#[derive(Clone)]
pub struct AppState<T: TimeslotBackend, S: Configuration> {
    pub backend: T,
    pub configuration: S,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
struct BookingRequest {
    id: Uuid,
    #[validate(
        length(min = 1, max = 20),
        regex(path = Regex::new(VALID_NAMES).unwrap(), message = "Invalid characters in name")
    )]
    client_name: String,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
struct AddTimeslotRequest {
    datetime: DateTime<Utc>,
    #[validate(
        length(min = 0, max = 60),
        regex(path = Regex::new(VALID_NOTES).unwrap(), message = "Invalid characters in notes")
    )]
    notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteTimeslotRequest {
    id: Uuid,
}

pub fn create_app<T: TimeslotBackend, S: Configuration>(backend: T, configuration: S) -> Router {
    let state = AppState {
        backend,
        configuration,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public = Router::new()
        .route("/frontend", get(get_frontend))
        .route("/timeslots", get(get_timeslots))
        .route("/book", post(book_timeslot));

    let admin = Router::new()
        .route("/admin_page", get(get_admin_page))
        .route("/add", post(add_timeslot))
        .route("/remove", delete(remove_timeslot))
        .route("/remove_all", post(remove_all_timeslot))
        .route_layer(middleware::from_fn_with_state(state.clone(), admin_auth));

    Router::new()
        .merge(public)
        .merge(admin)
        .with_state(state)
        .layer(cors)
}

async fn admin_auth<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let password = state.configuration.password();

    if let Some(auth_header) = request.headers().get("x-admin-password") {
        if auth_header.to_str().unwrap_or("") != password {
            error!("Authorization failed");
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    } else {
        error!("Authorization failed: Missing credentials");
        return Err((StatusCode::UNAUTHORIZED, "Missing credentials".to_string()));
    }
    Ok(next.run(request).await)
}

async fn get_timeslots<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    debug!("Starting SSE timeslot stream");

    Sse::new(
        state
            .backend
            .timeslot_stream()
            .map(|timeslots| Ok(Event::default().json_data(timeslots).unwrap())),
    )
}

async fn book_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(booking): Json<BookingRequest>,
) -> impl IntoResponse {
    debug!("Book timeslot");
    if let Err(err) = booking.validate() {
        error!(?err, "Invalid input");
        return (StatusCode::BAD_REQUEST, format!("Invalid input: {:?}", err));
    }

    match state.backend.book_timeslot(booking.id, booking.client_name) {
        Ok(()) => (StatusCode::OK, "Timeslot booked successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

// TODO_SD: Filter out special characters, limit length
async fn add_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(timeslot): Json<AddTimeslotRequest>,
) -> impl IntoResponse {
    debug!("Add timeslot");

    if let Err(err) = timeslot.validate() {
        error!(?err, "Invalid input");
        return (StatusCode::BAD_REQUEST, format!("Invalid input: {:?}", err));
    }

    match state
        .backend
        .add_timeslot(timeslot.datetime, timeslot.notes)
    {
        Ok(()) => (StatusCode::OK, "Timeslot added successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn remove_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(timeslot): Json<DeleteTimeslotRequest>,
) -> impl IntoResponse {
    debug!("Remove timeslot");
    match state.backend.remove_timeslot(timeslot.id) {
        Ok(()) => (StatusCode::OK, "Timeslot removed successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn remove_all_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> impl IntoResponse {
    debug!("Remove all timeslots");
    match state.backend.remove_all_timeslot() {
        Ok(()) => (
            StatusCode::OK,
            "All timeslots removed successfully".to_string(),
        ),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn get_frontend<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> Result<Html<String>, (StatusCode, String)> {
    debug!("Get frontend");
    let title = state.configuration.website_title();
    let path = state.configuration.frontend_path();
    let port = state.configuration.port();

    match fs::read_to_string(path).await {
        Ok(contents) => {
            let contents = contents.replace("generic_timeslot_booking_manager_name", &title);
            let contents = contents.replace("localhost:PORT", &format!("localhost:{port}"));
            Ok(Html(contents))
        }
        Err(e) => {
            let error_message = format!("Failed to read frontend file: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

async fn get_admin_page() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testutils::{MockConfiguration, MockTimeslotBackend};
    use axum::http::{response, StatusCode};
    use axum::serve::Serve;
    use chrono::Local;
    use futures::TryStreamExt;
    use mockall::predicate::*;
    use reqwest::Client;
    use std::io::Write;
    use std::net::SocketAddr;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering, time::Duration};
    use tempfile::NamedTempFile;
    use tokio::net::TcpListener;
    use tokio::{task::JoinHandle, time::sleep};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct EmptyRequest {}

    fn assert_backend_calls(
        mock_backend: MockTimeslotBackend,
        path: &str,
        expected_backend_calls: u64,
    ) {
        match path {
            "book" => assert_eq!(
                mock_backend.0.calls_to_book_timeslot.load(Ordering::SeqCst),
                expected_backend_calls
            ),
            "timeslots" => assert_eq!(
                mock_backend.0.calls_to_timeslots.load(Ordering::SeqCst),
                expected_backend_calls
            ),
            "add" => assert_eq!(
                mock_backend.0.calls_to_add_timeslot.load(Ordering::SeqCst),
                expected_backend_calls
            ),
            "remove" => assert_eq!(
                mock_backend
                    .0
                    .calls_to_remove_timeslot
                    .load(Ordering::SeqCst),
                expected_backend_calls
            ),
            "remove_all" => assert_eq!(
                mock_backend
                    .0
                    .calls_to_remove_all_timeslot
                    .load(Ordering::SeqCst),
                expected_backend_calls
            ),
            "admin_page" => {} // No related backend call
            _ => unimplemented!(),
        }
    }

    async fn init() -> (
        JoinHandle<Result<(), std::io::Error>>,
        SocketAddr,
        MockTimeslotBackend,
        MockConfiguration,
    ) {
        let mock_backend = MockTimeslotBackend::new();
        let mock_configuration = MockConfiguration::new();

        let app = create_app(mock_backend.clone(), mock_configuration.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let join = tokio::spawn(async move { axum::serve(listener, app).await });

        (join, addr, mock_backend, mock_configuration)
    }

    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, true)]
    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, false)]
    #[test_case::test_case ("add", AddTimeslotRequest { datetime: Utc::now(), notes: String::from("Example Notes") }, true)]
    #[test_case::test_case ("remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, true)]
    #[test_case::test_case ("remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, false)]
    #[test_case::test_case ("remove_all", EmptyRequest {  }, true)]
    #[tokio::test]
    async fn test_access_backend<T>(path: &str, request: T, backend_success: bool)
    where
        T: Serialize,
    {
        let (server, addr, mock_backend, mock_configuration) = init().await;
        let password = String::from("123");
        *mock_configuration.0.password.lock().unwrap() = password.clone();
        mock_backend
            .0
            .success
            .store(backend_success, Ordering::SeqCst);

        let client = Client::new();

        let request_builder = if path == "remove" {
            client.delete(format!("http://{addr}/{path}"))
        } else {
            client.post(format!("http://{addr}/{path}"))
        }
        .header("x-admin-password", password);
        let response = request_builder.json(&request).send().await.unwrap();

        if backend_success {
            assert_eq!(response.status(), StatusCode::OK.as_u16());
        } else {
            assert_eq!(
                response.status(),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16()
            );
        }

        assert_backend_calls(mock_backend, path, 1);
        server.abort();
    }

    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("\n") })]
    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("") })]
    #[test_case::test_case ("add", AddTimeslotRequest { datetime: Utc::now(), notes: String::from("'") })]
    #[tokio::test]
    async fn test_invalid_input<T>(path: &str, request: T)
    where
        T: Serialize,
    {
        let (server, addr, mock_backend, mock_configuration) = init().await;
        let password = String::from("123");
        *mock_configuration.0.password.lock().unwrap() = password.clone();
        mock_backend.0.success.store(false, Ordering::SeqCst);

        let client = Client::new();

        let request_builder = client
            .post(format!("http://{addr}/{path}"))
            .header("x-admin-password", password);
        let response = request_builder.json(&request).send().await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST.as_u16());

        assert_backend_calls(mock_backend, path, 0);
        server.abort();
    }

    enum Authorization {
        None,
        Invalid,
        Valid,
    }

    #[test_case::test_case ("post", "book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, Authorization::None, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, Authorization::Invalid, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, Authorization::Valid, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "add", AddTimeslotRequest { datetime: Utc::now(), notes: String::from("Example Notes") }, Authorization::None, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "add", AddTimeslotRequest { datetime: Utc::now(), notes: String::from("Example Notes") }, Authorization::Invalid, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "add", AddTimeslotRequest { datetime: Utc::now(), notes: String::from("Example Notes") }, Authorization::Valid, 1, StatusCode::OK)]
    #[test_case::test_case ("delete", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, Authorization::None, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("delete", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, Authorization::Valid, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "remove_all", EmptyRequest {  }, Authorization::None, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "remove_all", EmptyRequest {  }, Authorization::Valid, 1, StatusCode::OK)]
    #[test_case::test_case ("get", "admin_page", EmptyRequest {  }, Authorization::None, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("get", "admin_page", EmptyRequest {  }, Authorization::Valid, 0,StatusCode::OK)]
    #[tokio::test]
    async fn test_authorization<T>(
        method: &str,
        path: &str,
        request: T,
        authorization: Authorization,
        expected_backend_calls: u64,
        status_code: StatusCode,
    ) where
        T: Serialize,
    {
        let (server, addr, mock_backend, mock_configuration) = init().await;
        let password = String::from("123");
        let wrong_password = String::from("xyz");
        *mock_configuration.0.password.lock().unwrap() = password.clone();

        let client = Client::new();
        let mut request_builder = match method.to_lowercase().as_str() {
            "get" => client.get(format!("http://{addr}/{path}")),
            "post" => client.post(format!("http://{addr}/{path}")),
            "delete" => client.delete(format!("http://{addr}/{path}")),
            _ => panic!("Unsupported HTTP method: {}", method),
        };
        request_builder = match authorization {
            Authorization::None => request_builder,
            Authorization::Invalid => request_builder.header("x-admin-password", wrong_password),
            Authorization::Valid => request_builder.header("x-admin-password", password),
        };
        let response = request_builder.json(&request).send().await.unwrap();

        assert_eq!(response.status(), status_code.as_u16());
        assert_backend_calls(mock_backend, path, expected_backend_calls);
        server.abort();
    }

    #[tokio::test]
    async fn test_get_frontend() {
        let (server, addr, _, mock_configuration) = init().await;

        let mut tmp_file = NamedTempFile::new().unwrap();
        let expected_html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body><h1>Test</h1></body>
</html>"#;
        write!(tmp_file, "{}", expected_html).unwrap();
        *mock_configuration.0.frontend_path.lock().unwrap() = tmp_file.path().to_path_buf();

        let client = Client::new();
        let response = client
            .get(format!("http://{addr}/frontend"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK.as_u16());
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html; charset=utf-8"
        );

        let actual_html = response.text().await.unwrap();
        assert_eq!(actual_html, expected_html);

        server.abort();
    }

    // TODO_SD: Fix test
    // #[tokio::test]
    // async fn test_get_timeslots() {
    //     let (server, addr, mock_backend, _) = init().await;

    //     let timeslots = vec![
    //         Timeslot {
    //             id: Uuid::new_v4(),
    //             datetime: Utc::now(),
    //             available: true,
    //             booker_name: String::new(),
    //             notes: "First Timeslot".into(),
    //         },
    //         Timeslot {
    //             id: Uuid::new_v4(),
    //             datetime: Utc::now(),
    //             available: false,
    //             booker_name: "Stefan".into(),
    //             notes: "Second Timeslot".into(),
    //         },
    //     ];

    //     let client = Client::new();
    //     let response = client
    //         .get(format!("http://{addr}/timeslots"))
    //         .send()
    //         .await
    //         .unwrap();

    //     // mock_backend.0.timeslot_sender.send(timeslots).unwrap();

    //     assert_eq!(response.status(), StatusCode::OK.as_u16());
    //     assert_eq!(
    //         response
    //             .headers()
    //             .get("content-type")
    //             .unwrap()
    //             .to_str()
    //             .unwrap(),
    //         "text/event-stream"
    //     );

    //     let mut sse_stream = response.bytes_stream().into_async_read();

    //     // Create an SSE decoder
    //     let mut decoder = async_sse::decode(&mut sse_stream);

    //     // Send the test data (this should trigger an SSE event)
    //     mock_backend
    //         .0
    //         .timeslot_sender
    //         .send(timeslots.clone())
    //         .unwrap();

    //     // Read the first event
    //     if let Some(event) = decoder.next().await {
    //         let event = event.unwrap();
    //         assert_eq!(event.name(), "message"); // or whatever event name you're using

    //         // Parse the JSON data
    //         let received_timeslots: Vec<Timeslot> = from_str(&event.data()).unwrap();
    //         assert_eq!(received_timeslots, timeslots);
    //     } else {
    //         panic!("No SSE event received");
    //     }

    //     // let response_content = response.text().await.unwrap();
    //     // let response_content: Vec<Timeslot> = serde_json::from_str(&response_content).unwrap();
    //     // assert_eq!(response_content.len(), 2);
    //     // assert!(response_content.contains(&timeslots[0]));
    //     // assert!(response_content.contains(&timeslots[1]));

    //     server.abort();
    // }
}
