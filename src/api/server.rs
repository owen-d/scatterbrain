//! API Server module
//!
//! This module provides the HTTP API server functionality for the scatterbrain tool.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Json, Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::models::{self, parse_index, Index, PlanError, PlanResponse};
use crate::Core;

/// Request to add a new task
#[derive(Serialize, Deserialize)]
pub struct AddTaskRequest {
    pub description: String,
    pub level_index: usize,
}

/// Request to move to a specific task
#[derive(Serialize, Deserialize)]
pub struct MoveToRequest {
    pub index: Index,
}

/// Request to change a task's abstraction level
#[derive(Serialize, Deserialize)]
pub struct ChangeLevelRequest {
    pub index: Index,
    pub level_index: usize,
}

/// Request to complete a task, possibly with lease
#[derive(Serialize, Deserialize)]
pub struct CompleteTaskRequest {
    pub index: Index,
    pub lease: Option<u8>,
    pub force: bool,
    pub summary: Option<String>,
}

/// Request to generate a lease for a task
#[derive(Serialize, Deserialize)]
pub struct LeaseRequest {
    pub index: Index,
}

/// Request to uncomplete a task
#[derive(Serialize, Deserialize)]
pub struct UncompleteTaskRequest {
    pub index: Index,
}

/// Request to create a new plan with an optional prompt
#[derive(Serialize, Deserialize, Default)] // Add Default for optional body
pub struct CreatePlanRequest {
    pub prompt: Option<String>,
}

/// Server configuration
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub address: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: ([127, 0, 0, 1], 3000).into(),
        }
    }
}

/// API responses
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub type JSONResp<T> = Json<ApiResponse<PlanResponse<T>>>;

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Helper function to map Core results to Axum responses
fn map_core_result_to_response<T: Serialize>(
    result: Result<PlanResponse<T>, PlanError>,
) -> Response {
    match result {
        Ok(plan_response) => {
            (StatusCode::OK, Json(ApiResponse::success(plan_response))).into_response()
        }
        Err(PlanError::PlanNotFound(token)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<PlanResponse<T>>::error(format!(
                "Plan '{}' not found",
                token
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<PlanResponse<T>>::error(format!(
                "Internal server error: {}",
                e
            ))),
        )
            .into_response(),
    }
}

/// Helper function to map Core results (without PlanResponse) to Axum responses
fn map_core_result_simple<T: Serialize>(result: Result<T, PlanError>) -> Response {
    match result {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))).into_response(),
        Err(PlanError::PlanNotFound(token)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<T>::error(format!(
                "Plan '{}' not found",
                token
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<T>::error(format!(
                "Internal server error: {}",
                e
            ))),
        )
            .into_response(),
    }
}

