use std::path::Path;

use crate::timeslot_manager::Timeslot;
use crate::AppState;
use axum::response::Html;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::{
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookingRequest {
    datetime: DateTime<Local>,
    client_name: String,
    notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookingResponse {
    message: String,
    booked_slot: Timeslot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteTimeslotRequest {
    datetime: DateTime<Local>,
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

pub async fn start_server(state: AppState) {
    // Set up CORS
    // ODO: In production, you'd specify exact domains like .allow_origin("https://yourdomain.com".parse().unwrap())
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any) // Permits all HTTP methods (GET, POST, etc.)
        .allow_headers(Any); // Accepts all request headers

    let app = Router::new()
        .route("/frontend", get(get_frontend))
        .route("/admin_page", get(get_admin_page))
        .route("/timeslots", get(get_timeslots))
        .route("/add", post(add_timeslot))
        .route("/book", post(book_timeslot))
        .route("/remove", post(remove_timeslot))
        .route("/remove_all", post(remove_all_timeslot))
        .with_state(state)
        .layer(cors);

    // TODO: Read from config file
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!(
        // TODO: Use proper logging
        "Server running on http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}

async fn get_timeslots(State(state): State<AppState>) -> impl IntoResponse {
    let timeslots = state.timeslot_manager.timeslots();
    let timeslots = timeslots.lock().unwrap();
    let available_timeslots: Vec<Timeslot> = timeslots
        .values()
        .filter(|slot| slot.available)
        .cloned()
        .collect();
    Json(available_timeslots)
}

async fn book_timeslot(
    State(state): State<AppState>,
    Json(booking): Json<BookingRequest>,
) -> Result<Json<BookingResponse>, (StatusCode, String)> {
    let timeslots = state.timeslot_manager.timeslots();
    let mut timeslots = timeslots.lock().unwrap();

    // Find the first available slot that matches the requested datetime
    let slot = timeslots
        .values_mut()
        .find(|slot| slot.available && slot.datetime == booking.datetime)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "No available timeslot found for the requested datetime".to_string(),
            )
        })?;

    slot.available = false;

    Ok(Json(BookingResponse {
        message: format!("Successfully booked timeslot for {}", booking.client_name),
        booked_slot: slot.clone(),
    }))
}

async fn add_timeslot(
    State(state): State<AppState>,
    Json(timeslot): Json<AddTimeslotRequest>,
) -> Result<Json<AddTimeslotResponse>, (StatusCode, String)> {
    state.timeslot_manager.add_timeslot(timeslot.datetime);

    Ok(AddTimeslotResponse {
        message: "done".into(),
    }
    .into())
}

async fn remove_timeslot(
    State(state): State<AppState>,
    Json(timeslot): Json<DeleteTimeslotRequest>,
) -> Result<Json<DeleteTimeslotResponse>, (StatusCode, String)> {
    println!("remove timeslot called");

    let id = Uuid::new_v4();
    let response_message = match state.timeslot_manager.remove_timeslot(id) {
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

async fn get_admin_page() -> Result<Html<String>, (StatusCode, String)> {
    println!("get admin_page called");

    // Construct the path to the frontend file
    let path = Path::new("../frontend/admin_page.html");

    // Read the file asynchronously
    match fs::read_to_string(path).await {
        Ok(contents) => Ok(Html(contents)),
        Err(e) => {
            let error_message = format!("Failed to read frontend file: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}
