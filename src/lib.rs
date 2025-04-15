//! Scatterbrain library crate
//!
//! This library provides functionality for the scatterbrain tool.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// Represents an abstraction level for the LLM to work through
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Level {
    pub description: &'static str,
    pub questions: &'static [&'static str],
}

pub const PLAN: Level = Level {
    description: "high level planning; identifying architecture, scope, and approach",
    questions: &[
        "Is this approach simple?",
        "Is this approach extensible?",
        "Does this approach provide good, minimally leaking abstractions?",
    ],
};

pub const ISOLATION: Level = Level {
    description: "Identifying discrete parts of the plan which can be completed independently",
    questions: &[
        "If possible, can each part be completed and verified independently",
        "Are the boundaries between pieces modular and extensible?",
    ],
};

pub const ORDERING: Level = Level {
    description: "Ordering the parts of the plan",
    questions: &[
        "Do we move from foundational building blocks to more complex concepts?",
        "Do we follow idiomatic design patterns?",
    ],
};

pub const IMPLEMENTATION: Level = Level {
    description: "Turning each part into an ordered list of tasks",
    questions: &[
        "Can each task be completed independently?",
        "Is each task complimentary to, or does it build upon, the previous tasks?",
        "Does each task minimize the execution risk of the other tasks?",
    ],
};

pub const DEFAULT_LEVELS: &[Level] = &[PLAN, ISOLATION, ORDERING, IMPLEMENTATION];

/// Represents a task in the LLM's work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub completed: bool,
    pub subtasks: Vec<Task>,
}

impl Task {
    /// Creates a new task with the given level and description
    pub fn new(description: String) -> Self {
        Self {
            description,
            completed: false,
            subtasks: Vec::new(),
        }
    }

    /// Adds a subtask to this task
    pub fn add_subtask(&mut self, subtask: Task) {
        self.subtasks.push(subtask);
    }

    /// Marks this task as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// Returns true if this task and all its subtasks are completed
    pub fn is_fully_completed(&self) -> bool {
        self.completed && self.subtasks.iter().all(|t| t.is_fully_completed())
    }
}

#[derive(Clone, Serialize)]
pub struct Plan {
    pub root: Task,
    pub levels: Vec<Level>,
}

impl Plan {
    /// Creates a new plan with the given levels
    pub fn new(levels: Vec<Level>) -> Self {
        Self {
            root: Task::new("root".to_string()),
            levels,
        }
    }

    /// Returns the task at the given index, along with the hierarchy    of task descriptions that led to it
    pub fn get_with_history(&self, index: Index) -> Option<(Level, Task, Vec<String>)> {
        let mut current = &self.root;
        let mut history = Vec::new();

        for &i in &index {
            if i >= current.subtasks.len() {
                return None;
            }
            current = &current.subtasks[i];

            // Only add the description after descending (to avoid the implicit root)
            // and only if there are more levels to descend into (to avoid the final leaf which is included in full)
            if i < index.len() - 1 {
                history.push(current.description.clone());
            }
        }

        // Check if index is empty to avoid subtraction overflow
        if index.is_empty() {
            return None;
        }

        self.levels
            .get(index.len() - 1)
            .cloned()
            .map(|level| (level, current.clone(), history))
    }
}

impl From<Plan> for api_models::Plan {
    fn from(plan: Plan) -> Self {
        Self {
            root: plan.root,
            levels: plan.levels.into_iter().map(|l| l.into()).collect(),
        }
    }
}

// shorthand for the index of a task in the plan tree
type Index = Vec<usize>;

/// Context for managing the planning process
pub struct Context {
    plan: Plan,
    cursor: Index,
}

