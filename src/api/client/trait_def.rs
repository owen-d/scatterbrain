//! Client trait definition
//!
//! This module defines the `Client` trait that abstracts over different client implementations.

use super::ClientError;
use crate::models::{self, Index};

/// Trait defining the API client interface for the scatterbrain service
#[async_trait::async_trait]
pub trait Client {
    /// Get the full plan
    async fn get_plan(&self, id: u8) -> Result<models::PlanResponse<models::Plan>, ClientError>;

    /// Get the current task
    async fn get_current(
        &self,
        id: u8,
    ) -> Result<models::PlanResponse<Option<models::Current>>, ClientError>;

    /// Get the distilled context
    async fn get_distilled_context(&self, id: u8) -> Result<models::PlanResponse<()>, ClientError>;

    /// Add a new task
    async fn add_task(
        &self,
        id: u8,
        description: String,
        level_index: usize,
        notes: Option<String>,
    ) -> Result<models::PlanResponse<(models::Task, Index)>, ClientError>;

    /// Complete the current task
    async fn complete_task(
        &self,
        id: u8,
        index: Index,
        lease: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> Result<models::PlanResponse<bool>, ClientError>;

    /// Move to a specific task
    async fn move_to(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Option<String>>, ClientError>;

    /// Change the abstraction level of a task
    async fn change_level(
        &self,
        id: u8,
        index: Index,
        level_index: usize,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError>;

    /// Generate a lease for a specific task
    async fn generate_lease(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<(models::Lease, Vec<String>)>, ClientError>;

    /// Removes a task by its index
    async fn remove_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<models::Task, String>>, ClientError>;

    /// Gets the notes for a specific task
    async fn get_task_notes(&self, id: u8, index: Index) -> Result<Option<String>, ClientError>;

    /// Sets the notes for a specific task
    async fn set_task_notes(
        &self,
        id: u8,
        index: Index,
        notes: String,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError>;

    /// Deletes the notes for a specific task
    async fn delete_task_notes(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<(), String>>, ClientError>;

    /// Uncompletes a task by its index
    async fn uncomplete_task(
        &self,
        id: u8,
        index: Index,
    ) -> Result<models::PlanResponse<Result<bool, String>>, ClientError>;

    /// Create a new plan with a required prompt and optional notes
    async fn create_plan(
        &self,
        prompt: String,
        notes: Option<String>,
    ) -> Result<models::PlanId, ClientError>;

    /// Delete a plan by its ID
    async fn delete_plan(&self, id: u8) -> Result<(), ClientError>;

    /// List all available plans
    async fn list_plans(&self) -> Result<Vec<models::Lease>, ClientError>;
}