/// Starts the API server
pub async fn serve(core: Core, config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application with routes
    let app = Router::new()
        // --- Redirect root to the new plan listing UI --- //
        .route("/", get(|| async { Redirect::temporary("/ui") })) // Redirect to /ui
        // --- Plan Management --- //
        .route(
            "/api/plans",
            get(list_plans_handler).post(create_plan_handler),
        )
        .route("/api/plans/:id", delete(delete_plan_handler))
        // --- Existing Endpoints (now id-scoped) --- //
        .route("/api/plans/:id/plan", get(get_plan))
        .route("/api/plans/:id/current", get(get_current))
        .route("/api/plans/:id/distilled", get(get_distilled_context))
        .route("/api/plans/:id/task", post(add_task))
        .route("/api/plans/:id/task/complete", post(complete_task))
        .route("/api/plans/:id/task/level", post(change_level))
        .route("/api/plans/:id/task/lease", post(generate_lease))
        .route("/api/plans/:id/task/uncomplete", post(uncomplete_task))
        .route("/api/plans/:id/move", post(move_to))
        .route("/api/plans/:id/tasks/*index", delete(remove_task_handler))
        // --- UI --- //
        .route("/ui", get(list_plans_ui_handler)) // New route for listing plans
        .route("/ui/:id", get(ui_handler)) // Specific plan UI using ID
        .route("/ui/events/:id", get(events_handler)) // ID-scoped events
        .layer(cors)
        .with_state(core);

    // Start server
    tracing::info!("Starting server on {}", config.address);
    let listener = TcpListener::bind(config.address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- Plan Management Handlers --- //

async fn list_plans_handler(State(core): State<Core>) -> impl IntoResponse {
    let result = core.list_plans();
    map_core_result_simple(result) // Returns Vec<Lease> (PlanId)
}

// --- New UI Handler for Listing Plans --- //

async fn list_plans_ui_handler(State(core): State<Core>) -> impl IntoResponse {
    match core.list_plans() {
        Ok(plan_ids) => {
            let mut html_content = String::new();
            html_content.push_str(
                "<!DOCTYPE html><html><head><title>Scatterbrain Plans</title></head><body>",
            );
            html_content.push_str("<h1>Available Scatterbrain Plans</h1>");

            if plan_ids.is_empty() {
                html_content.push_str("<p>No plans found. Create one using the CLI: <code>scatterbrain plan create</code></p>");
            } else {
                html_content.push_str("<ul>");
                for id in plan_ids {
                    let id_val = id.value();
                    html_content.push_str(&format!(
                        "<li><a href=\"/ui/{}\">Plan {}</a></li>",
                        id_val, id_val
                    ));
                }
                html_content.push_str("</ul>");
            }

            html_content.push_str("</body></html>");
            Html(html_content)
        }
        Err(e) => {
            // Log the error on the server
            tracing::error!("Failed to list plans for UI: {}", e);
            // Return a user-friendly HTML error page
            Html(format!(
                "<!DOCTYPE html><html><head><title>Error</title></head><body><h1>Error</h1><p>Could not load plan list: {}</p></body></html>",
                e
            ))
        }
    }
}

async fn create_plan_handler(
    State(core): State<Core>,
    // Use optional Json extractor for the request body
    payload: Option<Json<CreatePlanRequest>>,
) -> impl IntoResponse {
    // Extract the prompt, defaulting to None if payload is missing or malformed
    let prompt = payload.and_then(|json_payload| json_payload.0.prompt);

    // Call core.create_plan with the prompt
    let result = core.create_plan(prompt);
    map_core_result_simple(result) // Returns Lease (PlanId)
}

async fn delete_plan_handler(
    State(core): State<Core>,
    Path(id): Path<u8>, // Use u8 ID from path
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let result = core.delete_plan(&plan_id);
    map_core_result_simple(result) // Use simple mapper as it returns ()
}

// --- Existing Handler Implementations (Updated) --- //

async fn get_plan(State(core): State<Core>, Path(id): Path<u8>) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let result = core.get_plan(&plan_id);
    map_core_result_to_response(result)
}

async fn get_current(State(core): State<Core>, Path(id): Path<u8>) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.current(&plan_id);
    map_core_result_to_response(response)
}

async fn get_distilled_context(State(core): State<Core>, Path(id): Path<u8>) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.distilled_context(&plan_id);
    map_core_result_to_response(response)
}

async fn add_task(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<AddTaskRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.add_task(&plan_id, payload.description, payload.level_index);
    map_core_result_to_response(response)
}

async fn complete_task(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<CompleteTaskRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.complete_task(
        &plan_id,
        payload.index,
        payload.lease, // Already Option<u8>
        payload.force,
        payload.summary,
    );
    // Custom handling for the bool inside PlanResponse<bool>
    match response {
        Ok(plan_response) => {
            if *plan_response.inner() {
                (StatusCode::OK, Json(ApiResponse::success(plan_response))).into_response()
            } else {
                // Use the distilled context from the response even on failure
                (
                    StatusCode::BAD_REQUEST, // Or another suitable code
                    Json(ApiResponse::<PlanResponse<bool>>::error(format!(
                        "Failed to complete task (lease mismatch, already complete, or other issue)"
                    ))),
                )
                    .into_response()
            }
        }
        Err(e) => map_core_result_to_response::<bool>(Err(e)), // Use helper for PlanErrors
    }
}

