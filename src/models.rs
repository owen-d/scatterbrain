//! Core models for the scatterbrain library
//!
//! This module contains the core data types and business logic for the scatterbrain tool.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, RwLock};
use thiserror::Error; // Import fmt

// Re-export levels from the levels module
pub use crate::levels::{default_levels, Level};

lazy_static! {
    static ref ROOT_VERIFICATION_SUGGESTIONS: Vec<String> = vec![
        "Ensure compilation passes successfully.".to_string(),
        "Ensure new logic is tested in the most concise and isolated way possible.".to_string(),
        "Ensure the code written is DRY, idiomatic, and conforms to existing conventions."
            .to_string(),
        "Review code for clarity, maintainability, and potential edge cases.".to_string(),
    ];
    // Define a default Lease value for the initial plan
    pub static ref DEFAULT_PLAN_ID: PlanId = Lease(0);
}

/// Represents a task in the LLM's work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    description: String,
    completed: bool,
    subtasks: Vec<Task>,
    level_index: Option<usize>,
    completion_summary: Option<String>,
}

impl Task {
    /// Creates a new task with the given level and description
    pub fn new(description: String) -> Self {
        Self {
            description,
            completed: false,
            subtasks: Vec::new(),
            level_index: None,
            completion_summary: None,
        }
    }

    /// Creates a new task with a specific level index
    pub fn with_level(description: String, level_index: usize) -> Self {
        Self {
            description,
            completed: false,
            subtasks: Vec::new(),
            level_index: Some(level_index),
            completion_summary: None,
        }
    }

    /// Adds a subtask to this task
    pub(crate) fn add_subtask(&mut self, subtask: Task) {
        self.subtasks.push(subtask);
    }

    /// Marks this task as completed
    pub(crate) fn complete(&mut self) {
        self.completed = true;

        // Recursively complete all subtasks
        for subtask in &mut self.subtasks {
            subtask.complete();
        }
    }

    /// Uncompletes the task and clears its completion summary.
    pub(crate) fn uncomplete(&mut self) {
        self.completed = false;
        self.completion_summary = None;
    }

    /// Sets the level index for this task
    pub(crate) fn set_level(&mut self, level_index: usize) {
        self.level_index = Some(level_index);
    }

    /// Gets the description of this task
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Checks if this task is completed
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Gets the subtasks of this task
    pub fn subtasks(&self) -> &[Task] {
        &self.subtasks
    }

    /// Gets the level index if it's explicitly set
    pub fn level_index(&self) -> Option<usize> {
        self.level_index
    }

    /// Gets the completion summary if it exists
    pub fn completion_summary(&self) -> Option<&String> {
        self.completion_summary.as_ref()
    }
}

/// Represents a single state transition event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionLogEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub details: Option<String>,
}