impl Context {
    /// Creates a new context with the given plan
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            cursor: Vec::new(), // Start at root
        }
    }

    // Task creation and navigation
    pub fn add_task(&mut self, description: String) -> Index {
        let task = Task::new(description);
        let new_index;

        if self.cursor.is_empty() {
            // Adding to root task, special case
            self.plan.root.add_subtask(task);
            new_index = vec![self.plan.root.subtasks.len() - 1];
        } else {
            // Navigate to the current task
            let current = self.get_current_task_mut().unwrap();

            // Add the new task
            current.add_subtask(task);
            let task_index = current.subtasks.len() - 1;

            // Create the new index
            let mut new_index = self.cursor.clone();
            new_index.push(task_index);
            return new_index;
        }

        new_index
    }

    pub fn move_to(&mut self, index: Index) -> bool {
        // Validate the index
        if index.is_empty() {
            self.cursor = index;
            return true;
        }

        // Check if the index is valid
        if let Some(_) = self.get_task(index.clone()) {
            self.cursor = index;
            true
        } else {
            false
        }
    }

    // Task state management
    pub fn complete_task(&mut self, index: Index) -> bool {
        if let Some(task) = self.get_task_mut(index) {
            task.complete();
            true
        } else {
            false
        }
    }

    // Information retrieval
    pub fn get_task(&self, index: Index) -> Option<&Task> {
        if index.is_empty() {
            return Some(&self.plan.root);
        }

        let mut current = &self.plan.root;
        for &idx in &index {
            if idx >= current.subtasks.len() {
                return None;
            }
            current = &current.subtasks[idx];
        }

        Some(current)
    }

    pub fn get_task_mut(&mut self, index: Index) -> Option<&mut Task> {
        if index.is_empty() {
            return Some(&mut self.plan.root);
        }

        let mut current = &mut self.plan.root;
        for &idx in &index {
            if idx >= current.subtasks.len() {
                return None;
            }
            current = &mut current.subtasks[idx];
        }

        Some(current)
    }

    pub fn get_current_task(&self) -> Option<&Task> {
        self.get_task(self.cursor.clone())
    }

    pub fn get_current_task_mut(&mut self) -> Option<&mut Task> {
        self.get_task_mut(self.cursor.clone())
    }

    pub fn get_current_index(&self) -> &Index {
        &self.cursor
    }

    pub fn get_current_level(&self) -> usize {
        self.cursor.len()
    }

    pub fn set_current_level(&mut self, level: usize) {
        while self.cursor.len() > level {
            self.cursor.pop();
        }
    }

    pub fn get_subtasks(&self, index: Index) -> Vec<(Index, &Task)> {
        if let Some(task) = self.get_task(index.clone()) {
            let mut result = Vec::new();
            for (i, subtask) in task.subtasks.iter().enumerate() {
                let mut new_index = index.clone();
                new_index.push(i);
                result.push((new_index, subtask));
            }
            result
        } else {
            Vec::new()
        }
    }

    // Plan access
    pub fn get_plan(&self) -> &Plan {
        &self.plan
    }

    pub fn get_plan_mut(&mut self) -> &mut Plan {
        &mut self.plan
    }
}

#[derive(Serialize)]
pub struct Current {
    pub index: Index,
    pub level: Level,
    pub task: Task,
    pub history: Vec<String>,
}

impl From<Current> for api_models::Current {
    fn from(current: Current) -> Self {
        Self {
            index: current.index,
            level: current.level.into(),
            task: current.task,
            history: current.history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation_and_navigation() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        assert_eq!(context.get_current_index(), &Vec::<usize>::new());
        // Add a task at the root level
        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(task1_index, vec![0]);

        // Add another task at the root level
        let task2_index = context.add_task("Task 2".to_string());
        assert_eq!(task2_index, vec![1]);

        // Move to the first task
        assert!(context.move_to(task1_index.clone()));

        // Add a subtask to the first task
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(subtask1_index, vec![0, 0]);

        // Move to the second task
        assert!(context.move_to(task2_index.clone()));
        assert_eq!(context.get_current_index(), &vec![1]);

        // Move to subtask 1
        assert!(context.move_to(subtask1_index.clone()));
        assert_eq!(context.get_current_index(), &vec![0, 0]);
    }

    #[test]
    fn test_task_completion() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string());
        let task2_index = context.add_task("Task 2".to_string());

        // Complete a task
        assert!(context.complete_task(task1_index.clone()));

        // Verify the task is completed
        let task = context.get_task(task1_index).unwrap();
        assert!(task.completed);