async fn change_level(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<ChangeLevelRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.change_level(&plan_id, payload.index, payload.level_index);
    // Handle the Result<(), String> inside PlanResponse
    match response {
        Ok(plan_response) => match plan_response.inner() {
            Ok(_) => (StatusCode::OK, Json(ApiResponse::success(plan_response))).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<PlanResponse<Result<(), String>>>::error(
                    e.clone(),
                )),
            )
                .into_response(),
        },
        Err(e) => map_core_result_to_response::<Result<(), String>>(Err(e)), // Use helper for PlanErrors
    }
}

async fn generate_lease(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<LeaseRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.generate_lease(&plan_id, payload.index);
    map_core_result_to_response(response)
}

async fn uncomplete_task(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<UncompleteTaskRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.uncomplete_task(&plan_id, payload.index);
    // Handle the Result<bool, String> inside PlanResponse
    match response {
        Ok(plan_response) => match plan_response.inner() {
            Ok(true) => (StatusCode::OK, Json(ApiResponse::success(plan_response))).into_response(),
            Ok(false) => (
                StatusCode::BAD_REQUEST, // Should ideally not happen
                Json(ApiResponse::<PlanResponse<Result<bool, String>>>::error(
                    "Task was already incomplete or uncompletion failed silently".to_string(),
                )),
            )
                .into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<PlanResponse<Result<bool, String>>>::error(
                    e.clone(),
                )),
            )
                .into_response(),
        },
        Err(e) => map_core_result_to_response::<Result<bool, String>>(Err(e)), // Use helper for PlanErrors
    }
}

async fn move_to(
    State(core): State<Core>,
    Path(id): Path<u8>,
    Json(payload): Json<MoveToRequest>,
) -> impl IntoResponse {
    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.move_to(&plan_id, payload.index);
    // Handle Option<String> inside PlanResponse
    match response {
        Ok(plan_response) => {
            if plan_response.inner().is_some() {
                (StatusCode::OK, Json(ApiResponse::success(plan_response))).into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST, // Or NOT_FOUND?
                    Json(ApiResponse::<PlanResponse<Option<String>>>::error(
                        "Failed to move: Task index not found".to_string(),
                    )),
                )
                    .into_response()
            }
        }
        Err(e) => map_core_result_to_response::<Option<String>>(Err(e)), // Use helper for PlanErrors
    }
}

async fn remove_task_handler(
    State(core): State<Core>,
    Path((id, index_str)): Path<(u8, String)>, // Extract id (u8) and index string
) -> impl IntoResponse {
    // Parse the index string (from the wildcard path)
    let index = match parse_index(&index_str) {
        Ok(idx) => idx,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid index format: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let plan_id = models::Lease::new(id); // Use constructor
    let response = core.remove_task(&plan_id, index);
    // Simplify: Use the mapping helper directly instead of custom match logic
    map_core_result_to_response::<Result<models::Task, String>>(response)
}

// --- UI and Event Handlers (Updated for PlanId) --- //

async fn events_handler(
    State(core): State<Core>,
    Path(id): Path<u8>, // Accept u8 ID from path
) -> impl IntoResponse {
    let receiver = core.subscribe();
    // Pass the specific PlanId to the EventStream
    let plan_id = models::Lease::new(id); // Use constructor
    let stream = EventStream::new(core.clone(), receiver, plan_id);

    // Set headers for event stream
    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/event-stream"),
        ),
        (
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-cache"),
        ),
    ];

    // Return response with headers and stream body
    (headers, axum::body::Body::from_stream(stream))
}

struct EventStream {
    core: Core,
    receiver: tokio::sync::broadcast::Receiver<models::PlanId>,
    plan_id: models::PlanId,
}

impl EventStream {
    // Accept and store the plan_id
    fn new(
        core: Core,
        receiver: tokio::sync::broadcast::Receiver<models::PlanId>,
        plan_id: models::PlanId,
    ) -> Self {
        Self {
            core,
            receiver,
            plan_id,
        }
    }
}

