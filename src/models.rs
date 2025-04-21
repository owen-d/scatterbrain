//! Core models for the scatterbrain library
//!
//! This module contains the core data types and business logic for the scatterbrain tool.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
}

impl Plan {
    /// Creates a new plan with the given levels
    pub fn new(levels: Vec<Level>) -> Self {
        Self {
            root: Task::new("root".to_string()),
            levels,
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
    /// Returns the inner value of the lease.
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Context for managing the planning process
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
            rng: StdRng::seed_from_u64(0),                      // Initialize RNG with seed 0
        }
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
                if lease_attempt != Some(*required_lease) {
                    let msg = format!(
                        "Lease mismatch for task {:?}. Provided: {:?}, Required: {:?}",
                        index, lease_attempt, required_lease
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

    /// Gets the current task mutably
    fn get_current_task_mut(&mut self) -> Option<&mut Task> {
        self.get_task_mut(self.cursor.clone())
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

    // Helper method to safely access context and notify observers about state changes
    fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Context) -> R,
    {
        // Get context from mutex
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Apply the function to context
        let result = f(&mut context);

        // Notify observers about state changes
        let _ = self.update_tx.send(());

        result
    }

    pub fn get_plan(&self) -> PlanResponse<Plan> {
        let context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        context.get_plan()
    }

    pub fn current(&self) -> PlanResponse<Option<Current>> {
        self.with_context(|context| {
            let PlanResponse {
                res: index,
                distilled_context,
                ..
            } = context.get_current_index();

            let current_opt = context
                .get_current_with_history()
                .map(|(level, task, history)| Current {
                    index,
                    level,
                    task,
                    history,
                });

            PlanResponse::new(current_opt, distilled_context)
        })
    }

    pub fn add_task(&self, description: String, level_index: usize) -> PlanResponse<(Task, Index)> {
        self.with_context(|context| context.add_task(description, level_index))
    }

    pub fn complete_task(
        &self,
        index: Index,
        lease_attempt: Option<u8>,
        force: bool,
        summary: Option<String>,
    ) -> PlanResponse<bool> {
        self.with_context(|context| {
            // Map the Option<u8> to Option<Lease>
            let lease_attempt_typed = lease_attempt.map(Lease);

            // Call the context method which returns PlanResponse<Result<bool, String>>
            let result_response = context.complete_task(index, lease_attempt_typed, force, summary);

            // Extract the inner Result<bool, String>
            let inner_result = result_response.into_inner();

            // Get the *current* distilled context by calling the method and extracting the field
            let distilled_context = context.distilled_context().distilled_context;

            // Now package the final PlanResponse<bool>
            match inner_result {
                Ok(success) => PlanResponse::new(success, distilled_context),
                Err(e) => {
                    eprintln!("Error completing task: {}", e);
                    PlanResponse::new(false, distilled_context) // Return false in case of error
                }
            }
        })
    }

    pub fn move_to(&self, index: Index) -> PlanResponse<Option<String>> {
        self.with_context(|context| context.move_to(index))
    }

    /// Generate a lease for the task at the given index
    /// Returns the lease and verification suggestions (if any)
    pub fn generate_lease(&self, index: Index) -> PlanResponse<(Lease, Vec<String>)> {
        self.with_context(|context| context.generate_lease(index))
    }

    /// Removes the task at the given index
    pub fn remove_task(&self, index: Index) -> PlanResponse<Result<Task, String>> {
        self.with_context(|context| context.remove_task(index))
    }

    /// Uncompletes the task at the given index.
    pub fn uncomplete_task(&self, index: Index) -> PlanResponse<Result<bool, String>> {
        // Pass a closure to with_context that calls context.uncomplete_task.
        // The with_context helper automatically wraps the returned PlanResponse.
        self.with_context(|context| context.uncomplete_task(index))
    }

    // Subscribe to state updates
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.update_tx.subscribe()
    }

    /// Changes the level of a task at the given index
    pub fn change_level(
        &self,
        index: Index,
        level_index: usize,
    ) -> PlanResponse<Result<(), String>> {
        self.with_context(|context| context.change_level(index, level_index))
    }

    pub fn get_current_index(&self) -> PlanResponse<Index> {
        self.with_context(|context| context.get_current_index())
    }

    /// Gets a distilled context with focused information about the current planning state
    /// This return type embeds () because the context is already embedded in the PlanResponse type
    pub fn distilled_context(&self) -> PlanResponse<()> {
        self.with_context(|context| context.distilled_context())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation_and_navigation() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        assert_eq!(*context.get_current_index().inner(), Vec::<usize>::new());
        // Add a task at the root level
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        assert_eq!(task1_index, vec![0]);

        // Add another task at the root level
        let task2_index = context.add_task("Task 2".to_string(), 0).into_inner().1;
        assert_eq!(task2_index, vec![1]);

        // Move to the first task
        let move_response = context.move_to(task1_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 1".to_string()));

        // Add a subtask to the first task
        let subtask1_index = context.add_task("Subtask 1".to_string(), 1).into_inner().1;
        assert_eq!(subtask1_index, vec![0, 0]);

        // Move to the second task
        let move_response = context.move_to(task2_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 2".to_string()));
        assert_eq!(*context.get_current_index().inner(), vec![1]);

        // Move to subtask 1
        let move_response = context.move_to(subtask1_index.clone());
        assert_eq!(move_response.inner(), &Some("Subtask 1".to_string()));
        assert_eq!(*context.get_current_index().inner(), vec![0, 0]);
    }

    #[test]
    fn test_task_completion() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string(), 0);

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        let task2_index = context.add_task("Task 2".to_string(), 0).into_inner().1;

        // Complete a task (provide summary or force)
        assert!(context
            .complete_task(
                task1_index.clone(),
                None,
                false,
                Some("Test summary".to_string())
            ) // Provide summary
            .inner()
            .as_ref()
            .unwrap());

        // Verify the task is completed
        let task = context.get_task(task1_index).unwrap();
        assert!(task.is_completed());

        // Verify the other task is not completed
        let task = context.get_task(task2_index).unwrap();
        assert!(!task.is_completed());
    }

    #[test]
    fn test_get_subtasks() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string(), 0);

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        let task2_index = context.add_task("Task 2".to_string(), 0).into_inner().1;

        // Move to the first task and add subtasks
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string(), 1).into_inner().1;
        let subtask2_index = context.add_task("Subtask 2".to_string(), 1).into_inner().1;

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
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        let root_index = context.add_task("Root task".to_string(), 0).into_inner().1;
        let move_response = context.move_to(root_index.clone());
        assert_eq!(move_response.inner(), &Some("Root task".to_string()));