        // Verify the other task is not completed
        let task = context.get_task(task2_index).unwrap();
        assert!(!task.completed);
    }

    #[test]
    fn test_get_subtasks() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string());
        let task2_index = context.add_task("Task 2".to_string());

        // Move to the first task and add subtasks
        context.move_to(task1_index.clone());
        let subtask1_index = context.add_task("Subtask 1".to_string());
        let subtask2_index = context.add_task("Subtask 2".to_string());

        // Get subtasks of the first task
        let subtasks = context.get_subtasks(task1_index.clone());
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].0, subtask1_index);
        assert_eq!(subtasks[1].0, subtask2_index);

        // Get subtasks of the second task (should be empty)
        let subtasks = context.get_subtasks(task2_index.clone());
        assert_eq!(subtasks.len(), 0);
    }

    #[test]
    fn test_navigation() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        let root_index = context.add_task("Root task".to_string());
        assert!(context.move_to(root_index.clone()));

        assert_eq!(context.get_current_index(), &vec![0]);

        let task1_index = context.add_task("Task 1".to_string());
        assert!(context.move_to(task1_index.clone()));
        assert_eq!(context.get_current_index(), &vec![0, 0]);

        let task2_index = context.add_task("Task 2".to_string());
        assert_eq!(context.get_current_index(), &vec![0, 0]);
        assert!(context.move_to(task2_index.clone()));
        assert_eq!(context.get_current_index(), &vec![0, 0, 0]);
    }

    #[test]
    fn test_get_with_history() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        let root_index = context.add_task("Root task".to_string());
        assert!(context.move_to(root_index.clone()));

        // Add sibling tasks
        let task1_index = context.add_task("Task 1".to_string());
        let _ = context.add_task("Task 2".to_string());

        // Move to the first task and add a subtask
        context.move_to(task1_index.clone());
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(subtask1_index, vec![0, 0, 0]);

        // Test getting history for the subtask
        let history = context
            .plan
            .get_with_history(subtask1_index.clone())
            .unwrap();
        let (level, task, task_history) = history;

        // Verify the level is correct
        assert_eq!(level.description, ORDERING.description);

        // Verify the task is correct
        assert_eq!(task.description, "Subtask 1");

        // Verify the history is correct
        assert_eq!(task_history.len(), 3);
        assert_eq!(task_history[0], "Root task");
        assert_eq!(task_history[1], "Task 1");
    }

    #[test]
    fn test_level_inference() {
        // Create a plan with default levels
        let plan = Plan::new(DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string());

        // Root level (empty cursor) should be 0
        assert_eq!(context.get_current_level(), 0);

        // Add a task at root level
        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(context.get_current_level(), 0);

        // Move to task1 (level 1)
        context.move_to(task1_index.clone());
        assert_eq!(context.get_current_level(), 1);

        // Add a subtask to task1
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(context.get_current_level(), 1);

        // Move to subtask1 (level 2)
        context.move_to(subtask1_index.clone());
        assert_eq!(context.get_current_level(), 2);

        // Set level back to 1
        context.set_current_level(1);
        assert_eq!(context.get_current_level(), 1);
        assert_eq!(context.get_current_index(), &task1_index);

        // Set level back to 0 (root)
        context.set_current_level(0);
        assert_eq!(context.get_current_level(), 0);
        assert!(context.get_current_index().is_empty());
    }
}

#[derive(Clone)]
pub struct Core {
    inner: Arc<Mutex<Context>>,
    update_tx: Arc<tokio::sync::broadcast::Sender<()>>,
}

impl Core {
    pub fn new(context: Context) -> Self {
        // Create a broadcast channel with capacity for 100 messages
        let (tx, _rx) = tokio::sync::broadcast::channel(100);

        Self {
            inner: Arc::new(Mutex::new(context)),
            update_tx: Arc::new(tx),
        }
    }

    pub fn get_plan(&self) -> Option<Plan> {
        match self.inner.lock() {
            Ok(guard) => Some(guard.get_plan().clone()),
            Err(poisoned) => {
                // Recover from a poisoned mutex by getting the guard anyway
                let guard = poisoned.into_inner();
                Some(guard.get_plan().clone())
            }
        }
    }

