//! Model Context Protocol (MCP) server implementation for scatterbrain
//!
//! This module provides an MCP server that exposes scatterbrain functionality as MCP tools,
//! allowing AI assistants to interact with scatterbrain plans and tasks through the standardized MCP protocol.

use crate::api::client::{Client, ClientError};
use crate::models::{self, Index, PlanError};
use crate::Core;
use rmcp::{model::*, tool, Error as McpError};

/// MCP server implementation for scatterbrain
///
/// This server wraps a Core instance and exposes scatterbrain functionality as MCP tools.
/// It provides comprehensive access to plan management, task operations, navigation, and notes management.
#[derive(Clone)]
pub struct ScatterbrainMcpServer {
    core: Core,
}

impl ScatterbrainMcpServer {
    /// Create a new MCP server with the given Core instance
    pub fn new(core: Core) -> Self {
        Self { core }
    }

    /// Create a new MCP server with the given Core instance (alias for new)
    pub fn with_core(core: Core) -> Self {
        Self::new(core)
    }
}

/// Convert PlanError to ClientError for interface compatibility
impl From<PlanError> for ClientError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::PlanNotFound(plan_id) => ClientError::PlanNotFound(plan_id),
            PlanError::Internal(msg) => ClientError::Internal(msg),
            PlanError::LockError => ClientError::Internal("Lock error".to_string()),
        }
    }
}

/// Helper function to convert scatterbrain results to MCP CallToolResult
fn to_mcp_result<T: serde::Serialize>(
    result: Result<T, ClientError>,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => {
            let json = serde_json::to_value(value)
                .map_err(|e| McpError::internal_error(format!("Serialization error: {e}"), None))?;
            Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json).map_err(|e| {
                    McpError::internal_error(format!("JSON formatting error: {e}"), None)
                })?,
            )]))
        }
        Err(e) => Err(McpError::internal_error(
            format!("Scatterbrain error: {e}"),
            None,
        )),
    }
}

/// Helper function to parse index from string
fn parse_index(index_str: &str) -> Result<Index, McpError> {
    models::parse_index(index_str).map_err(|e| {
        McpError::invalid_params(format!("Invalid index format '{index_str}': {e}"), None)
    })
}

// Implement the Client trait for ScatterbrainMcpServer
#[async_trait::async_trait]
impl Client for ScatterbrainMcpServer {
    async fn get_plan(&self, id: u8) -> Result<models::PlanResponse<models::Plan>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core.get_plan(&plan_id).map_err(ClientError::from)
    }

    async fn get_current(
        &self,
        id: u8,
    ) -> Result<models::PlanResponse<Option<models::Current>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core.current(&plan_id).map_err(ClientError::from)
    }

    async fn get_distilled_context(&self, id: u8) -> Result<models::PlanResponse<()>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .distilled_context(&plan_id)
            .map_err(ClientError::from)
    }

    async fn add_task(
        &self,
        id: u8,
        description: String,
        level_index: usize,
        notes: Option<String>,
    ) -> Result<models::PlanResponse<(models::Task, Index)>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .add_task(&plan_id, description, level_index, notes)
            .map_err(ClientError::from)
    }

    async fn complete_task(
        &self,
        id: u8,
        index: Index,
        lease: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> Result<models::PlanResponse<bool>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .complete_task(&plan_id, index, lease, force, summary)
            .map_err(ClientError::from)
    }

    async fn move_to(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Option<String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .move_to(&plan_id, index)
            .map_err(ClientError::from)
    }

    async fn change_level(
        &self,
        id: u8,
        index: Index,
        level_index: usize,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .change_level(&plan_id, index, level_index)
            .map_err(ClientError::from)
    }

    async fn generate_lease(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<(models::Lease, Vec<String>)>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .generate_lease(&plan_id, index)
            .map_err(ClientError::from)
    }

    async fn remove_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<models::Task, String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .remove_task(&plan_id, index)
            .map_err(ClientError::from)
    }

    async fn get_task_notes(&self, id: u8, index: Index) -> Result<Option<String>, ClientError> {
        let plan_id = models::Lease::new(id);
        // Note: Core's get_task_notes returns PlanResponse<Result<Option<String>, String>>
        // We need to extract the inner value and handle the nested Result
        match self.core.get_task_notes(&plan_id, index) {
            Ok(plan_response) => match plan_response.into_inner() {
                Ok(notes) => Ok(notes),
                Err(err) => Err(ClientError::Internal(err)),
            },
            Err(plan_error) => Err(ClientError::from(plan_error)),
        }
    }

    async fn set_task_notes(
        &self,
        id: u8,
        index: Index,
        notes: String,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .set_task_notes(&plan_id, index, notes)
            .map_err(ClientError::from)
    }

    async fn delete_task_notes(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .delete_task_notes(&plan_id, index)
            .map_err(ClientError::from)
    }

    async fn uncomplete_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<bool, String>>, ClientError> {
        let plan_id = models::Lease::new(id);
        self.core
            .uncomplete_task(&plan_id, index)
            .map_err(ClientError::from)
    }

    async fn create_plan(
        &self,
        prompt: String,
        notes: Option<String>,
    ) -> Result<models::PlanId, ClientError> {
        self.core
            .create_plan(prompt, notes)
            .map_err(ClientError::from)
    }

    async fn delete_plan(&self, id: u8) -> Result<(), ClientError> {
        let plan_id = models::Lease::new(id);
        self.core.delete_plan(&plan_id).map_err(ClientError::from)
    }

    async fn list_plans(&self) -> Result<Vec<models::Lease>, ClientError> {
        self.core.list_plans().map_err(ClientError::from)
    }
}

