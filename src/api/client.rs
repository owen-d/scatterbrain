//! API client module
//!
//! This module provides HTTP client functionality to interact with the scatterbrain API server.

use std::sync::Arc;

use reqwest::{Client as ReqwestClient, Error as ReqwestError};
use serde::{Deserialize, Serialize};

use crate::models;
use crate::models::Index;

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
    pub async fn get_plan(&self) -> Result<models::Plan, ClientError> {
        let url = format!("{}/api/plan", self.config.base_url);
        let response = self.http_client.get(&url).send().await?;
        let api_response: ApiResponse<models::Plan> = response.json().await?;

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
    pub async fn get_current(&self) -> Result<models::Current, ClientError> {
        let url = format!("{}/api/current", self.config.base_url);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Api(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<models::Current> = response.json().await?;

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
        let api_response: ApiResponse<models::PlanResponse<Option<(models::Task, Index)>>> =
            response.json().await?;

        if api_response.success {
            match api_response.data {
                Some(plan_response) => match plan_response.into_inner() {
                    Some((_, index)) => Ok(index),
                    None => Err(ClientError::MissingData),
                },
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
    pub async fn move_to(&self, index: Index) -> Result<String, ClientError> {
        #[derive(Serialize)]
        struct MoveToRequest {
            index: Index,
        }

        let url = format!("{}/api/move", self.config.base_url);
        let request = MoveToRequest { index };
        let response = self.http_client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let api_response: ApiResponse<String> = response.json().await?;
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
            let api_response: ApiResponse<String> = response.json().await?;
            Err(ClientError::Api(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown API error".to_string()),
            ))
        }
    }

    /// Change the abstraction level of a task
    pub async fn change_level(&self, index: Index, level_index: usize) -> Result<(), ClientError> {
        #[derive(Serialize)]
        struct ChangeLevelRequest {
            index: Index,
            level_index: usize,
        }

        let url = format!("{}/api/task/level", self.config.base_url);
        let request = ChangeLevelRequest { index, level_index };
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