        assert_eq!(*context.get_current_index().inner(), vec![0]);

        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        let move_response = context.move_to(task1_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 1".to_string()));
        assert_eq!(*context.get_current_index().inner(), vec![0, 0]);

        let task2_index = context.add_task("Task 2".to_string(), 0).into_inner().1;
        assert_eq!(*context.get_current_index().inner(), vec![0, 0]);
        let move_response = context.move_to(task2_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 2".to_string()));
        assert_eq!(*context.get_current_index().inner(), vec![0, 0, 0]);
    }

    #[test]
    fn test_get_with_history() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        let root_index = context.add_task("Root task".to_string(), 0).into_inner().1;
        let move_response = context.move_to(root_index.clone());
        assert!(move_response.inner().is_some());

        // Add sibling tasks
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        let _ = context.add_task("Task 2".to_string(), 0);

        // Move to the first task and add a subtask
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string(), 1).into_inner().1;
        assert_eq!(subtask1_index, vec![0, 0, 0]);

        // Test getting history for the subtask
        let move_response = context.move_to(subtask1_index.clone());
        assert!(move_response.inner().is_some());
        let (level, task, task_history) = context.get_current_with_history().unwrap();

        // Verify the level is correct (task has explicit level 1 - Isolation)
        assert_eq!(level.description(), default_levels()[1].description());
        assert_eq!(
            level.abstraction_focus(),
            default_levels()[1].abstraction_focus()
        );