impl Stream for EventStream {
    type Item = Result<String, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to receive from the broadcast channel with a non-blocking approach
        match self.receiver.try_recv() {
            Ok(id) => {
                if id == self.plan_id {
                    // Successfully received an update notification, send event to client
                    Poll::Ready(Some(Ok("event: update\ndata: change\n\n".to_string())))
                } else {
                    Poll::Pending
                }
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // No updates available now, register the waker to be notified later
                // Create a task to wake this future when the receiver might have data
                let waker = cx.waker().clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    waker.wake();
                });
                Poll::Pending
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                // Some messages were missed, but that's okay
                // Just notify the client that there was a change
                Poll::Ready(Some(Ok("event: update\ndata: change\n\n".to_string())))
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                // Channel closed, try to resubscribe
                self.receiver = self.core.subscribe();
                Poll::Pending
            }
        }
    }
}

// TODO: Update ui_handler to accept token and render for that plan
async fn ui_handler(State(core): State<Core>, Path(id): Path<u8>) -> impl IntoResponse {
    // Fetch all plan IDs for tabs
    let all_ids = match core.list_plans() {
        Ok(ids) => ids,
        Err(e) => {
            return Html(format!("<h1>Error loading plan list: {}</h1>", e)).into_response();
        }
    };
    let current_plan_id = models::Lease::new(id); // Use constructor

    // Fetch data for the requested plan id
    match core.get_plan(&current_plan_id) {
        Ok(plan_response) => {
            let plan = plan_response.inner();
            // Fetch current and distilled for the current plan id
            let current = core
                .current(&current_plan_id)
                .ok()
                .and_then(|pr| pr.into_inner());
            let distilled_context_res = core.distilled_context(&current_plan_id);

            match distilled_context_res {
                Ok(distilled_response) => {
                    let distilled_context = distilled_response.context(); // Extract the context
                    Html(render_ui_template(
                        &current_plan_id, // Pass current PlanId
                        &all_ids,         // Pass all PlanIds
                        plan,
                        current.as_ref(),
                        &distilled_context,
                    ))
                    .into_response()
                }
                Err(e) => {
                    // Handle error fetching distilled context for the specific plan
                    Html(format!(
                        "<h1>Error loading context for plan {:?}: {}</h1>",
                        current_plan_id, e
                    ))
                    .into_response()
                }
            }
        }
        Err(PlanError::PlanNotFound(_)) => {
            Html(format!("<h1>Plan {:?} not found</h1>", current_plan_id)).into_response()
        }
        Err(e) => {
            // Handle other errors fetching the plan itself
            Html(format!(
                "<h1>Error loading plan {:?}: {}</h1>",
                current_plan_id, e
            ))
            .into_response()
        }
    }
}

// --- Template Rendering (Needs Update for PlanId) --- //