    pub fn current(&self) -> Option<Current> {
        let context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let index = context.get_current_index();
        if let Some((level, task, history)) = context.plan.get_with_history(index.clone()) {
            Some(Current {
                index: index.clone(),
                level,
                task,
                history,
            })
        } else {
            None
        }
    }

    pub fn add_task(&self, description: String) -> Option<Index> {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let result = context.add_task(description);

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        Some(result)
    }

    pub fn complete_task(&self) -> bool {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let index = context.get_current_index().clone();
        let result = context.complete_task(index.clone());

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        result
    }

    pub fn move_to(&self, index: Index) -> bool {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let result = context.move_to(index.clone());

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        result
    }

    // Subscribe to state updates
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.update_tx.subscribe()
    }
}

// API Server module
pub mod api {
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use axum::{
        extract::State,
        http::StatusCode,
        response::{Html, IntoResponse},
        routing::{get, post},
        Json, Router,
    };
    use futures::Stream;
    use serde::{Deserialize, Serialize};
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};

    use super::{api_models, Core, Index};

    /// Request to add a new task
    #[derive(Deserialize)]
    pub struct AddTaskRequest {
        description: String,
    }

    /// Request to move to a specific task
    #[derive(Deserialize)]
    pub struct MoveToRequest {
        index: Index,
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
            .route("/api/task", post(add_task))
            .route("/api/task/complete", post(complete_task))
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
    async fn get_plan(State(core): State<Core>) -> impl IntoResponse {
        match core.get_plan() {
            Some(plan) => {
                let api_plan: api_models::Plan = plan.into();
                Json(ApiResponse::success(api_plan)).into_response()
            }
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<api_models::Plan>::error(
                    "Failed to access plan data".to_string(),
                )),
            )
                .into_response(),
        }
    }

    async fn get_current(State(core): State<Core>) -> impl IntoResponse {
        match core.current() {
            Some(current) => {
                let api_current: api_models::Current = current.into();
                Json(ApiResponse::success(api_current)).into_response()
            }
            None => (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error(
                    "Current task not found".to_string(),
                )),
            )
                .into_response(),
        }
    }

    async fn add_task(
        State(core): State<Core>,
        Json(payload): Json<AddTaskRequest>,
    ) -> impl IntoResponse {
        match core.add_task(payload.description) {
            Some(index) => Json(ApiResponse::success(index)).into_response(),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Index>::error(
                    "Failed to add task".to_string(),
                )),
            )
                .into_response(),
        }
    }

    async fn complete_task(State(core): State<Core>) -> impl IntoResponse {
        let success = core.complete_task();
        if success {
            StatusCode::OK.into_response()
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    "Failed to complete current task".to_string(),
                )),
            )
                .into_response()
        }
    }

    async fn move_to(
        State(core): State<Core>,
        Json(payload): Json<MoveToRequest>,
    ) -> impl IntoResponse {
        let success = core.move_to(payload.index);
        if success {
            StatusCode::OK.into_response()
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    "Failed to move to requested task".to_string(),
                )),
            )
                .into_response()
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
                    return Poll::Ready(Some(Ok(format!("event: update\ndata: change\n\n"))));
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    // No updates available now, register the waker to be notified later
                    // Create a task to wake this future when the receiver might have data
                    let waker = cx.waker().clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        waker.wake();
                    });
                    return Poll::Pending;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                    // Some messages were missed, but that's okay
                    // Just notify the client that there was a change
                    return Poll::Ready(Some(Ok(format!("event: update\ndata: change\n\n"))));
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    // Channel closed, try to resubscribe
                    self.receiver = self.core.subscribe();
                    return Poll::Pending;
                }
            }
        }
    }

    // UI handler
    async fn ui_handler(State(core): State<Core>) -> impl IntoResponse {
        let plan_opt = core.get_plan();
        let current_opt = core.current();

        match plan_opt {
            Some(plan) => {
                let html = render_ui_template(&plan, current_opt.as_ref());
                Html(html).into_response()
            }
            None => {
                // If we can't get a plan, return an error
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error accessing application state".to_string(),
                )
                    .into_response()
            }
        }
    }

    // Render the UI HTML template
    fn render_ui_template(plan: &super::Plan, current: Option<&super::Current>) -> String {
        let mut html = String::from(HTML_TEMPLATE_HEADER);

        // Add plan data
        html.push_str("<div class='plan-section'>");
        html.push_str("<h2>Plan</h2>");

        // Render tasks hierarchically
        render_tasks_html(&mut html, &plan.root.subtasks, current, Vec::new());

        html.push_str("</div>");

        // Add current task highlight if exists
        if let Some(curr) = current {
            html.push_str("<div class='current-section'>");
            html.push_str("<h2>Current Task</h2>");
            html.push_str(&format!(
                "<div class='current-task'><h3>{}</h3>",
                curr.task.description
            ));
            html.push_str(&format!(
                "<p><strong>Status:</strong> {}</p>",
                if curr.task.completed {
                    "Completed"
                } else {
                    "In Progress"
                }
            ));
            html.push_str(&format!(
                "<p><strong>Level:</strong> {}</p>",
                curr.level.description
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
            if !curr.task.subtasks.is_empty() {
                html.push_str("<div class='subtasks'>");
                html.push_str("<h4>Subtasks:</h4>");
                html.push_str("<ul>");
                for subtask in &curr.task.subtasks {
                    let status_class = if subtask.completed {
                        "completed"
                    } else {
                        "pending"
                    };
                    html.push_str(&format!(
                        "<li class='{}'>{}</li>",
                        status_class, subtask.description
                    ));
                }
                html.push_str("</ul>");
                html.push_str("</div>");
            }

            html.push_str("</div></div>");
        }

        html.push_str(HTML_TEMPLATE_FOOTER);
        html
    }

    // Helper function to render tasks hierarchically
    fn render_tasks_html(
        html: &mut String,
        tasks: &[super::Task],
        current: Option<&super::Current>,
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

            let class = if is_current {
                if task.completed {
                    "current completed"
                } else {
                    "current"
                }
            } else if task.completed {
                "completed"
            } else {
                ""
            };

            html.push_str(&format!("<li class='{}'><div class='task-item'>", class));

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
                task.description
            ));

            // Task status
            html.push_str(&format!(
                "<span class='task-status'>{}</span></div>",
                if task.completed { "✓" } else { "○" }
            ));

            // Render subtasks recursively
            if !task.subtasks.is_empty() {
                render_tasks_html(html, &task.subtasks, current, current_path);
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
            gap: 30px;
        }
        .plan-section {
            flex: 2;
            min-width: 300px;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .current-section {
            flex: 1;
            min-width: 300px;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
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
    </style>
</head>
<body>
    <h1>Scatterbrain UI</h1>
    <div class="controls">
        <p>Use the CLI to interact with tasks:</p>
        <code>$ scatterbrain task add "New task"</code> | 
        <code>$ scatterbrain move 0,1</code> | 
        <code>$ scatterbrain task complete</code>
        <div class="reactive-status">
            <span class="status-indicator" id="connection-status"></span>
            <span class="status-text" id="status-text">Waiting to connect...</span>
            <button class="manual-refresh" onclick="manualRefresh()">Refresh Now</button>
        </div>
    </div>
    <div class="container">
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
        
        function manualRefresh() {
            window.location.reload();
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
}

// Client-oriented API models
pub mod api_models {
    use super::{Index, Task};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Level {
        pub description: String,
        pub questions: Vec<String>,
    }

    impl From<super::Level> for Level {
        fn from(level: super::Level) -> Self {
            Self {
                description: level.description.to_string(),
                questions: level.questions.iter().map(|&q| q.to_string()).collect(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Plan {
        pub root: Task,
        pub levels: Vec<Level>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Current {
        pub index: Index,
        pub level: Level,
        pub task: Task,
        pub history: Vec<String>,
    }
}

// API client module
pub mod client {
    use std::sync::Arc;

    use reqwest::{Client as ReqwestClient, Error as ReqwestError};
    use serde::{Deserialize, Serialize};

    use super::{api_models, Index};

    /// API client configuration
    #[derive(Debug, Clone)]
    pub struct ClientConfig {
        pub base_url: String,
    }

    impl Default for ClientConfig {
        fn default() -> Self {
            Self {
                base_url: "http://localhost:3000".to_string(),
            }
        }
    }

    /// Generic API response structure
    #[derive(Debug, Deserialize)]
    struct ApiResponse<T> {
        success: bool,
        data: Option<T>,
        error: Option<String>,
    }

    /// Client errors
    #[derive(Debug, thiserror::Error)]
    pub enum ClientError {
        #[error("HTTP error: {0}")]
        Http(#[from] ReqwestError),

        #[error("API error: {0}")]
        Api(String),

        #[error("Missing data in response")]
        MissingData,
    }

    /// API client for the scatterbrain service
    #[derive(Debug, Clone)]
    pub struct Client {
        http_client: Arc<ReqwestClient>,
        config: ClientConfig,
    }

    impl Client {
        /// Create a new client with default configuration
        pub fn new() -> Self {
            Self::with_config(ClientConfig::default())
        }

        /// Create a new client with custom configuration
        pub fn with_config(config: ClientConfig) -> Self {
            Self {
                http_client: Arc::new(ReqwestClient::new()),
                config,
            }
        }

        /// Get the full plan
        pub async fn get_plan(&self) -> Result<api_models::Plan, ClientError> {
            let url = format!("{}/api/plan", self.config.base_url);
            let response = self.http_client.get(&url).send().await?;
            let api_response: ApiResponse<api_models::Plan> = response.json().await?;

            if api_response.success {
                api_response.data.ok_or(ClientError::MissingData)
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        }

        /// Get the current task
        pub async fn get_current(&self) -> Result<api_models::Current, ClientError> {
            let url = format!("{}/api/current", self.config.base_url);
            let response = self.http_client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(ClientError::Api(format!(
                    "HTTP error: {}",
                    response.status()
                )));
            }

            let api_response: ApiResponse<api_models::Current> = response.json().await?;

            if api_response.success {
                api_response.data.ok_or(ClientError::MissingData)
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        }

        /// Add a new task
        pub async fn add_task(&self, description: String) -> Result<Index, ClientError> {
            #[derive(Serialize)]
            struct AddTaskRequest {
                description: String,
            }

            let url = format!("{}/api/task", self.config.base_url);
            let request = AddTaskRequest { description };
            let response = self.http_client.post(&url).json(&request).send().await?;
            let api_response: ApiResponse<Index> = response.json().await?;

            if api_response.success {
                api_response.data.ok_or(ClientError::MissingData)
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        }

        /// Complete the current task
        pub async fn complete_task(&self) -> Result<(), ClientError> {
            let url = format!("{}/api/task/complete", self.config.base_url);
            let response = self.http_client.post(&url).send().await?;

            if response.status().is_success() {
                Ok(())
            } else {
                let api_response: ApiResponse<()> = response.json().await?;
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        }

        /// Move to a specific task
        pub async fn move_to(&self, index: Index) -> Result<(), ClientError> {
            #[derive(Serialize)]
            struct MoveToRequest {
                index: Index,
            }

            let url = format!("{}/api/move", self.config.base_url);
            let request = MoveToRequest { index };
            let response = self.http_client.post(&url).json(&request).send().await?;

            if response.status().is_success() {
                Ok(())
            } else {
                let api_response: ApiResponse<()> = response.json().await?;
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        }
    }

    impl Default for Client {
        fn default() -> Self {
            Self::new()
        }
    }
}

// CLI module
pub mod cli {
    use std::process;

    use clap::{CommandFactory, Parser, Subcommand};
    use clap_complete::{generate, Shell};
    use std::io;

    use crate::{
        api::{serve, ServerConfig},
        api_models,
        client::{Client, ClientConfig, ClientError},
        Context, Core, Plan, DEFAULT_LEVELS,
    };

    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    pub struct Cli {
        #[command(subcommand)]
        command: Commands,

        /// API server URL
        #[arg(short, long, default_value = "http://localhost:3000")]
        server: String,
    }

    #[derive(Subcommand)]
    enum Commands {
        /// Start the scatterbrain API server
        Serve {
            /// Port to listen on
            #[arg(short, long, default_value_t = 3000)]
            port: u16,
        },

        /// Task management commands
        Task {
            #[command(subcommand)]
            command: TaskCommands,
        },

        /// Move to a task at the given index
        Move {
            /// Task index (e.g., 0 or 0,1,2 for nested tasks)
            index: String,
        },

        /// Get the plan
        Plan,

        /// Get the current task
        Current,

        /// Interactive guide on how to use this tool
        Guide,

        /// Generate shell completions
        Completions {
            /// The shell to generate completions for
            #[arg(value_enum)]
            shell: Shell,
        },
    }

    #[derive(Subcommand)]
    enum TaskCommands {
        /// Add a new task
        Add {
            /// Task description
            description: String,
        },

        /// Complete the current task
        Complete,
    }

    /// Run the CLI application
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::parse();

        match &cli.command {
            Commands::Serve { port } => {
                println!("Starting scatterbrain API server on port {}...", port);

                // Create a default plan with the default levels
                let plan = Plan::new(DEFAULT_LEVELS.to_vec());
                let context = Context::new(plan);
                let core = Core::new(context);

                // Create a server configuration with the specified port
                let config = ServerConfig {
                    address: ([127, 0, 0, 1], *port).into(),
                };

                // Start the API server
                serve(core, config).await?;
                Ok(())
            }

            Commands::Task { command } => {
                let client = create_client(&cli.server);

                match command {
                    TaskCommands::Add { description } => {
                        let index = client.add_task(description.clone()).await?;
                        println!("Added task: \"{}\" at index: {:?}", description, index);
                        Ok(())
                    }

                    TaskCommands::Complete => {
                        client.complete_task().await?;
                        println!("Completed the current task");
                        Ok(())
                    }
                }
            }

            Commands::Move { index } => {
                let client = create_client(&cli.server);

                // Parse the index string (format: 0 or 0,1,2)
                let parsed_index = parse_index(index)?;

                client.move_to(parsed_index).await?;
                println!("Moved to task at index: {}", index);
                Ok(())
            }

            Commands::Plan => {
                let client = create_client(&cli.server);

                let plan = client.get_plan().await?;
                print_plan(&plan);
                Ok(())
            }

            Commands::Current => {
                let client = create_client(&cli.server);

                match client.get_current().await {
                    Ok(current) => {
                        println!("Current Task:");
                        println!("  Description: {}", current.task.description);
                        println!("  Completed: {}", current.task.completed);
                        println!("  Level: {}", current.level.description);
                        println!("  Index: {:?}", current.index);

                        if !current.task.subtasks.is_empty() {
                            println!("\nSubtasks:");
                            for (i, subtask) in current.task.subtasks.iter().enumerate() {
                                println!(
                                    "  {}. {} (completed: {})",
                                    i, subtask.description, subtask.completed
                                );
                            }
                        }

                        Ok(())
                    }
                    Err(ClientError::Api(msg)) if msg.contains("Current task not found") => {
                        println!("No current task selected. Use 'move' to select a task.");
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }

            Commands::Guide => {
                print_guide();
                Ok(())
            }

            Commands::Completions { shell } => {
                // Generate completions for the specified shell
                let mut cmd = Cli::command();
                let bin_name = cmd.get_name().to_string();
                generate(*shell, &mut cmd, bin_name, &mut io::stdout());
                Ok(())
            }
        }
    }

    fn create_client(server_url: &str) -> Client {
        let config = ClientConfig {
            base_url: server_url.to_string(),
        };

        Client::with_config(config)
    }

    fn parse_index(index_str: &str) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        let parts: Result<Vec<usize>, _> = index_str
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect();

        match parts {
            Ok(index) => Ok(index),
            Err(_) => {
                eprintln!("Error: Invalid index format. Use format like '0' or '0,1,2'");
                process::exit(1);
            }
        }
    }

    fn print_plan(plan: &api_models::Plan) {
        println!("Scatterbrain Plan:");
        println!("Levels: {}", plan.levels.len());

        println!("\nRoot Tasks:");
        if plan.root.subtasks.is_empty() {
            println!("  No tasks yet. Add some with 'scatterbrain task add'");
        } else {
            for (i, task) in plan.root.subtasks.iter().enumerate() {
                println!(
                    "  {}. {} (completed: {})",
                    i, task.description, task.completed
                );

                // Print first level of subtasks if any
                if !task.subtasks.is_empty() {
                    for (j, subtask) in task.subtasks.iter().enumerate() {
                        println!(
                            "    {}.{}. {} (completed: {})",
                            i, j, subtask.description, subtask.completed
                        );
                    }
                }
            }
        }

        println!("\nAvailable Levels:");
        for (i, level) in plan.levels.iter().enumerate() {
            println!("  {}. {}", i + 1, level.description);
        }
    }

    fn print_guide() {
        let guide = r#"
=== SCATTERBRAIN GUIDE ===

Scatterbrain is a hierarchical planning and task management tool designed to help agents
systematically work through complex projects by breaking them down into manageable tasks.

== CONCEPTUAL MODEL ==

Scatterbrain uses a multi-level approach to planning:

1. High-level planning: Identifying architecture, scope, and approach
   - Focus on simplicity, extensibility, and good abstractions
   - Set the overall direction and boundaries of your project

2. Isolation: Breaking down the plan into discrete, independent parts
   - Ensure each part can be completed and verified independently
   - Create modular boundaries between pieces

3. Ordering: Sequencing the parts in a logical flow
   - Start with foundational building blocks
   - Progress toward more complex concepts
   - Follow idiomatic patterns for the domain

4. Implementation: Converting each part into specific, actionable tasks
   - Make tasks independently completable
   - Ensure tasks build upon each other
   - Minimize execution risk between tasks

== WORKFLOW FOR AGENTS ==

1. START THE SERVER
   $ scatterbrain serve

2. CREATE A PLAN AND NAVIGATE THE LEVELS
   - Begin at Level 1 with high-level planning:
     $ scatterbrain task add "Design system architecture"
     $ scatterbrain move 0
     
   - Add subtasks at Level 2 to break down the approach:
     $ scatterbrain task add "Identify core components"
     $ scatterbrain move 0,0
     
   - Continue adding more granular tasks at deeper levels

3. STAY ON TRACK
   - Regularly review your plan:
     $ scatterbrain plan
     
   - Focus on your current task:
     $ scatterbrain current
     
   - Complete tasks when finished:
     $ scatterbrain task complete
     
   - Move between tasks to adapt to changing priorities:
     $ scatterbrain move 1,2

4. PROGRESSIVE REFINEMENT
   - Start with broad strokes at Level 1
   - Refine details as you move to deeper levels
   - Complete higher-level tasks only when all subtasks are done
   - Use completed tasks to validate your approach

== AGENT PRODUCTIVITY TECHNIQUES ==

1. FOCUS MANAGEMENT
   - Work on one task at a time
   - Use 'current' to maintain context between sessions
   - Complete the current task before moving to another

2. STRUCTURED THINKING
   - Use Level 1 for "why" questions
   - Use Level 2 for "what" questions
   - Use Level 3 for "when" questions
   - Use Level 4 for "how" questions

3. ADAPTIVE PLANNING
   - Revisit and adjust higher levels when assumptions change
   - Add new tasks as you discover them
   - Move between different branches as needed

4. PROGRESS TRACKING
   - Mark tasks as complete to see visible progress
   - Use the plan view to identify stuck areas
   - Balance work across different branches of the plan

== COMMAND REFERENCE ==

SERVER OPERATIONS:
  $ scatterbrain serve [--port <PORT>]     Start the server

TASK MANAGEMENT:
  $ scatterbrain task add "Task description"    Create new task
  $ scatterbrain task complete                  Complete current task
  
NAVIGATION:
  $ scatterbrain move <INDEX>                   Navigate to a task
                                               (e.g., 0 or 0,1,2)
VIEWING:
  $ scatterbrain plan                           View the full plan
  $ scatterbrain current                        View current task
  
HELP:
  $ scatterbrain guide                          Show this guide
  $ scatterbrain <COMMAND> --help               Show command help

== TIPS FOR AGENTS ==

- When stuck, move up a level and reconsider your approach
- Keep tasks small and focused for easier tracking
- Use consistent naming patterns for related tasks
- Review completed tasks to learn what works
- Balance breadth vs. depth in your planning
"#;

        println!("{}", guide);
    }
}
