//! API Server module
//!
//! This module provides the HTTP API server functionality for the scatterbrain tool.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::models::PlanResponse;
use crate::models::{self, Index};
use crate::Core;

/// Request to add a new task
#[derive(Serialize, Deserialize)]
pub struct AddTaskRequest {
    pub description: String,
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
        .route("/api/plan", get(get_plan))
        .route("/api/current", get(get_current))
        .route("/api/distilled", get(get_distilled_context))
        .route("/api/task", post(add_task))
        .route("/api/task/complete", post(complete_task))
        .route("/api/task/level", post(change_level))
        .route("/api/move", post(move_to))
        .route("/ui", get(ui_handler))
        .route("/ui/events", get(events_handler))
        .layer(cors)
        .with_state(core);

    // Start server
    tracing::info!("Starting server on {}", config.address);
    let listener = TcpListener::bind(config.address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Handler implementations
async fn get_plan(State(core): State<Core>) -> JSONResp<models::Plan> {
    let plan_response = core.get_plan();
    Json(ApiResponse::success(plan_response))
}

async fn get_current(State(core): State<Core>) -> JSONResp<Option<models::Current>> {
    let response = core.current();
    Json(ApiResponse::success(response))
}

async fn get_distilled_context(State(core): State<Core>) -> JSONResp<()> {
    let response = core.distilled_context();
    Json(ApiResponse::success(response))
}

async fn add_task(
    State(core): State<Core>,
    Json(payload): Json<AddTaskRequest>,
) -> JSONResp<(models::Task, Index)> {
    let response = core.add_task(payload.description);
    Json(ApiResponse::success(response))
}

async fn complete_task(State(core): State<Core>) -> JSONResp<bool> {
    let response = core.complete_task();
    let ok = *response.inner();
    if ok {
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Failed to complete task".to_string()))
    }
}

async fn change_level(
    State(core): State<Core>,
    Json(payload): Json<ChangeLevelRequest>,
) -> JSONResp<Result<(), String>> {
    let response = core.change_level(payload.index, payload.level_index);
    Json(ApiResponse::success(response))
}

async fn move_to(
    State(core): State<Core>,
    Json(payload): Json<MoveToRequest>,
) -> JSONResp<Option<String>> {
    let response = core.move_to(payload.index);

    if response.inner().is_some() {
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error(
            "Failed to move to requested task".to_string(),
        ))
    }
}

// Event-stream handler for reactive updates
async fn events_handler(State(core): State<Core>) -> impl IntoResponse {
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

    let event_stream = EventStream::new(core);

    // Return response with headers and stream body
    (headers, axum::body::Body::from_stream(event_stream))
}

// Custom EventStream implementation
struct EventStream {
    core: Core,
    receiver: tokio::sync::broadcast::Receiver<()>,
}

impl EventStream {
    fn new(core: Core) -> Self {
        // Subscribe to updates
        let receiver = core.subscribe();

        Self { core, receiver }
    }
}

impl Stream for EventStream {
    type Item = Result<String, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to receive from the broadcast channel with a non-blocking approach
        match self.receiver.try_recv() {
            Ok(_) => {
                // Successfully received an update notification, send event to client
                Poll::Ready(Some(Ok("event: update\ndata: change\n\n".to_string())))
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

// UI handler
async fn ui_handler(State(core): State<Core>) -> impl IntoResponse {
    let plan_response = core.get_plan();
    let current_response = core.current();
    let distilled_context = core.distilled_context().distilled_context;
    let current_opt = current_response.inner();
    let plan = plan_response.inner();

    // Render the UI template, passing the distilled context
    let html = render_ui_template(plan, current_opt.as_ref(), &distilled_context);
    Html(html).into_response()
}

// Render the UI HTML template
fn render_ui_template(
    plan: &crate::models::Plan,
    current: Option<&crate::models::Current>,
    distilled_context: &crate::models::DistilledContext,
) -> String {
    let mut html = String::from(HTML_TEMPLATE_HEADER);

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

        // Task status
        html.push_str(&format!(
            "<span class='task-status'>{}</span></div>",
            if task.is_completed() { "✓" } else { "○" }
        ));

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
        .completed {
            color: #7f8c8d;
            text-decoration: line-through;
        }
        .completed .task-status {
            color: #27ae60;
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
        .subtasks li.completed {
            color: #7f8c8d;
            text-decoration: line-through;
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
            eventSource = new EventSource('/ui/events');
            
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
