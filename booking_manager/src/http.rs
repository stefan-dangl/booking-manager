use crate::backend::TimeslotBackend;
use crate::types::Timeslot;
use crate::AppState;
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::{Html, Response};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::{
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

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
    datetime: DateTime<Local>,
    notes: String,
}

pub async fn start_server<T: TimeslotBackend>(state: AppState<T>) {
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
        .route_layer(middleware::from_fn(admin_auth));

    let app = Router::new()
        .merge(public)
        .merge(admin)
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn admin_auth(request: Request<Body>, next: Next) -> Result<Response, (StatusCode, String)> {
    // TODO: Read from env variable
    // const ADMIN_PASSWORD: &str = std::env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");
    const ADMIN_PASSWORD: &str = "123";

    if let Some(auth_header) = request.headers().get("x-admin-password") {
        if auth_header.to_str().unwrap_or("") != ADMIN_PASSWORD {
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Missing credentials".to_string()));
    }
    Ok(next.run(request).await)
}

async fn get_timeslots<T: TimeslotBackend>(State(state): State<AppState<T>>) -> impl IntoResponse {
    let timeslot_values: Vec<Timeslot> = state
        .timeslot_manager
        .timeslots()
        .values()
        .cloned()
        .collect();
    Json(timeslot_values)
}

async fn book_timeslot<T: TimeslotBackend>(
    State(state): State<AppState<T>>,
    Json(booking): Json<BookingRequest>,
) -> impl IntoResponse {
    match state
        .timeslot_manager
        .book_timeslot(booking.id, booking.client_name)
    {
        Ok(()) => (StatusCode::OK, "Timeslot booked successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn add_timeslot<T: TimeslotBackend>(
    State(state): State<AppState<T>>,
    Json(timeslot): Json<AddTimeslotRequest>,
) -> impl IntoResponse {
    state
        .timeslot_manager
        .add_timeslot(timeslot.datetime, timeslot.notes);

    (StatusCode::OK, "Timeslot added successfully".to_string())
}

async fn remove_timeslot<T: TimeslotBackend>(
    State(state): State<AppState<T>>,
    Json(timeslot): Json<DeleteTimeslotRequest>,
) -> impl IntoResponse {
    match state.timeslot_manager.remove_timeslot(timeslot.id) {
        Ok(()) => (StatusCode::OK, "Timeslot removed successfully".to_string()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn remove_all_timeslot<T: TimeslotBackend>(
    State(state): State<AppState<T>>,
) -> impl IntoResponse {
    state.timeslot_manager.remove_all_timeslot();
    (
        StatusCode::OK,
        "All timeslots removed successfully".to_string(),
    )
}

async fn get_frontend() -> Result<Html<String>, (StatusCode, String)> {
    println!("get frontend called");

    // Construct the path to the frontend file
    let path = Path::new("../frontend/index.html");

    // Read the file asynchronously
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
    use crate::testutils::MockTimeslotBackend;
    use axum::http::{response, StatusCode};
    use chrono::Local;
    use mockall::predicate::*;
    use reqwest::Client;
    use std::{collections::HashMap, sync::atomic::Ordering, time::Duration};
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

    fn init() -> (JoinHandle<()>, MockTimeslotBackend) {
        let mock_backend = MockTimeslotBackend::new();
        let state = AppState {
            timeslot_manager: mock_backend.clone(),
        };
        (tokio::spawn(start_server(state)), mock_backend)
    }

    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, true)]
    #[test_case::test_case ("book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, false)]
    #[test_case::test_case ("add", AddTimeslotRequest { datetime: Local::now(), notes: String::from("Example Notes") }, true)]
    #[test_case::test_case ("remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, true)]
    #[test_case::test_case ("remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, false)]
    #[test_case::test_case ("remove_all", EmptyRequest {  }, true)]
    #[tokio::test]
    async fn test_access_backend<T>(path: &str, request: T, backend_success: bool)
    where
        T: Serialize,
    {
        let (server, mock_backend) = init();
        mock_backend
            .0
            .success
            .store(backend_success, Ordering::SeqCst);

        let client = Client::new();
        let response = client
            .post(format!("http://localhost:3000/{path}"))
            .header("x-admin-password", "123") // TODO_SD: authenticate every request
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

    #[test_case::test_case ("post", "book", BookingRequest { id: Uuid::new_v4(), client_name: String::from("Stefan") }, false, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "add", AddTimeslotRequest { datetime: Local::now(), notes: String::from("Example Notes") }, false, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "add", AddTimeslotRequest { datetime: Local::now(), notes: String::from("Example Notes") }, true, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, false, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "remove", DeleteTimeslotRequest { id: Uuid::new_v4() }, true, 1, StatusCode::OK)]
    #[test_case::test_case ("post", "remove_all", EmptyRequest {  }, false, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("post", "remove_all", EmptyRequest {  }, true, 1, StatusCode::OK)]
    #[test_case::test_case ("get", "admin_page", EmptyRequest {  }, false, 0, StatusCode::UNAUTHORIZED)]
    #[test_case::test_case ("get", "admin_page", EmptyRequest {  }, true, 0,StatusCode::OK)]
    #[tokio::test]
    async fn test_authorization<T>(
        method: &str,
        path: &str,
        request: T,
        authorized: bool,
        expected_backend_calls: u64,
        status_code: StatusCode,
    ) where
        T: Serialize,
    {
        let (server, mock_backend) = init();

        let client = Client::new();
        let mut request_builder = match method.to_lowercase().as_str() {
            "get" => client.get(format!("http://localhost:3000/{path}")),
            "post" => client.post(format!("http://localhost:3000/{path}")),
            "put" => client.put(format!("http://localhost:3000/{path}")),
            "delete" => client.delete(format!("http://localhost:3000/{path}")), // TODO_SD: Make Remove requests delete instead of post?
            _ => panic!("Unsupported HTTP method: {}", method),
        };
        if authorized {
            request_builder = request_builder.header("x-admin-password", "123");
        }
        let response = request_builder.json(&request).send().await.unwrap();

        assert_eq!(response.status(), status_code.as_u16());
        assert_backend_calls(mock_backend, path, expected_backend_calls);
        server.abort();
    }

    #[tokio::test]
    async fn test_get_frontend() {
        let (server, _) = init();

        let client = Client::new();
        let response = client
            .get("http://localhost:3000/frontend")
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

        // TODO: Check html content
        let html_content = response.text().await.unwrap();
        println!("Received HTML content:\n{}", html_content);

        server.abort();
    }

    #[tokio::test]
    async fn test_get_timeslots() {
        let (server, mock_backend) = init();

        let timeslot_1 = Timeslot {
            id: Uuid::new_v4(),
            datetime: Local::now(),
            available: true,
            booker_name: String::new(),
            notes: "First Timeslot".into(),
        };
        let timeslot_2 = Timeslot {
            id: Uuid::new_v4(),
            datetime: Local::now(),
            available: false,
            booker_name: "Stefan".into(),
            notes: "Second Timeslot".into(),
        };

        let mut timeslots = HashMap::new();
        timeslots.insert(timeslot_1.id, timeslot_1.clone());
        timeslots.insert(timeslot_2.id, timeslot_2.clone());
        *mock_backend.0.timeslots.lock().unwrap() = timeslots;

        let client = Client::new();
        let response = client
            .get("http://localhost:3000/timeslots")
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
            "application/json"
        );

        let response_content = response.text().await.unwrap();
        let response_content: Vec<Timeslot> = serde_json::from_str(&response_content).unwrap();
        assert_eq!(response_content.len(), 2);
        assert!(response_content.contains(&timeslot_1));
        assert!(response_content.contains(&timeslot_2));

        server.abort();
    }
}
