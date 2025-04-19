//! API client module
//!
//! This module provides HTTP client functionality to interact with the scatterbrain API server.

use std::sync::Arc;

use reqwest::{Client as ReqwestClient, Error as ReqwestError};
use serde::Deserialize;

use crate::models;
use crate::models::Index;

// Import the request structs from the server module
use super::server::{
    AddTaskRequest, ChangeLevelRequest, CompleteTaskRequest, LeaseRequest, MoveToRequest,
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
    pub async fn get_plan(&self) -> Result<models::PlanResponse<models::Plan>, ClientError> {
        let url = format!("{}/api/plan", self.config.base_url);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Api(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<models::PlanResponse<models::Plan>> =
            match response.json().await {
                Ok(data) => data,
                Err(e) => {
                    // Get the text to debug error
                    let err_text = format!("JSON parse error: {}. Check model compatibility.", e);
                    return Err(ClientError::Api(err_text));
                }
            };

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
    pub async fn get_current(
        &self,
    ) -> Result<models::PlanResponse<Option<models::Current>>, ClientError> {
        let url = format!("{}/api/current", self.config.base_url);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Api(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<models::PlanResponse<Option<models::Current>>> =
            response.json().await?;

        if api_response.success {
            match api_response.data {
                Some(plan_response) => Ok(plan_response),
                None => Err(ClientError::MissingData),
            }
        } else {
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Get the distilled context
    pub async fn get_distilled_context(&self) -> Result<models::PlanResponse<()>, ClientError> {
        let url = format!("{}/api/distilled", self.config.base_url);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Api(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<models::PlanResponse<()>> = response.json().await?;

        if api_response.success {
            match api_response.data {
                Some(plan_response) => Ok(plan_response),
                None => Err(ClientError::MissingData),
            }
        } else {
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Add a new task
    pub async fn add_task(
        &self,
        description: String,
        level_index: usize,
    ) -> Result<models::PlanResponse<(models::Task, Index)>, ClientError> {
        let url = format!("{}/api/task", self.config.base_url);
        let request = AddTaskRequest {
            description,
            level_index,
        };
        let response = self.http_client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Api(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<models::PlanResponse<(models::Task, Index)>> =
            response.json().await?;

        if api_response.success {
            match api_response.data {
                Some(plan_response) => Ok(plan_response),
                None => Err(ClientError::MissingData),
            }
        } else {
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Complete the current task
    pub async fn complete_task(
        &self,
        lease: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> Result<models::PlanResponse<bool>, ClientError> {
        let url = format!("{}/api/task/complete", self.config.base_url);

        // Use the imported CompleteTaskRequest struct
        let request = CompleteTaskRequest {
            lease,
            force,
            summary,
        };

        let response = self.http_client.post(&url).json(&request).send().await?;
        let status = response.status(); // Read status before potentially consuming body

        if status.is_success() {
            let api_response: ApiResponse<models::PlanResponse<bool>> = match response.json().await
            {
                Ok(data) => data,
                Err(e) => {
                    return Err(ClientError::Api(format!(
                        "Failed to parse success response: {}",
                        e
                    )))
                }
            };
            if api_response.success {
                api_response.data.ok_or(ClientError::MissingData)
            } else {
                // The API returns success=false if completion fails (e.g., lease mismatch)
                // Extract the PlanResponse<bool> which should contain 'false'
                api_response.data.ok_or_else(|| {
                    ClientError::Api(
                        api_response
                            .error
                            .unwrap_or_else(|| "Unknown API error on failure".to_string()),
                    )
                })
            }
        } else {
            // Handle non-200 HTTP errors
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(ClientError::Api(format!(
                "HTTP error: {}. Body: {}",
                status, err_text
            )))
        }
    }

    /// Move to a specific task
    pub async fn move_to(
        &self,
        index: Index,
    ) -> Result<models::PlanResponse<Option<String>>, ClientError> {
        let url = format!("{}/api/move", self.config.base_url);
        let request = MoveToRequest { index };
        let response = self.http_client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let api_response: ApiResponse<models::PlanResponse<Option<String>>> =
                response.json().await?;
            if api_response.success {
                match api_response.data {
                    Some(plan_response) => Ok(plan_response),
                    None => Err(ClientError::MissingData),
                }
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        } else {
            let api_response: ApiResponse<()> = response.json().await?;
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Change the abstraction level of a task
    pub async fn change_level(
        &self,
        index: Index,
        level_index: usize,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let url = format!("{}/api/task/level", self.config.base_url);
        let request = ChangeLevelRequest { index, level_index };
        let response = self.http_client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let api_response: ApiResponse<models::PlanResponse<Result<(), String>>> =
                response.json().await?;
            if api_response.success {
                match api_response.data {
                    Some(plan_response) => Ok(plan_response),
                    None => Err(ClientError::MissingData),
                }
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        } else {
            let api_response: ApiResponse<()> = response.json().await?;
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Generate a lease for a specific task
    pub async fn generate_lease(
        &self,
        index: Index,
    ) -> Result<models::PlanResponse<models::Lease>, ClientError> {
        let url = format!("{}/api/task/lease", self.config.base_url);

        // Use the imported LeaseRequest struct
        let request = LeaseRequest { index };

        let response = self.http_client.post(&url).json(&request).send().await?;
        let status = response.status(); // Read status before potentially consuming body

        if status.is_success() {
            let api_response: ApiResponse<models::PlanResponse<models::Lease>> =
                match response.json().await {
                    Ok(data) => data,
                    Err(e) => {
                        return Err(ClientError::Api(format!(
                            "Failed to parse success response: {}",
                            e
                        )))
                    }
                };
            if api_response.success {
                api_response.data.ok_or(ClientError::MissingData)
            } else {
                Err(ClientError::Api(
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown API error".to_string()),
                ))
            }
        } else {
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(ClientError::Api(format!(
                "HTTP error: {}. Body: {}",
                status, err_text
            )))
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