fn render_ui_template(
    current_plan_id: &models::PlanId,
    all_ids: &[models::PlanId],
    plan: &crate::models::Plan,
    current: Option<&crate::models::Current>,
    distilled_context: &crate::models::DistilledContext,
) -> String {
    let mut html = String::from(HTML_TEMPLATE_HEADER);

    // --- Plan Tab Navigation ---
    html.push_str("<nav class='plan-tabs'>");
    if all_ids.is_empty() {
        html.push_str("<span class='no-plans'>No plans loaded.</span>");
    } else {
        for id in all_ids {
            let class = if id == current_plan_id { "active" } else { "" };
            // Use id.value() for the URL and display text
            html.push_str(&format!(
                "<a href='/ui/{}' class='{}'>Plan {}</a>&nbsp;",
                id.value(),
                class,
                id.value()
            ));
        }
    }
    html.push_str("</nav>");
    // --- End Plan Tab Navigation ---

    // Add level legend
    html.push_str("<div class='level-legend'>");
    html.push_str("<h3>Abstraction Levels</h3>");

    for (i, level) in plan.levels().iter().enumerate() {
        html.push_str(&format!(
            "<div class='level-item'><span class='task-level level-{}'>{}</span>",
            i, i
        ));
        html.push_str(&format!(
            "<div class='level-description'><strong>{}</strong>",
            level.description()
        ));
        html.push_str(&format!(
            "<div class='level-focus'>{}</div></div></div>",
            level.abstraction_focus()
        ));
    }
    html.push_str("</div>");

    // Add plan data
    html.push_str("<div class='plan-section'>");
    html.push_str("<h2>Plan</h2>");

    // Render tasks hierarchically
    render_tasks_html(
        &mut html,
        &plan.root().subtasks(),
        current,
        plan,
        Vec::new(),
    );

    html.push_str("</div>");

    // Add current task highlight if exists
    if let Some(curr) = current {
        html.push_str("<div class='current-section'>");
        html.push_str("<h2>Current Task</h2>");
        html.push_str(&format!(
            "<div class='current-task'><h3>{}</h3>",
            curr.task.description()
        ));
        html.push_str(&format!(
            "<p><strong>Status:</strong> {}</p>",
            if curr.task.is_completed() {
                "Completed"
            } else {
                "In Progress"
            }
        ));
        html.push_str(&format!(
            "<p><strong>Level:</strong> {} - {}</p>",
            curr.task.level_index().unwrap_or(curr.index.len() - 1),
            curr.level.description()
        ));
        html.push_str(&format!(
            "<p><strong>Index:</strong> {}</p>",
            curr.index
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));

        // Show subtasks if any
        if !curr.task.subtasks().is_empty() {
            html.push_str("<div class='subtasks'>");
            html.push_str("<h4>Subtasks:</h4>");
            html.push_str("<ul>");
            for subtask in curr.task.subtasks() {
                let status_class = if subtask.is_completed() {
                    "completed"
                } else {
                    "pending"
                };
                html.push_str(&format!(
                    "<li class='{}'>{}</li>",
                    status_class,
                    subtask.description()
                ));
            }
            html.push_str("</ul>");
            html.push_str("</div>");
        }

        html.push_str("</div></div>");
    }

    // Add History Panel (moved inside the container)
    html.push_str("<div class='history-panel'>");
    html.push_str("<h2>Transition History</h2>");
    html.push_str("<ul class='history-list'>");
    if distilled_context.transition_history.is_empty() {
        html.push_str("<li>No history yet.</li>");
    } else {
        // Iterate in reverse to show newest first
        for entry in distilled_context.transition_history.iter().rev() {
            let timestamp_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            let details_str = entry.details.as_deref().unwrap_or("");
            html.push_str(&format!(
                "<li class='history-item'><span class='history-ts'>{}</span><span class='history-action'>{}</span><span class='history-details'>{}</span></li>",
                timestamp_str,
                entry.action,
                details_str
            ));
        }
    }
    html.push_str("</ul></div>");

    // Embed the current plan id value for use in JavaScript
    html.push_str(&format!(
        "<script>const CURRENT_PLAN_ID = {};</script>",
        current_plan_id.value()
    ));

    html.push_str(HTML_TEMPLATE_FOOTER); // Footer now only contains closing tags and script
    html
}

// Helper function to render tasks hierarchically
fn render_tasks_html(
    html: &mut String,
    tasks: &[crate::models::Task],
    current: Option<&crate::models::Current>,
    plan: &crate::models::Plan,
    path: Vec<usize>,
) {
    if tasks.is_empty() {
        html.push_str("<p>No tasks yet.</p>");
        return;
    }

    html.push_str("<ul class='task-tree'>");
    for (i, task) in tasks.iter().enumerate() {
        let mut current_path = path.clone();
        current_path.push(i);

        // Check if this is the current task
        let is_current = if let Some(curr) = current {
            curr.index == current_path
        } else {
            false
        };

        // Determine the effective level (explicit or derived from position)
        let level_idx = task.level_index().unwrap_or(current_path.len());

        let class = if is_current {
            if task.is_completed() {
                "current completed"
            } else {
                "current"
            }
        } else if task.is_completed() {
            "completed"
        } else {
            ""
        };

        html.push_str(&format!("<li class='{}'><div class='task-item'>", class));

        // Level indicator
        html.push_str(&format!(
            "<span class='task-level level-{}'>{}</span>",
            level_idx, level_idx
        ));

        // Path identifier (e.g., 0.1.2)
        html.push_str(&format!(
            "<span class='task-path'>{}</span>",
            current_path
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(".")
        ));

        // Task description
        html.push_str(&format!(
            "<span class='task-desc'>{}</span>",
            task.description()
        ));

        // Add completion summary if available
        if task.is_completed() {
            if let Some(summary) = task.completion_summary() {
                html.push_str(&format!("<span class='task-summary'>{}</span>", summary));
            }
        }

        // Task status
        html.push_str(&format!(
            "<span class='task-status'>{}</span>",
            if task.is_completed() { "✓" } else { "○" }
        ));

        html.push_str("</div>"); // Close task-item div

        // Render subtasks recursively
        if !task.subtasks().is_empty() {
            render_tasks_html(html, task.subtasks(), current, plan, current_path);
        }

        html.push_str("</li>");
    }
    html.push_str("</ul>");
}

