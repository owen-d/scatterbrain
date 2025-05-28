//! API client module
//!
//! This module provides HTTP client functionality to interact with the scatterbrain API server.

use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client as ReqwestClient, Error as ReqwestError, Method,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::models::{self, Index};

// Import the request structs from the server module
use super::server::{
    AddTaskRequest, ChangeLevelRequest, CompleteTaskRequest, CreatePlanRequest, LeaseRequest,
    MoveToRequest, SetTaskNotesRequest, UncompleteTaskRequest,
};

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
    #[error("Request error: {0}")]
    Request(#[from] ReqwestError),

    #[error("API error: {0}")]
    Api(String),

    #[error("Plan not found: ID {0:?}")]
    PlanNotFound(models::PlanId),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal client error: {0}")]
    Internal(String),
}

/// API client for the scatterbrain service
#[derive(Debug, Clone)]
pub struct Client {
    http_client: ReqwestClient,
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
            http_client: ReqwestClient::new(),
            config,
        }
    }

    /// Helper function to send requests
    async fn request<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ClientError> {
        let url = format!("{}{}", self.config.base_url, path);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let mut request_builder = self.http_client.request(method, &url).headers(headers);

        if let Some(body_data) = body {
            request_builder = request_builder.json(body_data);
        }

        let response = request_builder.send().await?;
        let status = response.status();

        // Check if the status code indicates success
        if status.is_success() {
            // Attempt to deserialize the successful response
            let api_response: ApiResponse<T> = response.json().await?;
            if api_response.success {
                api_response.data.ok_or_else(|| {
                    ClientError::Internal("API reported success but sent no data".to_string())
                })
            } else {
                // Handle cases where API reports failure despite 2xx status (shouldn't happen ideally)
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        } else {
            // Attempt to deserialize the error response body
            let error_response: Result<ApiResponse<()>, _> = response.json().await;
            let error_message = error_response
                .ok()
                .and_then(|resp| resp.error)
                .unwrap_or_else(|| format!("HTTP error: {status}"));

            // Check for specific PlanNotFound error pattern if possible
            if error_message.contains("Plan ID '") && error_message.contains("' not found") {
                // Extract token (best effort) - Needs update for u8 ID
                let id_str = error_message.split('\'').nth(1).unwrap_or(""); // Fallback
                let id_val = id_str.parse::<u8>().unwrap_or(255); // Try parse, fallback
                Err(ClientError::PlanNotFound(models::Lease::new(id_val)))
            } else {
                Err(ClientError::Api(error_message))
            }
        }
    }

    /// Get the full plan
    pub async fn get_plan(
        &self,
        id: u8,
    ) -> Result<models::PlanResponse<models::Plan>, ClientError> {
        let path = format!("/api/plans/{id}/plan");
        self.request(Method::GET, &path, None::<&()>).await
    }

    /// Get the current task
    pub async fn get_current(
        &self,
        id: u8,
    ) -> Result<models::PlanResponse<Option<models::Current>>, ClientError> {
        let path = format!("/api/plans/{id}/current");
        self.request(Method::GET, &path, None::<&()>).await
    }

    /// Get the distilled context
    pub async fn get_distilled_context(
        &self,
        id: u8,
    ) -> Result<models::PlanResponse<()>, ClientError> {
        let path = format!("/api/plans/{id}/distilled");
        self.request(Method::GET, &path, None::<&()>).await
    }

    /// Add a new task
    pub async fn add_task(
        &self,
        id: u8,
        description: String,
        level_index: usize,
        notes: Option<String>,
    ) -> Result<models::PlanResponse<(models::Task, Index)>, ClientError> {
        let path = format!("/api/plans/{id}/task");
        let body = AddTaskRequest {
            description,
            level_index,
            notes,
        };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Complete the current task
    pub async fn complete_task(
        &self,
        id: u8,
        index: Index,
        lease: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> Result<models::PlanResponse<bool>, ClientError> {
        let path = format!("/api/plans/{id}/task/complete");
        let body = CompleteTaskRequest {
            index,
            lease,
            force,
            summary,
        };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Move to a specific task
    pub async fn move_to(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Option<String>>, ClientError> {
        let path = format!("/api/plans/{id}/move");
        let body = MoveToRequest { index };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Change the abstraction level of a task
    pub async fn change_level(
        &self,
        id: u8,
        index: Index,
        level_index: usize,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let path = format!("/api/plans/{id}/task/level");
        let body = ChangeLevelRequest { index, level_index };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Generate a lease for a specific task
    pub async fn generate_lease(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<(models::Lease, Vec<String>)>, ClientError> {
        let path = format!("/api/plans/{id}/task/lease");
        let body = LeaseRequest { index };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Removes a task by its index
    pub async fn remove_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<models::Task, String>>, ClientError> {
        let index_str = index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("/api/plans/{id}/tasks/{index_str}");
        self.request(Method::DELETE, &path, None::<&()>).await
    }

    /// Gets the notes for a specific task
    pub async fn get_task_notes(
        &self,
        id: u8,
        index: Index,
    ) -> Result<Option<String>, ClientError> {
        let index_str = index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("/api/plans/{id}/notes/{index_str}");
        self.request(Method::GET, &path, None::<&()>).await
    }

    /// Sets the notes for a specific task
    pub async fn set_task_notes(
        &self,
        id: u8,
        index: Index,
        notes: String,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let index_str = index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("/api/plans/{id}/notes/{index_str}");
        let body = SetTaskNotesRequest { notes };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Deletes the notes for a specific task
    pub async fn delete_task_notes(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let index_str = index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("/api/plans/{id}/notes/{index_str}");
        self.request(Method::DELETE, &path, None::<&()>).await
    }

    /// Uncompletes a task by its index
    pub async fn uncomplete_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<bool, String>>, ClientError> {
        let path = format!("/api/plans/{id}/task/uncomplete");
        let body = UncompleteTaskRequest { index };
        self.request(Method::POST, &path, Some(&body)).await
    }

    /// Create a new plan with an optional prompt and notes.
    pub async fn create_plan(
        &self,
        prompt: Option<String>,
        notes: Option<String>,
    ) -> Result<models::PlanId, ClientError> {
        // Use CreatePlanRequest struct, now assuming it has prompt and notes
        let body = CreatePlanRequest { prompt, notes };
        // Use the existing helper method
        self.request(Method::POST, "/api/plans", Some(&body)).await
    }

    pub async fn delete_plan(&self, id: u8) -> Result<(), ClientError> {
        let path = format!("/api/plans/{id}");
        self.request(Method::DELETE, &path, None::<&()>).await
    }

    pub async fn list_plans(&self) -> Result<Vec<models::Lease>, ClientError> {
        self.request(Method::GET, "/api/plans", None::<&()>).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
