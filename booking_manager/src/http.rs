use crate::timeslot_manager::Timeslot;
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
struct BookingResponse {
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteTimeslotRequest {
    id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteTimeslotResponse {
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddTimeslotRequest {
    datetime: DateTime<Local>,
    notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddTimeslotResponse {
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteAllTimeslotsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteAllTimeslotsResponse {
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminRequest {
    password: String,
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

pub async fn start_server(state: AppState) {
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

async fn get_timeslots(State(state): State<AppState>) -> impl IntoResponse {
    let timeslots = state.timeslot_manager.timeslots();
    let timeslots = timeslots.lock().unwrap();
    let timeslot_values: Vec<Timeslot> = timeslots.values().cloned().collect();
    Json(timeslot_values)
}

async fn book_timeslot(
    State(state): State<AppState>,
    Json(booking): Json<BookingRequest>,
) -> Result<Json<BookingResponse>, (StatusCode, String)> {
    let response_message = match state
        .timeslot_manager
        .book_timeslot(booking.id, booking.client_name)
    {
        Ok(()) => "success".into(),
        Err(err) => err,
    };

    Ok(BookingResponse {
        message: response_message,
    }
    .into())
}

async fn add_timeslot(
    State(state): State<AppState>,
    Json(timeslot): Json<AddTimeslotRequest>,
) -> Result<Json<AddTimeslotResponse>, (StatusCode, String)> {
    state
        .timeslot_manager
        .add_timeslot(timeslot.datetime, timeslot.notes);

    Ok(AddTimeslotResponse {
        message: "done".into(),
    }
    .into())
}

async fn remove_timeslot(
    State(state): State<AppState>,
    Json(timeslot): Json<DeleteTimeslotRequest>,
) -> Result<Json<DeleteTimeslotResponse>, (StatusCode, String)> {
    println!("timeslot: {timeslot:?}");

    let response_message = match state.timeslot_manager.remove_timeslot(timeslot.id) {
        Ok(()) => "success".into(),
        Err(err) => err,
    };

    Ok(DeleteTimeslotResponse {
        message: response_message,
    }
    .into())
}

async fn remove_all_timeslot(
    State(state): State<AppState>,
    Json(booking): Json<DeleteAllTimeslotsRequest>,
) -> Result<Json<DeleteAllTimeslotsResponse>, (StatusCode, String)> {
    println!("remove all timeslots called");

    state.timeslot_manager.remove_all_timeslot();

    Ok(DeleteAllTimeslotsResponse {
        message: "done".into(),
    }
    .into())
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
