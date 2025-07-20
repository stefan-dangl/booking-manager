use crate::backend::TimeslotBackend;
use crate::configuration::Configuration;
use crate::types::Timeslot;
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::{Html, Response};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::{
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState<T: TimeslotBackend, S: Configuration> {
    pub backend: T,
    pub configuration: S,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookingRequest {
    id: Uuid,
    client_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteTimeslotRequest {
    id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddTimeslotRequest {
    datetime: DateTime<Utc>,
    notes: String,
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
        .route("/remove", post(remove_timeslot))
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
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Missing credentials".to_string()));
    }
    Ok(next.run(request).await)
}

async fn get_timeslots<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> impl IntoResponse {
    Json(state.backend.timeslots())
}

async fn book_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(booking): Json<BookingRequest>,
) -> impl IntoResponse {
    match state.backend.book_timeslot(booking.id, booking.client_name) {
        Ok(()) => (StatusCode::OK, "Timeslot booked successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn add_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(timeslot): Json<AddTimeslotRequest>,
) -> impl IntoResponse {
    state
        .backend
        .add_timeslot(timeslot.datetime, timeslot.notes);

    (StatusCode::OK, "Timeslot added successfully".to_string())
}

async fn remove_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
    Json(timeslot): Json<DeleteTimeslotRequest>,
) -> impl IntoResponse {
    match state.backend.remove_timeslot(timeslot.id) {
        Ok(()) => (StatusCode::OK, "Timeslot removed successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn remove_all_timeslot<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> impl IntoResponse {
    state.backend.remove_all_timeslot();
    (
        StatusCode::OK,
        "All timeslots removed successfully".to_string(),
    )
}

async fn get_frontend<T: TimeslotBackend, S: Configuration>(
    State(state): State<AppState<T, S>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let path = state.configuration.frontend_path();

    match fs::read_to_string(path).await {
        Ok(contents) => Ok(Html(contents)),
        Err(e) => {
            let error_message = format!("Failed to read frontend file: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

async fn get_admin_page() -> impl IntoResponse {
    println!("get admin_page called");
    StatusCode::OK
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testutils::{MockConfiguration, MockTimeslotBackend};
    use axum::http::{response, StatusCode};
    use axum::serve::Serve;
    use chrono::Local;
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
        let response = client
            .post(format!("http://{addr}/{path}"))
            .header("x-admin-password", password)
            .json(&request)
            .send()
            .await
            .unwrap();

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
    #[test_case::test_case ("post", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, Authorization::None, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, Authorization::Valid, 1, StatusCode::OK)]
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
            "put" => client.put(format!("http://{addr}/{path}")),
            "delete" => client.delete(format!("http://{addr}/{path}")), // TODO_SD: Make Remove requests delete instead of post?
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

    #[tokio::test]
    async fn test_get_timeslots() {
        let (server, addr, mock_backend, _) = init().await;

        let timeslots = vec![
            Timeslot {
                id: Uuid::new_v4(),
                datetime: Utc::now(),
                available: true,
                booker_name: String::new(),
                notes: "First Timeslot".into(),
            },
            Timeslot {
                id: Uuid::new_v4(),
                datetime: Utc::now(),
                available: false,
                booker_name: "Stefan".into(),
                notes: "Second Timeslot".into(),
            },
        ];
        *mock_backend.0.timeslots.lock().unwrap() = timeslots.clone();

        let client = Client::new();
        let response = client
            .get(format!("http://{addr}/timeslots"))
            .send()
            .await
            .unwrap();

        println!("got response");

        assert_eq!(response.status(), StatusCode::OK.as_u16());
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "application/json"
        );

        let response_content = response.text().await.unwrap();
        let response_content: Vec<Timeslot> = serde_json::from_str(&response_content).unwrap();
        assert_eq!(response_content.len(), 2);
        assert!(response_content.contains(&timeslots[0]));
        assert!(response_content.contains(&timeslots[1]));

        server.abort();
    }
}