#[tool(tool_box)]
impl ScatterbrainMcpServer {
    // Plan Management Tools

    #[tool(description = "Get a plan by ID")]
    async fn get_plan(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = Client::get_plan(self, plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Create a new plan with required prompt and optional notes")]
    async fn create_plan(
        &self,
        #[tool(param)] prompt: String,
        #[tool(param)] notes: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let result = Client::create_plan(self, prompt, notes).await;
        to_mcp_result(result)
    }

    #[tool(description = "Delete a plan by ID")]
    async fn delete_plan(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = Client::delete_plan(self, plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "List all available plans")]
    async fn list_plans(&self) -> Result<CallToolResult, McpError> {
        let result = Client::list_plans(self).await;
        to_mcp_result(result)
    }

    // Navigation Tools

    #[tool(description = "Get the current task for a plan")]
    async fn get_current(&self, #[tool(param)] plan_id: u8) -> Result<CallToolResult, McpError> {
        let result = Client::get_current(self, plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Get distilled context for a plan")]
    async fn get_distilled_context(
        &self,
        #[tool(param)] plan_id: u8,
    ) -> Result<CallToolResult, McpError> {
        let result = Client::get_distilled_context(self, plan_id).await;
        to_mcp_result(result)
    }

    #[tool(description = "Move to a specific task by index (e.g., '0,1,2')")]
    async fn move_to(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = Client::move_to(self, plan_id, parsed_index).await;
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
        let result = Client::add_task(self, plan_id, description, level_index, notes).await;
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
        let result = Client::complete_task(
            self,
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
        let result = Client::uncomplete_task(self, plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Remove a task by index")]
    async fn remove_task(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = Client::remove_task(self, plan_id, parsed_index).await;
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
        let result = Client::change_level(self, plan_id, parsed_index, level_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Generate a lease for a task")]
    async fn generate_lease(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = Client::generate_lease(self, plan_id, parsed_index).await;
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
        let result = Client::get_task_notes(self, plan_id, parsed_index).await;
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
        let result = Client::set_task_notes(self, plan_id, parsed_index, notes).await;
        to_mcp_result(result)
    }

    #[tool(description = "Delete notes for a task")]
    async fn delete_task_notes(
        &self,
        #[tool(param)] plan_id: u8,
        #[tool(param)] index: String,
    ) -> Result<CallToolResult, McpError> {
        let parsed_index = parse_index(&index)?;
        let result = Client::delete_task_notes(self, plan_id, parsed_index).await;
        to_mcp_result(result)
    }

    #[tool(description = "Get comprehensive guide on using Scatterbrain through MCP")]
    async fn get_guide(&self) -> Result<CallToolResult, McpError> {
        let guide_content = crate::guide::get_guide_string(crate::guide::GuideMode::Mcp);
        Ok(CallToolResult::success(vec![Content::text(guide_content)]))
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
                 Use plan_id to specify which plan to work with, and index format like '0,1,2' for task navigation.\n\
                 Start with the `get_guide()` tool to get started."
                    .into(),
            ),
        }
    }
}
