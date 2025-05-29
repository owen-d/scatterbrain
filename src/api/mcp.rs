//! Model Context Protocol (MCP) server implementation for scatterbrain
//!
//! This module provides an MCP server that exposes scatterbrain functionality as MCP tools,
//! allowing AI assistants to interact with scatterbrain plans and tasks through the standardized MCP protocol.

use crate::api::client::{Client, CoreClient};
use crate::models::{self, Index};
use crate::Core;
use rmcp::{model::*, tool, Error as McpError};

/// MCP server implementation for scatterbrain
///
/// This server wraps a CoreClient and exposes scatterbrain functionality as MCP tools.
/// It provides comprehensive access to plan management, task operations, navigation, and notes management.
#[derive(Clone)]
pub struct ScatterbrainMcpServer {
    client: CoreClient,
}

impl ScatterbrainMcpServer {
    /// Create a new MCP server with the given Core instance
    pub fn new(core: Core) -> Self {
        Self {
            client: CoreClient::new(core),
        }
    }

    /// Create a new MCP server with the given CoreClient
    pub fn with_client(client: CoreClient) -> Self {
        Self { client }
    }
}

/// Helper function to convert scatterbrain results to MCP CallToolResult
fn to_mcp_result<T: serde::Serialize>(
    result: Result<T, crate::api::client::ClientError>,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => {
            let json = serde_json::to_value(value).map_err(|e| {
                McpError::internal_error(format!("Serialization error: {}", e), None)
            })?;
            Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json).map_err(|e| {
                    McpError::internal_error(format!("JSON formatting error: {}", e), None)
                })?,
            )]))
        }
        Err(e) => Err(McpError::internal_error(
            format!("Scatterbrain error: {}", e),
            None,
        )),
    }
}

/// Helper function to parse index from string
fn parse_index(index_str: &str) -> Result<Index, McpError> {
    models::parse_index(index_str).map_err(|e| {
        McpError::invalid_params(format!("Invalid index format '{}': {}", index_str, e), None)
    })
}

#[tool(tool_box)]
impl ScatterbrainMcpServer {
    // Plan Management Tools

    #[tool(description = "Get a plan by ID")]
    async fn get_plan(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = self.client.get_plan(plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Create a new plan with required prompt and optional notes")]
    async fn create_plan(
        &self,
        #[tool(param)] prompt: Option<String>,
        #[tool(param)] notes: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let prompt = prompt
            .ok_or_else(|| McpError::invalid_params("prompt is required".to_string(), None))?;
        let result = self.client.create_plan(prompt, notes).await;
        to_mcp_result(result)
    }

    #[tool(description = "Delete a plan by ID")]
    async fn delete_plan(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = self.client.delete_plan(plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "List all available plans")]
    async fn list_plans(&self) -> Result<CallToolResult, McpError> {
        let result = self.client.list_plans().await;
        to_mcp_result(result)
    }

    // Navigation Tools

    #[tool(description = "Get the current task for a plan")]
    async fn get_current(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = self.client.get_current(plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Get distilled context for a plan")]
    async fn get_distilled_context(
        &self,
        #[tool(param)] plan_id: u8,
    ) -> Result<CallToolResult, McpError> {
        let result = self.client.get_distilled_context(plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Move to a specific task by index (e.g., '0,1,2')")]
    async fn move_to(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.move_to(plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    // Task Operations

    #[tool(description = "Add a new task to a plan")]
    async fn add_task(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] description: String,
        #[tool(param)] level_index: usize,
        #[tool(param)] notes: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .client
            .add_task(plan_id, description, level_index, notes)
            .await;
        to_mcp_result(result)
    }

    #[tool(description = "Complete a task by index")]
    async fn complete_task(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
        #[tool(param)] lease: Option<u8>,
        #[tool(param)] force: Option<bool>,
        #[tool(param)] summary: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self
            .client
            .complete_task(
                plan_id,
                parsed_index,
                lease,
                force.unwrap_or(false),
                summary,
            )
            .await;
        to_mcp_result(result)
    }

    #[tool(description = "Uncomplete a task by index")]
    async fn uncomplete_task(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.uncomplete_task(plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Remove a task by index")]
    async fn remove_task(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.remove_task(plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Change the level of a task")]
    async fn change_level(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
        #[tool(param)] level_index: usize,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self
            .client
            .change_level(plan_id, parsed_index, level_index)
            .await;
        to_mcp_result(result)
    }

    #[tool(description = "Generate a lease for a task")]
    async fn generate_lease(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.generate_lease(plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    // Notes Management

    #[tool(description = "Get notes for a task")]
    async fn get_task_notes(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.get_task_notes(plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Set notes for a task")]
    async fn set_task_notes(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
        #[tool(param)] notes: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self
            .client
            .set_task_notes(plan_id, parsed_index, notes)
            .await;
        to_mcp_result(result)
    }

    #[tool(description = "Delete notes for a task")]
    async fn delete_task_notes(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = self.client.delete_task_notes(plan_id, parsed_index).await;
        to_mcp_result(result)
    }
}

// Implement ServerHandler for the MCP server
#[tool(tool_box)]
impl rmcp::ServerHandler for ScatterbrainMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            server_info: Implementation {
                name: "scatterbrain-mcp-server".into(),
                version: "0.1.0".into(),
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Scatterbrain MCP Server - Hierarchical planning and task management through MCP.\n\
                 Provides tools for plan management, task operations, navigation, and notes management.\n\
                 Use plan_id to specify which plan to work with, and index format like '0,1,2' for task navigation."
                    .into(),
            ),
        }
    }
}