// HTML template header with CSS styles and EventSource JavaScript
const HTML_TEMPLATE_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Scatterbrain UI</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f7f9fc;
        }
        h1 {
            color: #2c3e50;
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }
        h2 {
            color: #3498db;
            margin-top: 30px;
        }
        .container {
            display: flex;
            flex-wrap: wrap;
            gap: 20px;
        }
        .plan-section,
        .current-section,
        .history-panel {
            flex: 1;
            min-width: 300px;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            align-self: flex-start;
        }
        .plan-section {
            /* Specific styles for plan if needed */
        }
        .current-section {
            /* Specific styles for current task if needed */
             /* Ensure it aligns even if not always present */
             order: 1;
        }
        .history-panel {
             order: 2;
        }
        .task-tree {
            list-style-type: none;
            padding-left: 20px;
        }
        .task-item {
            display: flex;
            align-items: center;
            padding: 8px 0;
            gap: 10px;
        }
        .task-path {
            font-family: monospace;
            color: #7f8c8d;
            min-width: 50px;
        }
        .task-desc {
            flex-grow: 1;
        }
        .task-status {
            color: #7f8c8d;
            font-weight: bold;
        }
        .task-level {
            display: inline-block;
            width: 24px;
            height: 24px;
            border-radius: 12px;
            color: white;
            text-align: center;
            line-height: 24px;
            font-size: 12px;
            font-weight: bold;
            margin-right: 8px;
        }
        .level-0 {
            background-color: #3498db; /* Blue - High Level */
            border: 2px solid #2980b9;
        }
        .level-1 {
            background-color: #9b59b6; /* Purple - Isolation */
            border: 2px solid #8e44ad;
        }
        .level-2 {
            background-color: #2ecc71; /* Green - Ordering */
            border: 2px solid #27ae60;
        }
        .level-3 {
            background-color: #e67e22; /* Orange - Implementation */
            border: 2px solid #d35400;
        }
        .current {
            background-color: #e8f4fc;
            border-left: 4px solid #3498db;
            padding-left: 10px;
            margin-left: -14px;
        }
        .completed .task-status {
            color: #27ae60;
            text-decoration: none !important; /* Ensure status icon is never struck through */
        }
        .task-summary {
            font-size: 0.9em;
            color: #555;
            margin-left: 15px; /* Indent summary slightly */
            font-style: italic;
        }
        .current-task {
            background-color: #f8f9fa;
            padding: 15px;
            border-radius: 5px;
            border-left: 4px solid #3498db;
        }
        .subtasks ul {
            margin-top: 5px;
            padding-left: 20px;
        }
        .subtasks li {
            margin-bottom: 5px;
        }
        .controls {
            margin-top: 30px;
            padding: 15px;
            background: #f0f4f8;
            border-radius: 5px;
        }
        .reactive-status {
            display: flex;
            align-items: center;
            gap: 10px;
            margin-top: 10px;
        }
        .status-indicator {
            display: inline-block;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            background-color: #95a5a6;
        }
        .status-indicator.connected {
            background-color: #2ecc71;
        }
        .status-indicator.updating {
            background-color: #f39c12;
        }
        .status-text {
            font-size: 14px;
            color: #7f8c8d;
        }
        .manual-refresh {
            margin-left: auto;
        }
        .level-legend {
            margin-top: 20px;
            background: white;
            padding: 15px;
            border-radius: 5px;
            box-shadow: 0 2px 5px rgba(0,0,0,0.1);
        }
        .level-legend h3 {
            margin-top: 0;
            border-bottom: 1px solid #eee;
            padding-bottom: 8px;
        }
        .level-item {
            display: flex;
            align-items: center;
            margin-bottom: 10px;
        }
        .level-description {
            margin-left: 10px;
        }
        .level-focus {
            font-size: 0.9em;
            color: #666;
            margin-top: 5px;
        }
        .history-list {
            list-style-type: none;
            padding-left: 0;
            max-height: 400px;
            overflow-y: auto;
        }
        .history-item {
            border-bottom: 1px solid #eee;
            padding: 8px 0;
            font-size: 0.9em;
            display: flex;
            gap: 10px;
        }
        .history-ts {
            color: #7f8c8d;
            min-width: 160px;
            white-space: nowrap;
        }
        .history-action {
            font-weight: bold;
            color: #3498db;
        }
        .history-details {
            color: #555;
            flex-grow: 1;
        }
        /* Style completed task description */
        .completed .task-desc {
            color: #7f8c8d;
            text-decoration: line-through;
        }
        /* Style completed subtask description */
        .subtasks li.completed .task-desc {
            color: #7f8c8d;
            text-decoration: line-through;
        }
    </style>