impl TransitionLogEntry {
    pub fn new(action: String, details: Option<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            action,
            details,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Plan {
    root: Task,
    levels: Vec<Level>,
    /// The original prompt or high-level goal for this plan.
    goal: Option<String>,
}

impl Plan {
    /// Creates a new plan with the given levels and an optional goal
    pub fn new(levels: Vec<Level>, goal: Option<String>) -> Self {
        Self {
            root: Task::new("root".to_string()),
            levels,
            goal,
        }
    }

    /// Returns the task at the given index, along with the hierarchy of task descriptions that led to it
    pub(crate) fn get_with_history(&self, index: Index) -> Option<(Level, Task, Vec<String>)> {
        let mut current = &self.root;
        let mut history = Vec::new();

        for &i in &index {
            if i >= current.subtasks().len() {
                return None;
            }
            current = &current.subtasks()[i];

            // Only add the description after descending (to avoid the implicit root)
            // and only if there are more levels to descend into (to avoid the final leaf which is included in full)
            if i < index.len() - 1 {
                history.push(current.description().to_string());
            }
        }

        // Check if index is empty to avoid subtraction overflow
        if index.is_empty() {
            return None;
        }

        // Use the task's explicit level_index if set, otherwise fallback to position-based level
        let level_idx = current.level_index().unwrap_or(index.len() - 1);
        self.levels
            .get(level_idx)
            .cloned()
            .map(|level| (level, current.clone(), history))
    }

    /// Returns the root task
    pub(crate) fn root(&self) -> &Task {
        &self.root
    }

    /// Returns the root task mutably
    pub(crate) fn root_mut(&mut self) -> &mut Task {
        &mut self.root
    }

    /// Returns the levels in this plan
    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    /// Returns the number of levels in the plan
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }
}

// shorthand for the index of a task in the plan tree
pub type Index = Vec<usize>;

/// Parses a string representation of an index (e.g., "0,1,2") into an Index
pub fn parse_index(index_str: &str) -> Result<Index, Box<dyn std::error::Error>> {
    let parts: Result<Vec<usize>, _> = index_str
        .split(',')
        .map(|s| s.trim().parse::<usize>())
        .collect();

    match parts {
        Ok(index) => Ok(index),
        Err(e) => Err(e.into()),
    }
}

/// Represents a lease token for task completion
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Lease(u8);

impl Lease {
    /// Returns the inner u8 value of the lease.
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Creates a new Lease.
    pub fn new(value: u8) -> Self {
        Self(value)
    }
}

// Implement Display for Lease
impl fmt::Display for Lease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context for managing the planning process for a *single* plan
pub struct Context {
    plan: Plan,
    cursor: Index,
    history: VecDeque<TransitionLogEntry>,
    leases: HashMap<Index, Lease>,
    rng: StdRng,
}

// Define the maximum size for the history buffer
const MAX_HISTORY_SIZE: usize = 20;

impl Context {
    /// Creates a new context with the given plan
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            cursor: Vec::new(),                                 // Start at root
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE), // Initialize history
            leases: HashMap::new(),                             // Initialize leases
            rng: StdRng::seed_from_u64(0),
        }
    }

    /// Creates a new context with the given plan and a specific seed
    pub fn new_with_seed(plan: Plan, seed: u64) -> Self {
        Self {
            plan,
            cursor: Vec::new(),
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            leases: HashMap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Creates a default context with default levels and a seed RNG
    pub fn default_with_seed(seed: u64) -> Self {
        let plan = Plan::new(default_levels(), None); // Pass None for goal here
        Self::new_with_seed(plan, seed)
    }

    /// Logs a state transition, maintaining the history buffer size.
    fn log_transition(&mut self, action: String, details: Option<String>) {
        if self.history.len() == MAX_HISTORY_SIZE {
            self.history.pop_front(); // Remove the oldest entry
        }
        self.history
            .push_back(TransitionLogEntry::new(action, details));
    }

    /// Generates a new lease for the task at the given index,
    /// returning the lease and a list of verification suggestions if it's the root task.
    pub fn generate_lease(&mut self, index: Index) -> PlanResponse<(Lease, Vec<String>)> {
        // Generate a u8 lease value using rng
        let lease_val = self.rng.random::<u8>();
        let lease = Lease(lease_val);
        self.leases.insert(index.clone(), lease);

        // Check if this is the root task
        let verification_suggestions = if index.is_empty() {
            // Return a clone of the static suggestions
            ROOT_VERIFICATION_SUGGESTIONS.clone()
        } else {
            Vec::new() // No suggestions for non-root tasks
        };

        self.log_transition(
            "generate_lease".to_string(),
            Some(format!(
                "Generated lease {} for task {:?}. Suggestions provided: {}",
                lease_val,
                index,
                !verification_suggestions.is_empty()
            )),
        );

        PlanResponse::new(
            (lease, verification_suggestions),
            self.distilled_context().context(),
        )
    }

    // Task creation and navigation
    /// Adds a new task with the given description and level
    pub fn add_task(
        &mut self,
        description: String,
        level_index: usize,
    ) -> PlanResponse<(Task, Index)> {
        self.log_transition(
            "add_task".to_string(),
            Some(format!(
                "Adding task: '{}' with level {} to parent index {:?}",
                description, level_index, self.cursor
            )),
        );

        // Use Task::with_level instead of Task::new
        let task = Task::with_level(description, level_index);
        let new_index;
        let task_clone = task.clone();

        if self.cursor.is_empty() {
            // Adding to root task, special case
            self.plan.root_mut().add_subtask(task);
            new_index = vec![self.plan.root().subtasks().len() - 1];
            // No parents to uncomplete when adding to root
        } else {
            // Navigate to the current task (the parent of the new task)
            let parent_index = self.cursor.clone();
            let current = self.get_task_mut(parent_index.clone()).unwrap(); // Assume parent exists if cursor is not empty

            // Add the new task
            current.add_subtask(task);
            let task_index = current.subtasks().len() - 1;

            // Create the new index for the added task
            let mut task_index_vec = parent_index.clone();
            task_index_vec.push(task_index);
            new_index = task_index_vec;

            // Uncomplete parent tasks
            let mut ancestor_index = parent_index;
            while !ancestor_index.is_empty() {
                if let Some(ancestor_task) = self.get_task_mut(ancestor_index.clone()) {
                    ancestor_task.uncomplete();
                    self.log_transition(
                        "uncomplete_parent".to_string(),
                        Some(format!(
                            "Uncompleted parent task at index: {:?}",
                            ancestor_index
                        )),
                    );
                } else {
                    // Should not happen if indices are correct, but log defensively
                    self.log_transition(
                        "uncomplete_parent_failed".to_string(),
                        Some(format!(
                            "Failed to find ancestor task at index: {:?}",
                            ancestor_index
                        )),
                    );
                    break; // Stop if an ancestor is missing
                }
                // Move up to the next ancestor
                ancestor_index.pop();
            }
        }

        PlanResponse::new((task_clone, new_index), self.distilled_context().context())
    }

    /// Removes the task at the given index
    /// Returns the removed task on success, or an error message on failure
    pub fn remove_task(&mut self, index: Index) -> PlanResponse<Result<Task, String>> {
        self.log_transition(
            "remove_task".to_string(),
            Some(format!("Attempting to remove task at index: {:?}", index)),
        );

        // Basic validation: Cannot remove root (empty index)
        if index.is_empty() {
            let err_msg = "Cannot remove the root task.".to_string();
            self.log_transition("remove_task_failed".to_string(), Some(err_msg.clone()));
            return PlanResponse::new(Err(err_msg), self.distilled_context().context());
        }

        // Separate the last index (child index) from the parent path
        let child_idx = index.last().unwrap(); // We know index is not empty
        let parent_index = index[0..index.len() - 1].to_vec();

        // Get the parent task mutably
        let parent_task = match self.get_task_mut(parent_index.clone()) {
            Some(task) => task,
            None => {
                let err_msg = format!("Parent task at index {:?} not found.", parent_index);
                self.log_transition("remove_task_failed".to_string(), Some(err_msg.clone()));
                return PlanResponse::new(Err(err_msg), self.distilled_context().context());
            }
        };

        // Validate the child index and remove the task
        if *child_idx >= parent_task.subtasks.len() {
            let err_msg = format!(
                "Child index {} out of bounds for parent {:?}",
                child_idx, parent_index
            );
            self.log_transition("remove_task_failed".to_string(), Some(err_msg.clone()));
            return PlanResponse::new(Err(err_msg), self.distilled_context().context());
        }

        // Remove the task
        let removed_task = parent_task.subtasks.remove(*child_idx);

        // Remove associated lease if it exists
        self.leases.remove(&index);

        // Adjust cursor if necessary
        // If the cursor was pointing to the removed task or one of its descendants,
        // move the cursor to the parent task.
        if self.cursor.starts_with(&index) {
            self.cursor = parent_index;
            self.log_transition(
                "cursor_adjusted_after_removal".to_string(),
                Some(format!("Cursor moved to parent {:?}", self.cursor)),
            );
        }

        self.log_transition(
            "remove_task_success".to_string(),
            Some(format!("Removed task: '{}'", removed_task.description())),
        );

        PlanResponse::new(Ok(removed_task), self.distilled_context().context())
    }

    /// Moves to the task at the given index
    pub fn move_to(&mut self, index: Index) -> PlanResponse<Option<String>> {
        self.log_transition(
            "move_to".to_string(),
            Some(format!("Moving cursor to index: {:?}", index)),
        );

        // Validate the index
        if index.is_empty() {
            self.cursor = Vec::new();
            return PlanResponse::new(Some("root".to_string()), self.distilled_context().context());
        }

        // Check if the index is valid
        let task_opt = self.get_task(index.clone());
        if let Some(task) = task_opt {
            let description = task.description().to_string();

            // Set cursor after we're done with task operations
            self.cursor = index;

            PlanResponse::new(Some(description), self.distilled_context().context())
        } else {
            PlanResponse::new(None, self.distilled_context().context())
        }
    }

    // Task state management
    /// Completes the task at the given index, checking the lease if provided
    pub fn complete_task(
        &mut self,
        index: Index,
        lease_attempt: Option<Lease>,
        force: bool,
        summary: Option<String>,
    ) -> PlanResponse<Result<bool, String>> {
        // Lease check
        if !force {
            if let Some(required_lease) = self.leases.get(&index) {
                if lease_attempt.is_none() {
                    let msg = format!(
                        "Task at index {:?} requires a lease to be completed.",
                        index
                    );
                    self.log_transition("complete_task_failed".to_string(), Some(msg.clone()));
                    return PlanResponse::new(Err(msg), self.distilled_context().context());
                }
                // Compare the full Lease struct (containing u8)
                if lease_attempt != Some(*required_lease) {
                    let msg = format!(
                        "Lease mismatch for task {:?}. Provided: {:?}, Required: {:?}",
                        index,
                        lease_attempt.map(|l| l.value()),
                        required_lease.value()
                    );
                    self.log_transition("complete_task_failed".to_string(), Some(msg.clone()));
                    return PlanResponse::new(Err(msg), self.distilled_context().context());
                }
            }
            // If no lease exists for the index, completion is allowed without a lease (unless forced)
        }

        // Check for summary if force is false
        if !force && summary.is_none() {
            let msg = format!(
                "Task at index {:?} requires a summary for non-forced completion.",
                index
            );
            self.log_transition("complete_task_failed".to_string(), Some(msg.clone()));
            return PlanResponse::new(Err(msg), self.distilled_context().context());
        }

        self.log_transition(
            "complete_task".to_string(),
            Some(format!(
                "Completing task at index: {:?} (force: {})",
                index, force
            )),
        );

        // First, get a clone of the task for generating suggestions
        let task_clone_opt = self.get_task(index.clone()).map(|t| t.clone());

        // Complete the task
        let success = if let Some(task) = self.get_task_mut(index.clone()) {
            task.complete();
            task.completion_summary = summary; // Store the summary
                                               // Remove the lease once completed
            self.leases.remove(&index);
            true
        } else {
            false
        };

        if success {
            // Check if this is the root task being completed
            if index.is_empty() {
                // Root task completed - Verification logic removed as per redesign.
                // Client is now responsible for checks before calling complete.
                self.log_transition("plan_complete_root_task".to_string(), None);
            }

            // Now that we've modified the task, use the clone for suggestions
            if let Some(mut task_clone) = task_clone_opt {
                // Mark the clone as completed
                task_clone.complete();

                // Get level information
                return PlanResponse::new(Ok(success), self.distilled_context().context());
            }
        }

        // Fallback if task not found or clone unavailable
        PlanResponse::new(Ok(success), self.distilled_context().context())
    }

    /// Changes the level of a task at the given index,
    /// returning a followup suggestion and reminder
    pub fn change_level(
        &mut self,
        index: Index,
        level_index: usize,
    ) -> PlanResponse<Result<(), String>> {
        self.log_transition(
            "change_level".to_string(),
            Some(format!(
                "Changing level for task {:?} to {}",
                index, level_index
            )),
        );

        // Validate: the level must exist
        if level_index >= self.plan.level_count() {
            return PlanResponse::new(
                Err(format!("Level index {} is out of bounds", level_index)),
                self.distilled_context().context(),
            );
        }

        // Validate parent-child level relationship
        if !index.is_empty() {
            // This isn't the root task, so check parent level
            let parent_index = index[0..index.len() - 1].to_vec();
            if let Some(parent) = self.get_task(parent_index.clone()) {
                let parent_level = parent.level_index().unwrap_or(parent_index.len());
                if level_index > parent_level {
                    return PlanResponse::new(
                        Err(format!(
                            "Child task cannot have a higher abstraction level ({}) than its parent ({})",
                            level_index, parent_level
                        )),
                        self.distilled_context().context(),
                    );
                }
            }
        }

        // Define a recursive function to check all child levels
        fn check_children(task: &Task, depth: usize, max_level: usize) -> Result<(), String> {
            for subtask in task.subtasks() {
                let subtask_level = subtask.level_index().unwrap_or(depth + 1);
                if subtask_level > max_level {
                    return Err(format!(
                        "Cannot set level to {} because a child task has a higher level ({})",
                        max_level, subtask_level
                    ));
                }

                // Recursively check this subtask's children
                if let Err(e) = check_children(subtask, depth + 1, max_level) {
                    return Err(e);
                }
            }
            Ok(())
        }

        // Validate that no child has a higher level
        if let Some(task) = self.get_task(index.clone()) {
            if let Err(e) = check_children(task, index.len(), level_index) {
                return PlanResponse::new(Err(e), self.distilled_context().context());
            }
        }

        // Apply the change
        if let Some(task) = self.get_task_mut(index.clone()) {
            task.set_level(level_index);
            PlanResponse::new(Ok(()), self.distilled_context().context())
        } else {
            PlanResponse::new(
                Err("Task not found".to_string()),
                self.distilled_context().context(),
            )
        }
    }

    /// Uncompletes the task at the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the task to uncomplete.
    ///
    /// # Returns
    ///
    /// A `PlanResponse` containing a `Result` which is `Ok(true)` on success,
    /// or `Err(String)` if the task could not be found or uncompleted.
    pub fn uncomplete_task(&mut self, index: Index) -> PlanResponse<Result<bool, String>> {
        // Perform mutable operations first to resolve borrow conflicts
        let uncomplete_result = match self.get_task_mut(index.clone()) {
            None => Err("Task not found".to_string()),
            Some(task) => {
                let task_description = task.description().to_string(); // Capture before potential error
                if !task.is_completed() {
                    Err("Task is already incomplete".to_string())
                } else {
                    task.uncomplete();
                    let index_str = index
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    // Log transition *after* task modification but before getting distilled context
                    self.log_transition(
                        "Uncomplete Task".to_string(),
                        Some(format!(
                            "Uncompleted task \"{}\" at index {}",
                            task_description, index_str
                        )),
                    );
                    Ok(true)
                }
            }
        };

        // Now get the distilled context (immutable borrow)
        let distilled = self.distilled_context().context();

        // Construct the final response
        PlanResponse::new(uncomplete_result, distilled)
    }

    // Information retrieval
    /// Gets the task at the given index
    fn get_task(&self, index: Index) -> Option<&Task> {
        if index.is_empty() {
            return Some(self.plan.root());
        }

        let mut current = self.plan.root();
        for &idx in &index {
            if idx >= current.subtasks().len() {
                return None;
            }
            current = &current.subtasks()[idx];
        }

        Some(current)
    }

    /// Gets the task at the given index mutably
    fn get_task_mut(&mut self, index: Index) -> Option<&mut Task> {
        if index.is_empty() {
            return Some(self.plan.root_mut());
        }

        // Using a recursive approach since we can't easily get mutable references
        // through iterative indexing with private fields
        fn get_task_at_path<'a>(task: &'a mut Task, path: &[usize]) -> Option<&'a mut Task> {
            if path.is_empty() {
                return Some(task);
            }

            let idx = path[0];
            let remaining = &path[1..];

            if idx >= task.subtasks().len() {
                None
            } else {
                // We need to reach into the private field directly
                // This is fine since we're in the same crate
                let subtask = &mut task.subtasks[idx];
                get_task_at_path(subtask, remaining)
            }
        }

        // Call the recursive helper
        get_task_at_path(self.plan.root_mut(), &index)
    }

    /// Gets the current task
    pub fn get_current_task(&self) -> Option<&Task> {
        self.get_task(self.cursor.clone())
    }

    /// Gets the current index
    pub fn get_current_index(&self) -> PlanResponse<Index> {
        // Get the current task and level for better context
        let current_task_opt = self.get_current_task();
        if let Some(_current_task) = current_task_opt {
            PlanResponse::new(self.cursor.clone(), self.distilled_context().context())
        } else {
            // Fallback if no current task
            PlanResponse::new(self.cursor.clone(), self.distilled_context().context())
        }
    }

    /// Gets the current task and level based on cursor depth
    pub fn get_current_level(&self) -> usize {
        self.get_current_index().inner().len()
    }

    /// Sets the current level by trimming the cursor
    pub fn set_current_level(&mut self, level: usize) {
        self.log_transition(
            "set_current_level".to_string(),
            Some(format!("Setting current level to: {}", level)),
        );

        while self.cursor.len() > level {
            self.cursor.pop();
        }
    }

    /// Gets subtasks of the task at the given index
    pub fn get_subtasks(&self, index: Index) -> Vec<(Index, &Task)> {
        if let Some(task) = self.get_task(index.clone()) {
            let mut result = Vec::new();
            for (i, subtask) in task.subtasks().iter().enumerate() {
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
    /// Gets the plan
    pub fn get_plan(&self) -> PlanResponse<Plan> {
        PlanResponse::new(self.plan.clone(), self.distilled_context().context())
    }

    /// Gets the current task with history
    pub fn get_current_with_history(&self) -> Option<(Level, Task, Vec<String>)> {
        self.plan.get_with_history(self.cursor.clone())
    }

    /// Builds a task tree from the root to the current task with one level of children at each node
    fn build_task_tree(&self) -> Vec<TaskTreeNode> {
        // Start with root and include all tasks along the path to the current task
        // plus immediate children of the current task
        let current_idx = Vec::new();

        // First, add the root (which is not actually shown to users but is the parent of top-level tasks)
        let root_children = self
            .get_subtasks(current_idx.clone())
            .into_iter()
            .map(|(idx, task)| {
                // Check if this child is on the path to the current task
                let is_on_path = !self.cursor.is_empty() && self.cursor[0] == idx[0];

                // Only include children for tasks on the path to current
                let children = if is_on_path {
                    self.get_subtasks(idx.clone())
                        .into_iter()
                        .map(|(child_idx, child_task)| {
                            // For deeper levels, recursively check if on path
                            let is_on_deeper_path = self.cursor.len() > 1
                                && child_idx.len() <= self.cursor.len()
                                && child_idx == self.cursor[0..child_idx.len()];

                            TaskTreeNode {
                                description: child_task.description().to_string(),
                                index: child_idx.clone(),
                                completed: child_task.is_completed(),
                                is_current: child_idx == self.cursor,
                                completion_summary: child_task.completion_summary().cloned(),
                                children: if is_on_deeper_path {
                                    self.build_subtree(&child_idx)
                                } else {
                                    Vec::new()
                                },
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                TaskTreeNode {
                    description: task.description().to_string(),
                    index: idx.clone(),
                    completed: task.is_completed(),
                    is_current: idx == self.cursor,
                    completion_summary: task.completion_summary().cloned(),
                    children,
                }
            })
            .collect();

        // Add the current task's children if we're at a valid task
        if let Some(_current_task) = self.get_current_task() {
            if !self.cursor.is_empty() {
                // If we're at a valid task, the children were already added above
                return root_children;
            }
        }

        // If we're at root, just return the root's children
        root_children
    }

    /// Helper method to build a subtree for a given index
    fn build_subtree(&self, index: &Index) -> Vec<TaskTreeNode> {
        // Get all subtasks for this index
        self.get_subtasks(index.clone())
            .into_iter()
            .map(|(idx, task)| {
                // Check if this is on the path to current task
                let is_on_path = idx.len() <= self.cursor.len() && idx == self.cursor[0..idx.len()];

                TaskTreeNode {
                    description: task.description().to_string(),
                    index: idx.clone(),
                    completed: task.is_completed(),
                    is_current: idx == self.cursor,
                    completion_summary: task.completion_summary().cloned(),
                    children: if is_on_path {
                        self.build_subtree(&idx)
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect()
    }

    /// Creates a distilled context with focused information about the current planning state
    pub fn distilled_context(&self) -> PlanResponse<()> {
        // Create the usage summary
        let usage_summary = "Scatterbrain is a hierarchical planning tool that helps break down complex tasks into manageable pieces. Use 'task add' to add tasks, 'move <index>' to navigate, and 'task complete' to mark tasks as done. Use '--help' on any command (e.g., `scatterbrain task --help`) for more details. Tasks are organized in levels from high-level planning to specific implementation details.".to_string();

        // Build the task tree from root to current, with one level of children
        let task_tree = self.build_task_tree();

        // Get the current task and level if we're at a valid position
        let (current_level, current_task_opt) = if !self.cursor.is_empty() {
            if let Some((level, task, _)) = self.get_current_with_history() {
                (Some(level), Some(task))
            } else {
                (None, None)
            }
        } else {
            // At root level
            self.plan
                .levels()
                .first()
                .cloned()
                .map(|level| (Some(level), None))
                .unwrap_or((None, None))
        };

        // Get all levels from the plan
        let levels = self.plan.levels().to_vec();

        // Create the distilled context with all components using the new constructor
        let distilled = DistilledContext::new(
            usage_summary,
            task_tree,
            current_task_opt,
            current_level,
            levels,
            self.history.iter().cloned().collect(),
        );

        PlanResponse::new((), distilled)
    }
}

/// Represents a unique identifier for a plan instance.
// Use Lease as the PlanId
pub type PlanId = Lease;

/// Error type for plan operations.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum PlanError {
    #[error("Plan with ID '{0:?}' not found")]
    PlanNotFound(PlanId),
    #[error("Failed to acquire lock for plan operations")]
    LockError, // Simplified lock error representation
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResponse<T> {
    pub res: T,
    pub suggested_followups: Vec<String>,
    pub reminder: Option<String>,
    pub distilled_context: DistilledContext,
}

impl<T> PlanResponse<T> {
    pub fn new(res: T, distilled_context: DistilledContext) -> Self {
        Self {
            res,
            suggested_followups: Vec::new(),
            reminder: None,
            distilled_context,
        }
    }

    pub fn inner(&self) -> &T {
        &self.res
    }

    pub fn into_inner(self) -> T {
        self.res
    }

    pub fn replace<B>(self, res: B) -> PlanResponse<B> {
        PlanResponse {
            res,
            suggested_followups: Vec::new(),
            reminder: None,
            distilled_context: self.distilled_context,
        }
    }

    pub fn context(self) -> DistilledContext {
        self.distilled_context
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Current {
    pub index: Index,
    pub level: Level,
    pub task: Task,
    pub history: Vec<String>,
}

/// Distilled context containing focused information about the current planning state
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DistilledContext {
    /// A summary of what scatterbrain is and how to use it
    pub usage_summary: String,
    /// The task tree from root to the current node, plus one level of children
    pub task_tree: Vec<TaskTreeNode>,
    /// The current task
    pub current_task: Option<Task>,
    /// The current level information
    pub current_level: Option<Level>,
    /// All available abstraction levels
    pub levels: Vec<Level>,
    /// Recent state transition history
    pub transition_history: Vec<TransitionLogEntry>,
}

impl DistilledContext {
    /// Creates a new distilled context with the given components
    pub fn new(
        usage_summary: String,
        task_tree: Vec<TaskTreeNode>,
        current_task: Option<Task>,
        current_level: Option<Level>,
        levels: Vec<Level>,
        transition_history: Vec<TransitionLogEntry>,
    ) -> Self {
        Self {
            usage_summary,
            task_tree,
            current_task,
            current_level,
            levels,
            transition_history,
        }
    }
}

/// A node in the task tree for the distilled context
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskTreeNode {
    /// The description of the task
    pub description: String,
    /// The index path to this task
    pub index: Index,
    /// Whether this task is completed
    pub completed: bool,
    /// Whether this is the current task
    pub is_current: bool,
    /// Optional completion summary
    pub completion_summary: Option<String>,
    /// Child tasks (only included for the current task and its ancestors)
    pub children: Vec<TaskTreeNode>,
}

#[derive(Clone)]
pub struct Core {
    // Use RwLock for better concurrency with multiple readers (API calls)
    // Store multiple Contexts keyed by PlanId
    inner: Arc<RwLock<HashMap<PlanId, Context>>>,
    // Broadcast channel now sends the PlanId (Lease) that was updated
    update_tx: Arc<tokio::sync::broadcast::Sender<PlanId>>,
}

impl Core {
    /// Creates a new Core instance, initializing with a default plan.
    pub fn new() -> Self {
        // Create a broadcast channel for PlanId updates
        let (tx, _rx) = tokio::sync::broadcast::channel(100);

        // Initialize the map with a default plan context
        let mut plans = HashMap::new();
        // Use the static DEFAULT_PLAN_ID for the default plan's context
        let default_context = Context::default_with_seed(0);
        plans.insert(*DEFAULT_PLAN_ID, default_context);

        Self {
            inner: Arc::new(RwLock::new(plans)),
            update_tx: Arc::new(tx),
        }
    }

    /// Helper method to safely access a specific plan's context and potentially modify it.
    /// Notifies observers about state changes for the specific plan token.
    pub fn with_plan_context<F, R>(&self, id: &PlanId, f: F) -> Result<R, PlanError>
    where
        F: FnOnce(&mut Context) -> R, // Closure now operates on the specific context
    {
        // Get write lock to potentially modify the context
        let mut plans = self.inner.write().map_err(|_| PlanError::LockError)?;

        // Get the mutable context for the given id
        let context = plans
            .get_mut(id)
            .ok_or_else(|| PlanError::PlanNotFound(*id))?;

        // Apply the function to the specific context
        let result = f(context);

        // Notify observers about state change for this specific plan id
        let _ = self.update_tx.send(*id); // Send the id

        Ok(result)
    }

    /// Helper method to safely access a specific plan's context immutably.
    fn with_plan_context_read<F, R>(&self, id: &PlanId, f: F) -> Result<R, PlanError>
    where
        F: FnOnce(&Context) -> R, // Closure operates immutably
    {
        // Get read lock
        let plans = self.inner.read().map_err(|_| PlanError::LockError)?;

        // Get the immutable context for the given id
        let context = plans.get(id).ok_or_else(|| PlanError::PlanNotFound(*id))?;

        // Apply the function
        let result = f(context);

        Ok(result)
    }

    /// Creates a new plan with the given goal and returns its unique ID (Lease).
    /// Handles potential collisions if a randomly generated u8 ID already exists.
    pub fn create_plan(&self, goal: Option<String>) -> Result<PlanId, PlanError> {
        let mut plans = self.inner.write().map_err(|_| PlanError::LockError)?;

        let mut new_id_val;
        loop {
            new_id_val = rand::random::<u8>();
            let potential_id = Lease(new_id_val);
            if !plans.contains_key(&potential_id) {
                // Found an unused ID
                break;
            }
            // ID collision, loop again to generate a new one
        }

        let new_id = Lease(new_id_val);
        // Create a new plan with the provided goal
        let plan = Plan::new(default_levels(), goal);
        // Use a random seed for new plans, creating context directly with seed
        let new_context = Context::new_with_seed(plan, rand::random());
        plans.insert(new_id, new_context);

        // Notify about the creation
        let _ = self.update_tx.send(new_id);

        Ok(new_id)
    }

    /// Deletes a plan context identified by its ID.
    // Use id: &PlanId instead of token: &PlanToken
    pub fn delete_plan(&self, id: &PlanId) -> Result<(), PlanError> {
        let mut plans = self.inner.write().map_err(|_| PlanError::LockError)?;

        if !plans.contains_key(id) {
            return Err(PlanError::PlanNotFound(*id));
        }

        plans.remove(id);

        // Notify about the deletion
        let _ = self.update_tx.send(*id);

        Ok(())
    }

    // Subscribe to state updates for ANY plan.
    // Subscribers will need to filter based on the received PlanId.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PlanId> {
        self.update_tx.subscribe()
    }

    // --- Methods below use PlanId ---

    pub fn get_plan(&self, id: &PlanId) -> Result<PlanResponse<Plan>, PlanError> {
        self.with_plan_context_read(id, |context| context.get_plan())
    }

    pub fn current(&self, id: &PlanId) -> Result<PlanResponse<Option<Current>>, PlanError> {
        self.with_plan_context_read(id, |context| {
            let PlanResponse { res: index, .. } = context.get_current_index();
            let current_opt = context
                .get_current_with_history()
                .map(|(level, task, history)| Current {
                    index,
                    level,
                    task,
                    history,
                });
            // Use context.distilled_context() to get the response shell
            context.distilled_context().replace(current_opt)
        })
    }

    pub fn add_task(
        &self,
        id: &PlanId,
        description: String,
        level_index: usize,
    ) -> Result<PlanResponse<(Task, Index)>, PlanError> {
        self.with_plan_context(id, |context| context.add_task(description, level_index))
    }

    pub fn complete_task(
        &self,
        id: &PlanId,
        index: Index,
        lease_attempt: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> Result<PlanResponse<bool>, PlanError> {
        self.with_plan_context(id, |context| {
            let lease_attempt_typed = lease_attempt.map(Lease);
            let result_response = context.complete_task(index, lease_attempt_typed, force, summary);
            let inner_result = result_response.into_inner();
            let distilled_context = context.distilled_context().distilled_context;
            match inner_result {
                Ok(success) => PlanResponse::new(success, distilled_context),
                Err(e) => {
                    eprintln!("Error completing task in plan {:?}: {}", id, e);
                    PlanResponse::new(false, distilled_context)
                }
            }
        })
    }

    pub fn move_to(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<Option<String>>, PlanError> {
        self.with_plan_context(id, |context| context.move_to(index))
    }

    /// Generate a lease for the task at the given index
    pub fn generate_lease(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<(Lease, Vec<String>)>, PlanError> {
        self.with_plan_context(id, |context| context.generate_lease(index))
    }

    /// Removes the task at the given index
    pub fn remove_task(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<Result<Task, String>>, PlanError> {
        self.with_plan_context(id, |context| context.remove_task(index))
    }

    /// Uncompletes the task at the given index.
    pub fn uncomplete_task(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<Result<bool, String>>, PlanError> {
        self.with_plan_context(id, |context| context.uncomplete_task(index))
    }

    /// Changes the level of a task at the given index
    pub fn change_level(
        &self,
        id: &PlanId,
        index: Index,
        level_index: usize,
    ) -> Result<PlanResponse<Result<(), String>>, PlanError> {
        self.with_plan_context(id, |context| context.change_level(index, level_index))
    }

    pub fn get_current_index(&self, id: &PlanId) -> Result<PlanResponse<Index>, PlanError> {
        self.with_plan_context_read(id, |context| context.get_current_index())
    }

    /// Gets a distilled context with focused information about the current planning state
    pub fn distilled_context(&self, id: &PlanId) -> Result<PlanResponse<()>, PlanError> {
        self.with_plan_context_read(id, |context| context.distilled_context())
    }

    /// Lists all available plan IDs.
    pub fn list_plans(&self) -> Result<Vec<PlanId>, PlanError> {
        let plans = self.inner.read().map_err(|_| PlanError::LockError)?;
        Ok(plans.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a Core instance for testing
    fn setup_core() -> Core {
        Core::new() // Core::new now initializes a default plan
    }

    // Helper to get the default token
    fn default_token() -> PlanId {
        // Return the static default plan ID
        *DEFAULT_PLAN_ID
    }

    #[test]
    fn test_core_new_and_default_plan() {
        let core = setup_core();
        let plans = core.inner.read().unwrap();
        assert_eq!(plans.len(), 1, "Should initialize with one default plan");
        assert!(
            plans.contains_key(&default_token()),
            "Default plan ID should exist"
        );
    }

    #[test]
    fn test_core_create_and_delete_plan() {
        let core = setup_core();

        // Create a new plan - check it's not the default ID
        let id1 = core.create_plan(None).expect("Failed to create plan 1");
        assert_ne!(id1, default_token(), "New ID should not be the default");

        // Verify it exists
        {
            let plans = core.inner.read().unwrap();
            assert_eq!(plans.len(), 2, "Should have default + new plan");
            assert!(plans.contains_key(&id1));
        } // Read lock released

        // Create another plan
        let id2 = core.create_plan(None).expect("Failed to create plan 2");
        assert_ne!(id2, default_token());
        assert_ne!(id2, id1);

        // Verify it exists
        {
            let plans = core.inner.read().unwrap();
            assert_eq!(plans.len(), 3, "Should have default + two new plans");
            assert!(plans.contains_key(&id2));
        } // Read lock released

        // Delete the first new plan
        core.delete_plan(&id1).expect("Failed to delete plan 1");

        // Verify it's gone, but others remain
        {
            let plans = core.inner.read().unwrap();
            assert_eq!(
                plans.len(),
                2,
                "Should have default + second new plan after deletion"
            );
            assert!(!plans.contains_key(&id1));
            assert!(plans.contains_key(&default_token())); // Use helper
            assert!(plans.contains_key(&id2));
        } // Read lock released

        // Try deleting non-existent plan - create a Lease not likely to exist
        let non_existent_id = Lease::new(255); // Use constructor
                                               // Ensure the non-existent id is actually not present before testing deletion
                                               // Account for the possibility that 255 was randomly generated
        while core.inner.read().unwrap().contains_key(&non_existent_id) {
            // This is highly unlikely with only a few plans, but safer
            panic!("Test setup failed: Non-existent ID 255 was already present.");
        }
        let delete_err = core.delete_plan(&non_existent_id).unwrap_err();
        match delete_err {
            PlanError::PlanNotFound(t) => assert_eq!(t, non_existent_id),
            _ => panic!("Expected PlanNotFound error"),
        }

        // Try deleting default plan (should work for now)
        core.delete_plan(&default_token()) // Use helper
            .expect("Failed to delete default plan");
        {
            let plans = core.inner.read().unwrap();
            assert_eq!(plans.len(), 1, "Only second new plan should remain");
            assert!(plans.contains_key(&id2));
            assert!(!plans.contains_key(&default_token())); // Use helper
        }
    }

    #[test]
    fn test_core_list_plans() {
        let core = setup_core();
        let initial_ids = core.list_plans().unwrap();
        assert_eq!(initial_ids.len(), 1);
        assert!(initial_ids.contains(&default_token())); // Use helper

        let id1 = core.create_plan(None).unwrap();
        let id2 = core.create_plan(None).unwrap();

        let current_ids = core.list_plans().unwrap();
        assert_eq!(current_ids.len(), 3);
        assert!(current_ids.contains(&default_token())); // Use helper
        assert!(current_ids.contains(&id1));
        assert!(current_ids.contains(&id2));

        core.delete_plan(&id1).unwrap();
        let after_delete_ids = core.list_plans().unwrap();
        assert_eq!(after_delete_ids.len(), 2);
        assert!(after_delete_ids.contains(&default_token())); // Use helper
        assert!(after_delete_ids.contains(&id2));
        assert!(!after_delete_ids.contains(&id1));
    }

    #[test]
    fn test_create_plan_with_goal() {
        let core = setup_core();
        let goal = "Test goal for the plan".to_string();
        let id = core
            .create_plan(Some(goal.clone()))
            .expect("Failed to create plan with goal");

        // Verify the goal is stored
        let plan_response = core.get_plan(&id).expect("Failed to get created plan");
        let plan = plan_response.inner();
        assert_eq!(plan.goal, Some(goal));

        // Verify creating a plan without a goal still works
        let id_no_goal = core
            .create_plan(None)
            .expect("Failed to create plan without goal");
        let plan_response_no_goal = core
            .get_plan(&id_no_goal)
            .expect("Failed to get plan without goal");
        let plan_no_goal = plan_response_no_goal.inner();
        assert_eq!(plan_no_goal.goal, None);
    }

    // Example modification for one test:
    #[test]
    fn test_task_creation_and_navigation_multi_plan() {
        let core = setup_core();
        let id = default_token(); // Use the default plan ID

        // Add a task at the root level - Use updated Core method signature
        let response = core
            .add_task(&id, "Task 1".to_string(), 0)
            .expect("Failed to add task 1");
        let task1_index = response.into_inner().1;
        assert_eq!(task1_index, vec![0]);

        // Add another task at the root level
        let response = core
            .add_task(&id, "Task 2".to_string(), 0)
            .expect("Failed to add task 2");
        let task2_index = response.into_inner().1;
        assert_eq!(task2_index, vec![1]);

        // Move to the first task
        let move_response = core
            .move_to(&id, task1_index.clone())
            .expect("Failed to move to task 1");
        assert_eq!(move_response.inner(), &Some("Task 1".to_string()));

        // Add a subtask to the first task
        let response = core
            .add_task(&id, "Subtask 1".to_string(), 1)
            .expect("Failed to add subtask 1");
        let subtask1_index = response.into_inner().1;
        assert_eq!(subtask1_index, vec![0, 0]);

        // Move to the second task
        let move_response = core
            .move_to(&id, task2_index.clone())
            .expect("Failed to move to task 2");
        assert_eq!(move_response.inner(), &Some("Task 2".to_string()));
        let current_index_resp = core
            .get_current_index(&id)
            .expect("Failed to get current index");
        assert_eq!(*current_index_resp.inner(), vec![1]);

        // Move to subtask 1
        let move_response = core
            .move_to(&id, subtask1_index.clone())
            .expect("Failed to move to subtask 1");
        assert_eq!(move_response.inner(), &Some("Subtask 1".to_string()));
        let current_index_resp = core
            .get_current_index(&id)
            .expect("Failed to get current index");
        assert_eq!(*current_index_resp.inner(), vec![0, 0]);

        // Test with a second plan
        let id2 = core.create_plan(None).unwrap();
        let response2 = core
            .add_task(&id2, "Task A Plan 2".to_string(), 0)
            .expect("Failed to add task to plan 2");
        let task_a_index2 = response2.into_inner().1;
        assert_eq!(task_a_index2, vec![0]);

        // Check current index of plan 1 is unchanged
        let current_index_resp1_again = core
            .get_current_index(&id)
            .expect("Failed to get current index for plan 1 again");
        assert_eq!(*current_index_resp1_again.inner(), vec![0, 0]);

        // Check the root task's subtasks in plan 2.
        let plan2_response = core.get_plan(&id2).expect("Failed to get plan 2");
        let plan2 = plan2_response.inner();
        assert!(!plan2.root().subtasks().is_empty());
        assert_eq!(plan2.root().subtasks()[0].description(), "Task A Plan 2");
    }

    // TODO: Update ALL remaining tests similarly...
    #[test]
    fn test_task_completion() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();
        // Adding a root task isn't strictly necessary now as Core::new creates a default plan
        // core.add_task(&token, "Root task".to_string(), 0).unwrap();

        // Add tasks
        let task1_index = core
            .add_task(&token, "Task 1".to_string(), 0)
            .expect("Failed to add task 1")
            .into_inner()
            .1;
        let task2_index = core
            .add_task(&token, "Task 2".to_string(), 0)
            .expect("Failed to add task 2")
            .into_inner()
            .1;

        // Complete a task (provide summary or force)
        let complete_response = core
            .complete_task(
                &token,
                task1_index.clone(),
                None, // No lease needed unless generated
                false,
                Some("Test summary".to_string()), // Summary required if not forcing
            )
            .expect("Failed to complete task 1");
        assert!(*complete_response.inner());

        // Verify the task is completed
        let plan_response = core.get_plan(&token).expect("Failed to get plan");
        // Use get_with_history to fetch the task and its context
        let (_, task1, _) = plan_response
            .inner()
            .get_with_history(task1_index)
            .expect("Task 1 not found");
        assert!(task1.is_completed());
        assert_eq!(
            task1.completion_summary(),
            Some(&"Test summary".to_string())
        );

        // Verify the other task is not completed
        let plan_response_again = core.get_plan(&token).expect("Failed to get plan again");
        let (_, task2, _) = plan_response_again
            .inner()
            .get_with_history(task2_index)
            .expect("Task 2 not found");
        assert!(!task2.is_completed());
    }

    #[test]
    fn test_get_subtasks_via_plan() {
        // Renamed as get_subtasks is on Context, not Core
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();
        // No need to add root task

        // Add tasks
        let task1_index = core
            .add_task(&token, "Task 1".to_string(), 0)
            .expect("Failed to add task 1")
            .into_inner()
            .1;
        let task2_index = core
            .add_task(&token, "Task 2".to_string(), 0)
            .expect("Failed to add task 2")
            .into_inner()
            .1;

        // Move to the first task and add subtasks
        core.move_to(&token, task1_index.clone())
            .expect("Failed to move to task 1");
        let subtask1_index = core
            .add_task(&token, "Subtask 1".to_string(), 1)
            .expect("Failed to add subtask 1")
            .into_inner()
            .1;
        let subtask2_index = core
            .add_task(&token, "Subtask 2".to_string(), 1)
            .expect("Failed to add subtask 2")
            .into_inner()
            .1;

        // Get subtasks via reading the plan and context
        // get_subtasks is on Context, so we use with_plan_context_read
        // Clone tasks inside closure to avoid lifetime issues
        let subtasks: Vec<(Index, Task)> = core
            .with_plan_context_read(&token, |ctx| {
                ctx.get_subtasks(task1_index.clone())
                    .into_iter()
                    .map(|(idx, task_ref)| (idx, task_ref.clone())) // Clone Task
                    .collect()
            })
            .expect("Failed to read context for subtasks");

        assert_eq!(subtasks.len(), 2);
        // Now we have owned Tasks, we can assert directly
        assert_eq!(subtasks[0].0, subtask1_index);
        assert_eq!(subtasks[0].1.description(), "Subtask 1");
        assert_eq!(subtasks[1].0, subtask2_index);
        assert_eq!(subtasks[1].1.description(), "Subtask 2");

        // Get subtasks of the second task (should be empty)
        let subtasks_2: Vec<(Index, Task)> = core
            .with_plan_context_read(&token, |ctx| {
                ctx.get_subtasks(task2_index.clone())
                    .into_iter()
                    .map(|(idx, task_ref)| (idx, task_ref.clone())) // Clone Task
                    .collect()
            })
            .expect("Failed to read context for subtasks 2");
        assert_eq!(subtasks_2.len(), 0);
    }

    #[test]
    fn test_lease_system() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        let task_idx = core
            .add_task(&token, "Lease Task".to_string(), 0)
            .expect("Add task")
            .into_inner()
            .1;

        // Generate lease
        let lease_resp = core
            .generate_lease(&token, task_idx.clone())
            .expect("Generate lease");
        let (lease, _) = lease_resp.inner(); // Ignore suggestions for this test

        // Attempt completion without lease - should fail
        let complete_fail_resp = core
            .complete_task(
                &token,
                task_idx.clone(),
                None,
                false, // Not forcing
                Some("Attempt 1".to_string()),
            )
            .expect("Attempt complete without lease"); // Expect Ok(PlanResponse) even on logical failure
        assert!(
            !*complete_fail_resp.inner(),
            "Completion should fail without lease"
        );

        // Attempt completion with wrong lease - should fail
        // Generate a different u8 lease value
        let wrong_lease_val = lease.value().wrapping_add(1);
        let complete_fail_wrong_resp = core
            .complete_task(
                &token,
                task_idx.clone(),
                Some(wrong_lease_val), // Pass the wrong u8 value
                false,
                Some("Attempt 2".to_string()),
            )
            .expect("Attempt complete with wrong lease");
        assert!(
            !*complete_fail_wrong_resp.inner(),
            "Completion should fail with wrong lease"
        );

        // Attempt completion with correct lease - should succeed
        let complete_success_resp = core
            .complete_task(
                &token,
                task_idx.clone(),
                Some(lease.value()), // Pass the correct u8 value
                false,
                Some("Attempt 3 Success".to_string()),
            )
            .expect("Attempt complete with correct lease");
        assert!(
            *complete_success_resp.inner(),
            "Completion should succeed with correct lease"
        );

        // Verify task is completed
        let plan = core.get_plan(&token).expect("Get plan").into_inner();
        let (_, task, _) = plan
            .get_with_history(task_idx)
            .expect("Task not found after completion");
        assert!(task.is_completed());
    }

    #[test]
    fn test_task_completion_with_summary() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        let task_idx = core
            .add_task(&token, "Summary Task".to_string(), 0)
            .expect("Add task")
            .into_inner()
            .1;

        // Attempt completion without summary (and not forcing) - should fail
        let fail_resp = core
            .complete_task(
                &token,
                task_idx.clone(),
                None,  // No lease
                false, // Not forcing
                None,  // NO SUMMARY
            )
            .expect("Attempt complete without summary");
        assert!(
            !*fail_resp.inner(),
            "Completion should fail without summary"
        );

        // Attempt completion with summary - should succeed
        let summary = "Task completed successfully.".to_string();
        let success_resp = core
            .complete_task(&token, task_idx.clone(), None, false, Some(summary.clone()))
            .expect("Attempt complete with summary");
        assert!(
            *success_resp.inner(),
            "Completion should succeed with summary"
        );

        // Verify task is completed and has summary
        let plan = core.get_plan(&token).expect("Get plan").into_inner();
        let (_, task, _) = plan.get_with_history(task_idx).expect("Task not found");
        assert!(task.is_completed());
        assert_eq!(task.completion_summary(), Some(&summary));

        // Test forcing completion without summary
        let task2_idx = core
            .add_task(&token, "Force Complete Task".to_string(), 0)
            .expect("Add task 2")
            .into_inner()
            .1;
        let force_resp = core
            .complete_task(
                &token,
                task2_idx.clone(),
                None,
                true, // Force completion
                None, // No summary
            )
            .expect("Attempt force complete");
        assert!(
            *force_resp.inner(),
            "Forced completion should succeed without summary"
        );

        let plan2 = core.get_plan(&token).expect("Get plan again").into_inner();
        let (_, task2, _) = plan2.get_with_history(task2_idx).expect("Task 2 not found");
        assert!(task2.is_completed());
        assert!(task2.completion_summary().is_none());
    }

    #[test]
    fn test_task_removal() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        // Add tasks: Task1 -> Subtask1, Task2
        let task1_idx = core
            .add_task(&token, "Task 1".to_string(), 0)
            .expect("Add T1")
            .into_inner()
            .1;
        core.move_to(&token, task1_idx.clone()).expect("Move T1");
        let sub1_idx = core
            .add_task(&token, "Subtask 1".to_string(), 1)
            .expect("Add S1")
            .into_inner()
            .1;
        core.move_to(&token, vec![]).expect("Move Root"); // Move back to root to add Task 2
        let task2_idx = core
            .add_task(&token, "Task 2".to_string(), 0)
            .expect("Add T2")
            .into_inner()
            .1;

        // Generate lease for subtask to test lease removal
        let lease_resp = core
            .generate_lease(&token, sub1_idx.clone())
            .expect("Generate lease");
        let _lease_val = lease_resp.inner().0.value();

        // Remove Subtask1
        let remove_resp = core
            .remove_task(&token, sub1_idx.clone())
            .expect("Remove S1 Result");
        let removed_task_result = remove_resp.inner();
        assert!(removed_task_result.is_ok(), "Removal should succeed");
        assert_eq!(
            removed_task_result.as_ref().unwrap().description(),
            "Subtask 1"
        );

        // Verify Subtask1 is gone
        let plan1 = core
            .get_plan(&token)
            .expect("Get Plan after S1 remove")
            .into_inner();
        let (_level, task1, _hist) = plan1
            .get_with_history(task1_idx.clone())
            .expect("T1 should still exist");
        assert!(task1.subtasks().is_empty(), "T1 should have no subtasks");

        // Verify lease for Subtask1 is gone (by trying to complete with it - should fail if task gone)
        // We can't directly check internal lease map state easily from Core API.
        // Trying to complete a removed task should fail, but how?
        // Let's try completing Task 1 (parent) to see if state is consistent.
        core.complete_task(
            &token,
            task1_idx.clone(),
            None,
            false,
            Some("Complete T1".to_string()),
        )
        .expect("Complete T1");

        // Remove Task2
        let remove_resp2 = core
            .remove_task(&token, task2_idx.clone())
            .expect("Remove T2 Result");
        assert!(remove_resp2.inner().is_ok());
        assert_eq!(
            remove_resp2.inner().as_ref().unwrap().description(),
            "Task 2"
        );

        // Verify Task2 is gone
        let plan2 = core
            .get_plan(&token)
            .expect("Get Plan after T2 remove")
            .into_inner();
        assert_eq!(plan2.root().subtasks().len(), 1); // Only Task1 (completed) should remain
        assert_eq!(plan2.root().subtasks()[0].description(), "Task 1");

        // Try removing root (should fail)
        let remove_root_resp = core
            .remove_task(&token, vec![])
            .expect("Remove Root Result");
        assert!(remove_root_resp.inner().is_err());
        // Check error message if possible/stable
        // assert!(remove_root_resp.inner().err().unwrap().contains("Cannot remove the root task"));

        // Try removing non-existent index
        let remove_bad_idx_resp = core
            .remove_task(&token, vec![0, 99])
            .expect("Remove Bad Index Result");
        assert!(remove_bad_idx_resp.inner().is_err());
    }

    #[test]
    fn test_task_uncompletion() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        let task_idx = core
            .add_task(&token, "Uncomplete Me".to_string(), 0)
            .expect("Add task")
            .into_inner()
            .1;

        // Complete the task first
        core.complete_task(
            &token,
            task_idx.clone(),
            None,
            false,
            Some("Initial completion".to_string()),
        )
        .expect("Initial complete");

        // Verify it's completed
        let plan1 = core.get_plan(&token).expect("Get plan 1").into_inner();
        let (_, task1, _) = plan1
            .get_with_history(task_idx.clone())
            .expect("Task not found 1");
        assert!(task1.is_completed());
        assert!(task1.completion_summary().is_some());

        // Uncomplete the task
        let uncomplete_resp = core
            .uncomplete_task(&token, task_idx.clone())
            .expect("Uncomplete Result");
        let uncomplete_inner_result = uncomplete_resp.inner();
        assert!(
            uncomplete_inner_result.is_ok(),
            "Uncompletion should succeed"
        );
        assert!(
            *uncomplete_inner_result.as_ref().unwrap(),
            "Uncompletion inner bool should be true"
        );

        // Verify it's not completed and summary is gone
        let plan2 = core.get_plan(&token).expect("Get plan 2").into_inner();
        let (_, task2, _) = plan2
            .get_with_history(task_idx.clone())
            .expect("Task not found 2");
        assert!(!task2.is_completed());
        assert!(task2.completion_summary().is_none());

        // Try uncompleting again (should fail)
        let uncomplete_again_resp = core
            .uncomplete_task(&token, task_idx.clone())
            .expect("Uncomplete Again Result");
        assert!(uncomplete_again_resp.inner().is_err());
        // Check error message if possible/stable
        // assert!(uncomplete_again_resp.inner().err().unwrap().contains("Task is already incomplete"));
    }

    #[test]
    fn test_add_task_uncompletes_parents() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        // Create structure: T1 -> S1 -> SS1
        let t1_idx = core
            .add_task(&token, "T1".to_string(), 0)
            .expect("Add T1")
            .into_inner()
            .1;
        core.move_to(&token, t1_idx.clone()).expect("Move T1");
        let s1_idx = core
            .add_task(&token, "S1".to_string(), 1)
            .expect("Add S1")
            .into_inner()
            .1;
        core.move_to(&token, s1_idx.clone()).expect("Move S1");
        let ss1_idx = core
            .add_task(&token, "SS1".to_string(), 2)
            .expect("Add SS1")
            .into_inner()
            .1;

        // Complete all tasks from bottom up
        core.complete_task(
            &token,
            ss1_idx.clone(),
            None,
            false,
            Some("Done SS1".to_string()),
        )
        .expect("Complete SS1");
        core.complete_task(
            &token,
            s1_idx.clone(),
            None,
            false,
            Some("Done S1".to_string()),
        )
        .expect("Complete S1");
        core.complete_task(
            &token,
            t1_idx.clone(),
            None,
            false,
            Some("Done T1".to_string()),
        )
        .expect("Complete T1");

        // Verify all are completed
        let plan1 = core.get_plan(&token).expect("Get Plan 1").into_inner();
        let (_, t1, _) = plan1.get_with_history(t1_idx.clone()).expect("Get T1");
        let (_, s1, _) = plan1.get_with_history(s1_idx.clone()).expect("Get S1");
        let (_, ss1, _) = plan1.get_with_history(ss1_idx.clone()).expect("Get SS1");
        assert!(t1.is_completed());
        assert!(s1.is_completed());
        assert!(ss1.is_completed());

        // Add a new subtask to S1 (S2)
        core.move_to(&token, s1_idx.clone()).expect("Move S1 again");
        let _s2_idx = core
            .add_task(&token, "S2".to_string(), 1)
            .expect("Add S2")
            .into_inner()
            .1;

        // Verify T1 and S1 are now incomplete, but SS1 remains complete
        let plan2 = core.get_plan(&token).expect("Get Plan 2").into_inner();
        let (_, t1_after, _) = plan2
            .get_with_history(t1_idx.clone())
            .expect("Get T1 after");
        let (_, s1_after, _) = plan2
            .get_with_history(s1_idx.clone())
            .expect("Get S1 after");
        let (_, ss1_after, _) = plan2
            .get_with_history(ss1_idx.clone())
            .expect("Get SS1 after");

        assert!(!t1_after.is_completed(), "T1 should be incomplete");
        assert!(!s1_after.is_completed(), "S1 should be incomplete");
        assert!(ss1_after.is_completed(), "SS1 should remain complete"); // Existing children aren't affected
    }

    #[test]
    fn test_generate_lease_suggestions() {
        // Update test for PlanId and Result handling
        let core = setup_core();
        let token = default_token();

        // Generate lease for root task (empty index)
        let root_lease_resp = core
            .generate_lease(&token, vec![])
            .expect("Generate root lease");
        let (_, root_suggestions) = root_lease_resp.inner();
        assert!(
            !root_suggestions.is_empty(),
            "Root task lease should provide suggestions"
        );
        // Check for a known suggestion (fragile, but okay for now)
        assert!(root_suggestions
            .iter()
            .any(|s| s.contains("compilation passes")));

        // Generate lease for a non-root task
        let task_idx = core
            .add_task(&token, "Non-root Task".to_string(), 0)
            .expect("Add task")
            .into_inner()
            .1;
        let non_root_lease_resp = core
            .generate_lease(&token, task_idx.clone())
            .expect("Generate non-root lease");
        let (_, non_root_suggestions) = non_root_lease_resp.inner();
        assert!(
            non_root_suggestions.is_empty(),
            "Non-root task lease should not provide suggestions"
        );
    }
}