        // Verify the task is correct
        assert_eq!(task.description(), "Subtask 1");

        // Verify the history is correct
        assert_eq!(task_history.len(), 3);
        assert_eq!(task_history[0], "Root task");
        assert_eq!(task_history[1], "Task 1");
    }

    #[test]
    fn test_level_inference() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string(), 0);

        // Root level (empty cursor) should be 0
        assert_eq!(context.get_current_level(), 0);

        // Add a task at root level
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        assert_eq!(context.get_current_level(), 0);

        // Move to task1 (level 1)
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        assert_eq!(context.get_current_level(), 1);

        // Add a subtask to task1
        let subtask1_index = context.add_task("Subtask 1".to_string(), 1).into_inner().1;
        assert_eq!(context.get_current_level(), 1);

        // Move to subtask1 (level 2)
        let move_response = context.move_to(subtask1_index.clone());
        assert!(move_response.inner().is_some());
        assert_eq!(context.get_current_level(), 2);

        // Set level back to 1
        context.set_current_level(1);
        assert_eq!(context.get_current_level(), 1);
        assert_eq!(*context.get_current_index().inner(), task1_index);

        // Set level back to 0 (root)
        context.set_current_level(0);
        assert_eq!(context.get_current_level(), 0);
        assert!(context.get_current_index().inner().is_empty());
    }

    #[test]
    fn test_parse_index() {
        let index = parse_index("0,1,2").unwrap();
        assert_eq!(index, vec![0, 1, 2]);

        let index = parse_index("0").unwrap();
        assert_eq!(index, vec![0]);

        let result = parse_index("a,b,c");
        assert!(result.is_err());
    }

    #[test]
    fn test_change_level() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        // Add a task at the root level
        let task_index = context.add_task("Task 1".to_string(), 0).into_inner().1;

        // Change the level
        let result = context.change_level(task_index.clone(), 0);
        assert!(result.inner().is_ok());

        // Verify the level was changed
        let task = context.get_task(task_index).unwrap();
        assert_eq!(task.level_index(), Some(0));
    }

    #[test]
    fn test_transition_history_logging() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        // Initial state: history should be empty
        assert!(context.history.is_empty());

        // Perform some actions
        let task1_index = context.add_task("Task 1".to_string(), 0).into_inner().1;
        let history_len_after_add1 = context.history.len();
        assert_eq!(
            history_len_after_add1, 1,
            "History should have 1 entry after first add"
        );
        assert_eq!(context.history.back().unwrap().action, "add_task");

        context.move_to(task1_index.clone());
        let history_len_after_move = context.history.len();
        assert_eq!(
            history_len_after_move, 2,
            "History should have 2 entries after move"
        );
        assert_eq!(context.history.back().unwrap().action, "move_to");

        let subtask_index = context.add_task("Subtask 1".to_string(), 1).into_inner().1;
        let history_len_after_add2 = context.history.len();
        assert_eq!(
            history_len_after_add2,
            4, // Adjusted: add_task + uncomplete_parent
            "History should have 4 entries after second add (incl. uncomplete)"
        );
        assert_eq!(context.history.back().unwrap().action, "uncomplete_parent"); // Last action is uncomplete

        context.complete_task(subtask_index, None, false, Some("Test summary".to_string())); // Provide summary
        let history_len_after_complete = context.history.len();
        assert_eq!(
            history_len_after_complete,
            5, // Adjusted: 4 + 1
            "History should have 5 entries after complete"
        );
        assert_eq!(context.history.back().unwrap().action, "complete_task"); // Expect success

        context.change_level(task1_index, 0);
        let history_len_after_change_level = context.history.len();
        assert_eq!(
            history_len_after_change_level,
            6, // Adjusted: 5 + 1
            "History should have 6 entries after change_level"
        );
        assert_eq!(context.history.back().unwrap().action, "change_level");

        context.set_current_level(0);
        let history_len_after_set_level = context.history.len();
        assert_eq!(
            history_len_after_set_level,
            7, // Adjusted: 6 + 1
            "History should have 7 entries after set_current_level"
        );
        assert_eq!(context.history.back().unwrap().action, "set_current_level");

        // Test buffer limit
        for i in 0..MAX_HISTORY_SIZE + 5 {
            context.add_task(format!("Filler task {}", i), 0);
        }
        assert_eq!(
            context.history.len(),
            MAX_HISTORY_SIZE,
            "History should not exceed MAX_HISTORY_SIZE"
        );
    }

    #[test]
    fn test_core_with_context() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let context = Context::new(plan);
        let core = Core::new(context);

        // Use add_task through Core
        let response = core.add_task("Task through Core".to_string(), 0);
        let task_index = response.into_inner().1;

        // Move to the task
        let move_response = core.move_to(task_index.clone());
        assert_eq!(
            move_response.inner(),
            &Some("Task through Core".to_string())
        );

        // Complete the task (provide summary or force)
        assert!(*core
            .complete_task(
                task_index.clone(),
                None,
                false,
                Some("Test summary".to_string())
            )
            .inner()); // Provide summary

        // Verify task is completed via Current
        let current_response = core.current();
        assert!(current_response
            .inner()
            .as_ref()
            .unwrap()
            .task
            .is_completed());
    }

    #[test]
    fn test_task_accessors() {
        let task = Task::new("Test Task".to_string());

        assert_eq!(task.description(), "Test Task");
        assert!(!task.is_completed());
        assert!(task.subtasks().is_empty());
        assert_eq!(task.level_index(), None);
    }

    #[test]
    fn test_plan_accessors() {
        let plan = Plan::new(default_levels());

        assert_eq!(plan.levels().len(), 4);
        assert_eq!(plan.level_count(), 4);
    }

    #[test]
    fn test_lease_system() {
        let plan = Plan::new(default_levels());
        let context = Context::new(plan);
        let core = Core::new(context); // Use Core for interacting

        // Add a task
        let task_index = core
            .add_task("Task with Lease".to_string(), 0)
            .into_inner()
            .1;

        // --- IMPORTANT: Move to the task before generating lease and completing ---
        core.move_to(task_index.clone());

        // Generate a lease FOR THE CURRENT TASK (after moving)
        let lease_response = core.generate_lease(task_index.clone());
        let generated_lease = lease_response.inner().0.value(); // Access the Lease (index 0) in the tuple

        // --- Test Completion --- //

        // 1. Fail completion without lease
        let complete_fail_no_lease = core.complete_task(task_index.clone(), None, false, None);
        assert!(
            !complete_fail_no_lease.inner(),
            "Completion should fail without lease"
        );

        // 2. Fail completion with wrong lease
        let wrong_lease = generated_lease.wrapping_add(1); // Ensure different lease
        let complete_fail_wrong_lease =
            core.complete_task(task_index.clone(), Some(wrong_lease), false, None);
        assert!(
            !complete_fail_wrong_lease.inner(),
            "Completion should fail with wrong lease"
        );

        // 3. Succeed completion with correct lease and summary
        let summary1 = "Lease success".to_string();
        let complete_success = core.complete_task(
            task_index.clone(),
            Some(generated_lease),
            false,
            Some(summary1.clone()),
        );
        assert!(
            complete_success.inner(),
            "Completion should succeed with correct lease"
        );

        // Verify task is completed and summary stored
        let task = core
            .get_plan()
            .into_inner()
            .get_with_history(task_index.clone())
            .unwrap()
            .1;
        assert!(
            task.is_completed(),
            "Task should be completed after successful lease completion"
        );
        assert_eq!(task.completion_summary(), Some(&summary1));

        // --- Test Force Completion --- //

        // Add another task
        let task2_index = core
            .add_task("Task Force Lease".to_string(), 0)
            .into_inner()
            .1;

        // --- IMPORTANT: Move to the second task before generating lease and completing ---
        core.move_to(task2_index.clone());

        let lease2_response = core.generate_lease(task2_index.clone());
        let _generated_lease2 = lease2_response.inner().0.value(); // Access the Lease (index 0) in the tuple

        // 4. Succeed completion with --force, even without lease (provide optional summary)
        let summary2 = "Forced completion summary".to_string();
        let complete_force_no_lease =
            core.complete_task(task2_index.clone(), None, true, Some(summary2.clone())); // Clone here as well
        assert!(
            complete_force_no_lease.inner(),
            "Completion should succeed with force, no lease"
        );
        let task2 = core
            .get_plan()
            .into_inner()
            .get_with_history(task2_index.clone())
            .unwrap()
            .1;
        assert!(task2.is_completed(), "Task 2 should be completed via force");
        assert_eq!(task2.completion_summary(), Some(&summary2)); // Verify summary stored even with force

        // --- Test Lease Removal --- //
        // Check if lease was removed after completion (test this indirectly via Context)
        let context_lock = core.inner.lock().unwrap();
        assert!(
            !context_lock.leases.contains_key(&task_index),
            "Lease should be removed after successful completion"
        );
        assert!(
            !context_lock.leases.contains_key(&task2_index),
            "Lease should be removed after forced completion"
        );
    }

    #[test]
    fn test_task_completion_with_summary() {
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        // Add a task
        let task_index = context
            .add_task("Task with Summary".to_string(), 0)
            .into_inner()
            .1;
        context.move_to(task_index.clone()); // Move cursor to the task

        // 1. Fail completion without force and without summary
        let result_no_summary = context.complete_task(task_index.clone(), None, false, None);
        assert!(result_no_summary.inner().is_err());
        assert!(result_no_summary
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("requires a summary"));

        // Verify task is still not completed and has no summary
        let task = context.get_task(task_index.clone()).unwrap();
        assert!(!task.is_completed());
        assert!(task.completion_summary().is_none());

        // 2. Succeed completion without force, *with* summary
        let summary_text = "Task completed successfully".to_string();
        let result_with_summary =
            context.complete_task(task_index.clone(), None, false, Some(summary_text.clone()));
        assert!(result_with_summary.inner().is_ok());
        assert!(*result_with_summary.inner().as_ref().unwrap());

        // Verify task is completed and summary is stored
        let task = context.get_task(task_index.clone()).unwrap();
        assert!(task.is_completed());
        assert_eq!(task.completion_summary(), Some(&summary_text));

        // Add another task for force testing
        let force_task_index = context.add_task("Force Task".to_string(), 0).into_inner().1;
        context.move_to(force_task_index.clone());

        // 3. Succeed completion *with* force, without summary
        let result_force_no_summary =
            context.complete_task(force_task_index.clone(), None, true, None);
        assert!(result_force_no_summary.inner().is_ok());
        assert!(*result_force_no_summary.inner().as_ref().unwrap());

        // Verify task is completed and summary is None
        let task = context.get_task(force_task_index.clone()).unwrap();
        assert!(task.is_completed());
        assert!(task.completion_summary().is_none());

        // Add a third task for force + summary testing
        let force_summary_task_index = context
            .add_task("Force Summary Task".to_string(), 0)
            .into_inner()
            .1;
        context.move_to(force_summary_task_index.clone());
        let force_summary_text = "Forced completion with summary".to_string();

        // 4. Succeed completion *with* force, *with* summary
        let result_force_with_summary = context.complete_task(
            force_summary_task_index.clone(),
            None,
            true,
            Some(force_summary_text.clone()),
        );
        assert!(result_force_with_summary.inner().is_ok());
        assert!(*result_force_with_summary.inner().as_ref().unwrap());

        // Verify task is completed and summary *is* stored
        let task = context.get_task(force_summary_task_index.clone()).unwrap();
        assert!(task.is_completed());
        assert_eq!(task.completion_summary(), Some(&force_summary_text));
    }

    #[test]
    fn test_task_removal() {
        // Setup: Create a context with a few nested tasks
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        let task0_idx = context.add_task("Task 0".to_string(), 0).into_inner().1; // [0]
        context.move_to(task0_idx.clone());
        let _task0_0_idx = context.add_task("Task 0.0".to_string(), 1).into_inner().1; // [0, 0]
        let task0_1_idx = context.add_task("Task 0.1".to_string(), 1).into_inner().1; // [0, 1]
        context.move_to(task0_1_idx.clone());
        let task0_1_0_idx = context.add_task("Task 0.1.0".to_string(), 2).into_inner().1; // [0, 1, 0]

        // Generate a lease for the task we intend to remove
        let _ = context.generate_lease(task0_1_idx.clone());
        assert!(context.leases.contains_key(&task0_1_idx));

        // Move cursor to a child of the task to be removed
        context.move_to(task0_1_0_idx.clone());
        assert_eq!(*context.get_current_index().inner(), task0_1_0_idx);

        // 1. Test successful removal
        let remove_result = context.remove_task(task0_1_idx.clone());
        assert!(remove_result.inner().is_ok(), "Removal should succeed");
        let removed_task = remove_result.into_inner().unwrap();
        assert_eq!(removed_task.description(), "Task 0.1");

        // Verify task is gone from parent
        let parent_task = context.get_task(task0_idx.clone()).unwrap();
        assert_eq!(parent_task.subtasks().len(), 1); // Only Task 0.0 should remain
        assert_eq!(parent_task.subtasks()[0].description(), "Task 0.0");

        // Verify cursor was adjusted to parent
        assert_eq!(
            *context.get_current_index().inner(),
            task0_idx,
            "Cursor should move to parent after removal"
        );

        // Verify lease was removed
        assert!(
            !context.leases.contains_key(&task0_1_idx),
            "Lease should be removed after task removal"
        );

        // 2. Test removing root (empty index)
        let remove_root_result = context.remove_task(vec![]);
        assert!(remove_root_result.inner().is_err());
        assert!(remove_root_result
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("Cannot remove the root task"));

        // 3. Test removing non-existent index (invalid parent)
        let invalid_parent_idx = vec![5, 0];
        let remove_invalid_parent_result = context.remove_task(invalid_parent_idx);
        assert!(remove_invalid_parent_result.inner().is_err());
        assert!(remove_invalid_parent_result
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("Parent task at index [5] not found"));

        // 4. Test removing non-existent index (invalid child index)
        let invalid_child_idx = vec![0, 5]; // Parent [0] exists, child 5 does not
        let remove_invalid_child_result = context.remove_task(invalid_child_idx);
        assert!(remove_invalid_child_result.inner().is_err());
        assert!(remove_invalid_child_result
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("Child index 5 out of bounds"));
    }

    #[test]
    fn test_task_uncompletion() {
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        // Add a task
        let task_index = context
            .add_task("Task to Uncomplete".to_string(), 0)
            .into_inner()
            .1;
        let task_idx_clone = task_index.clone(); // Clone for later use

        // Complete the task with a summary
        let summary = "Task done".to_string();
        let complete_res =
            context.complete_task(task_index.clone(), None, false, Some(summary.clone()));
        assert!(complete_res.inner().is_ok());
        assert!(*complete_res.inner().as_ref().unwrap());

        // Verify it's completed and has the summary
        let task = context.get_task(task_index.clone()).unwrap();
        assert!(task.is_completed());
        assert_eq!(task.completion_summary(), Some(&summary));

        // Uncomplete the task
        let uncomplete_res = context.uncomplete_task(task_index.clone());
        assert!(
            uncomplete_res.inner().is_ok(),
            "Uncompletion should succeed"
        );
        assert!(
            *uncomplete_res.inner().as_ref().unwrap(),
            "Uncompletion result should be true"
        );

        // Verify it's no longer completed and summary is cleared
        let task = context.get_task(task_index.clone()).unwrap();
        assert!(
            !task.is_completed(),
            "Task should be incomplete after uncompleting"
        );
        assert!(
            task.completion_summary().is_none(),
            "Summary should be cleared after uncompleting"
        );

        // Try uncompleting the already incomplete task
        let uncomplete_again_res = context.uncomplete_task(task_index.clone());
        assert!(
            uncomplete_again_res.inner().is_err(),
            "Uncompleting an incomplete task should fail"
        );
        assert!(uncomplete_again_res
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("already incomplete"));

        // Try uncompleting a non-existent task
        let non_existent_index = vec![99];
        let uncomplete_non_existent_res = context.uncomplete_task(non_existent_index);
        assert!(
            uncomplete_non_existent_res.inner().is_err(),
            "Uncompleting non-existent task should fail"
        );
        assert!(uncomplete_non_existent_res
            .inner()
            .as_ref()
            .err()
            .unwrap()
            .contains("not found"));

        // Verify the original task state hasn't changed unexpectedly
        let task_final_check = context.get_task(task_idx_clone).unwrap();
        assert!(!task_final_check.is_completed());
        assert!(task_final_check.completion_summary().is_none());
    }

    #[test]
    fn test_add_task_uncompletes_parents() {
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);

        // Add Task A
        let idx_a = context.add_task("Task A".to_string(), 0).into_inner().1;
        context.move_to(idx_a.clone());

        // Add Task B under A
        let idx_b = context.add_task("Task B".to_string(), 1).into_inner().1;
        context.move_to(idx_b.clone());

        // Add Task C under B
        let idx_c = context.add_task("Task C".to_string(), 2).into_inner().1;

        // Complete Task A (should complete A, B, C)
        let complete_res =
            context.complete_task(idx_a.clone(), None, false, Some("Done A".to_string()));
        assert!(complete_res.inner().is_ok() && *complete_res.inner().as_ref().unwrap());

        // Verify A, B, C are completed
        assert!(context.get_task(idx_a.clone()).unwrap().is_completed());
        assert!(context.get_task(idx_b.clone()).unwrap().is_completed());
        assert!(context.get_task(idx_c.clone()).unwrap().is_completed());

        // Move to Task C
        context.move_to(idx_c.clone());

        // Add Task D under C
        let idx_d = context.add_task("Task D".to_string(), 3).into_inner().1;

        // Verify A, B, C are now INCOMPLETE
        assert!(
            !context.get_task(idx_a.clone()).unwrap().is_completed(),
            "Task A should be incomplete after adding D"
        );
        assert!(
            !context.get_task(idx_b.clone()).unwrap().is_completed(),
            "Task B should be incomplete after adding D"
        );
        assert!(
            !context.get_task(idx_c.clone()).unwrap().is_completed(),
            "Task C should be incomplete after adding D"
        );

        // Verify D is incomplete
        assert!(
            !context.get_task(idx_d.clone()).unwrap().is_completed(),
            "Task D should be initially incomplete"
        );
    }

    #[test]
    fn test_generate_lease_suggestions() {
        let plan = Plan::new(default_levels());
        let core = Core::new(Context::new(plan));

        // 1. Test lease for ROOT task (index []) - should have suggestions
        let root_lease_response = core.generate_lease(vec![]);
        let (_root_lease, root_suggestions) = root_lease_response.inner();
        assert!(
            !root_suggestions.is_empty(),
            "Suggestions should be present for root task lease"
        );

        // 2. Add a non-root task
        let task1_index = core.add_task("Task 1".to_string(), 0).into_inner().1;

        // 3. Test lease for NON-ROOT task - should NOT have suggestions
        let task1_lease_response = core.generate_lease(task1_index);
        let (_task1_lease, task1_suggestions) = task1_lease_response.inner();
        assert!(
            task1_suggestions.is_empty(),
            "Suggestions should be empty for non-root task lease"
        );
    }
}