</head>
<body>
    <h1>Scatterbrain UI</h1>
    <div class="controls">
        <p>Use the CLI to interact with tasks:</p>
        <code>$ scatterbrain task add "New task"</code> | 
        <code>$ scatterbrain move 0,1</code> | 
        <code>$ scatterbrain task complete</code> |
        <code>$ scatterbrain task change-level 1</code>
        <div class="reactive-status">
            <span class="status-indicator" id="connection-status"></span>
            <span class="status-text" id="status-text">Waiting to connect...</span>
        </div>
    </div>
    <div class="container">
        <!-- HISTORY PANEL -->

        <div class="plan-section">
"#;

// HTML template footer with EventSource JavaScript for reactive refreshing
const HTML_TEMPLATE_FOOTER: &str = r#"
    </div>
    <script>
        // EventSource for reactive updates
        const statusIndicator = document.getElementById('connection-status');
        const statusText = document.getElementById('status-text');
        let eventSource;
        
        function connectEvents() {
            // Use the CURRENT_PLAN_ID injected by the template
            if (typeof CURRENT_PLAN_ID === 'undefined') {
                console.error('CURRENT_PLAN_ID is not defined.');
                statusText.textContent = 'Error: Plan ID missing.';
                return;
            }
            const eventSourceUrl = '/ui/events/' + CURRENT_PLAN_ID;
            console.log('Connecting to SSE:', eventSourceUrl); 
            eventSource = new EventSource(eventSourceUrl);
            
            eventSource.onopen = () => {
                statusIndicator.classList.add('connected');
                statusText.textContent = 'Connected: Listening for changes';
            };
            
            eventSource.addEventListener('update', (event) => {
                // Show updating status
                statusIndicator.classList.remove('connected');
                statusIndicator.classList.add('updating');
                statusText.textContent = 'Updating...';
                
                // Reload the page to reflect changes
                window.location.reload();
            });
            
            eventSource.addEventListener('ping', (event) => {
                // Just keep the connection alive
            });
            
            eventSource.onerror = () => {
                statusIndicator.classList.remove('connected');
                statusIndicator.classList.remove('updating');
                statusText.textContent = 'Connection lost. Reconnecting...';
                
                // Close connection and try again after a delay
                eventSource.close();
                setTimeout(connectEvents, 3000);
            };
        }
        
        // Start event connection when page loads
        window.addEventListener('load', connectEvents);
        
        // Clean up on unload
        window.addEventListener('beforeunload', () => {
            if (eventSource) {
                eventSource.close();
            }
        });
    </script>
</body>
</html>
"#;
