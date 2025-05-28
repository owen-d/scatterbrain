//! Core client implementation
//!
//! This module provides a client implementation that wraps Core directly,
//! providing the same interface as HttpClientImpl but without HTTP overhead.

use super::{Client, ClientError};
use crate::models::{self, Index, PlanError};
use crate::Core;

/// A client implementation that wraps Core directly
#[derive(Clone)]
pub struct CoreClient {
    core: Core,
}

impl CoreClient {
    /// Create a new CoreClient with the given Core instance
    pub fn new(core: Core) -> Self {
        Self { core }
    }

    /// Create a new CoreClient with the given Core instance (alias for new)
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

#[async_trait::async_trait]
impl Client for CoreClient {
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
        prompt: Option<String>,
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
