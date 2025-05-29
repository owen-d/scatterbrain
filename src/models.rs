//! Core models for the scatterbrain library
//!
//! This module contains the core data types and business logic for the scatterbrain tool.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::Rng;
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
    notes: Option<String>,
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
            notes: None,
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
            notes: None,
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

    /// Sets the notes for this task
    pub(crate) fn set_notes(&mut self, notes: Option<String>) {
        self.notes = notes;
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

    /// Gets the notes if they exist
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
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
    pub goal: Option<String>,
    pub notes: Option<String>,
}

impl Plan {
    /// Creates a new plan with the given levels and an optional goal
    pub fn new(levels: Vec<Level>, goal: Option<String>, notes: Option<String>) -> Self {
        Self {
            root: Task::new("root".to_string()),
            levels,
            goal,
            notes,
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
        let plan = Plan::new(default_levels(), None, None); // Pass None for goal here
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
        let lease_val = self.rng.gen::<u8>();
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
        notes: Option<String>,
    ) -> PlanResponse<(Task, Index)> {
        self.log_transition(
            "add_task".to_string(),
            Some(format!(
                "Adding task: '{}' with level {} (notes: {}) to parent index {:?}",
                description,
                level_index,
                notes.is_some(),
                self.cursor
            )),
        );

        // Use Task::with_level and set notes
        let mut task = Task::with_level(description, level_index);
        task.set_notes(notes);

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
                            "Uncompleted parent task at index: {ancestor_index:?}"
                        )),
                    );
                } else {
                    // Should not happen if indices are correct, but log defensively
                    self.log_transition(
                        "uncomplete_parent_failed".to_string(),
                        Some(format!(
                            "Failed to find ancestor task at index: {ancestor_index:?}"
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
            Some(format!("Attempting to remove task at index: {index:?}")),
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
                let err_msg = format!("Parent task at index {parent_index:?} not found.");
                self.log_transition("remove_task_failed".to_string(), Some(err_msg.clone()));
                return PlanResponse::new(Err(err_msg), self.distilled_context().context());
            }
        };

        // Validate the child index and remove the task
        if *child_idx >= parent_task.subtasks.len() {
            let err_msg =
                format!("Child index {child_idx} out of bounds for parent {parent_index:?}");
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
            Some(format!("Moving cursor to index: {index:?}")),
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
                    let msg = format!("Task at index {index:?} requires a lease to be completed.");
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
            let msg =
                format!("Task at index {index:?} requires a summary for non-forced completion.");
            self.log_transition("complete_task_failed".to_string(), Some(msg.clone()));
            return PlanResponse::new(Err(msg), self.distilled_context().context());
        }

        self.log_transition(
            "complete_task".to_string(),
            Some(format!(
                "Completing task at index: {index:?} (force: {force})"
            )),
        );

        // First, get a clone of the task for generating suggestions
        let task_clone_opt = self.get_task(index.clone()).cloned();

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
                "Changing level for task {index:?} to {level_index}"
            )),
        );

        // Validate: the level must exist
        if level_index >= self.plan.level_count() {
            return PlanResponse::new(
                Err(format!("Level index {level_index} is out of bounds")),
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
                            "Child task cannot have a higher abstraction level ({level_index}) than its parent ({parent_level})"
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
                        "Cannot set level to {max_level} because a child task has a higher level ({subtask_level})"
                    ));
                }

                // Recursively check this subtask's children
                check_children(subtask, depth + 1, max_level)?
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
                            "Uncompleted task \"{task_description}\" at index {index_str}"
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
            Some(format!("Setting current level to: {level}")),
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

    /// Builds a task tree focusing on the path to the current cursor.
    /// Shows all nodes on the path, and recursively shows all children for nodes on the path.
    fn build_task_tree(&self) -> Vec<TaskTreeNode> {
        self.get_subtasks(Vec::new()) // Get top-level tasks
            .into_iter()
            .map(|(idx, task)| {
                // Determine if the current task is this task or one of its descendants
                let is_on_path = self.cursor.starts_with(&idx);

                TaskTreeNode {
                    description: task.description().to_string(),
                    index: idx.clone(),
                    completed: task.is_completed(),
                    is_current: idx == self.cursor,
                    completion_summary: task.completion_summary().cloned(),
                    notes: task.notes().map(|s| s.to_string()),
                    children: if is_on_path {
                        // If on the path, recursively build the subtree below this node,
                        // but only expanding children that are ALSO on the path.
                        self.build_path_focused_subtree(&idx)
                    } else {
                        // If not on the path, don't include children
                        Vec::new()
                    },
                }
            })
            .collect()
    }

    /// Helper method to recursively build the subtree for nodes on the path to the cursor.
    fn build_path_focused_subtree(&self, index: &Index) -> Vec<TaskTreeNode> {
        self.get_subtasks(index.clone())
            .into_iter()
            .map(|(child_idx, child_task)| {
                // Determine if this child is also on the path to the cursor
                let is_child_on_path = self.cursor.starts_with(&child_idx);
                TaskTreeNode {
                    description: child_task.description().to_string(),
                    index: child_idx.clone(),
                    completed: child_task.is_completed(),
                    is_current: child_idx == self.cursor,
                    completion_summary: child_task.completion_summary().cloned(),
                    notes: child_task.notes().map(|s| s.to_string()),
                    // Only recurse if the child itself is on the path
                    children: if is_child_on_path {
                        self.build_path_focused_subtree(&child_idx)
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

        // Get the plan's goal and notes
        let goal = self.plan.goal.clone();
        let plan_notes = self.plan.notes.clone(); // Clone plan notes

        // Create the distilled context with all components using the builder pattern
        let distilled = DistilledContext::builder()
            .usage_summary(usage_summary)
            .task_tree(task_tree)
            .current_task(current_task_opt)
            .current_level(current_level)
            .levels(levels)
            .transition_history(self.history.iter().cloned().collect())
            .goal(goal)
            .plan_notes(plan_notes)
            .build();

        PlanResponse::new((), distilled)
    }

    /// Sets the notes for the task at the given index.
    pub fn set_task_notes(
        &mut self,
        index: Index,
        notes: String,
    ) -> PlanResponse<Result<(), String>> {
        self.log_transition(
            "set_task_notes".to_string(),
            Some(format!("Setting notes for task at index: {index:?}")),
        );

        let result = match self.get_task_mut(index.clone()) {
            Some(task) => {
                task.set_notes(Some(notes));
                Ok(())
            }
            None => Err(format!("Task not found at index: {index:?}")),
        };

        PlanResponse::new(result, self.distilled_context().context())
    }

    /// Gets the notes for the task at the given index.
    pub fn get_task_notes(&self, index: Index) -> PlanResponse<Result<Option<String>, String>> {
        // Log transition *before* getting distilled context if possible,
        // but here we need the result first to log accurately.
        let result = match self.get_task(index.clone()) {
            Some(task) => Ok(task.notes().map(|s| s.to_string())),
            None => Err(format!("Task not found at index: {index:?}")),
        };

        // Log transition after getting the result -- REMOVED because get_task_notes is &self
        // self.log_transition(
        //     "get_task_notes".to_string(),
        //     Some(format!("Getting notes for task at index: {:?}", index))
        // );

        PlanResponse::new(result, self.distilled_context().context())
    }

    /// Deletes the notes for the task at the given index.
    pub fn delete_task_notes(&mut self, index: Index) -> PlanResponse<Result<(), String>> {
        self.log_transition(
            "delete_task_notes".to_string(),
            Some(format!("Deleting notes for task at index: {index:?}")),
        );

        let result = match self.get_task_mut(index.clone()) {
            Some(task) => {
                task.set_notes(None);
                Ok(())
            }
            None => Err(format!("Task not found at index: {index:?}")),
        };

        PlanResponse::new(result, self.distilled_context().context())
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
    /// The original goal of the plan, if any.
    pub goal: Option<String>,
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
    /// Optional notes associated with the plan.
    pub plan_notes: Option<String>,
}

impl DistilledContext {
    /// Creates a new builder for DistilledContext
    pub fn builder() -> DistilledContextBuilder {
        DistilledContextBuilder::new()
    }
}

/// Builder for DistilledContext to avoid too many constructor arguments
pub struct DistilledContextBuilder {
    usage_summary: Option<String>,
    task_tree: Option<Vec<TaskTreeNode>>,
    current_task: Option<Task>,
    current_level: Option<Level>,
    levels: Option<Vec<Level>>,
    transition_history: Option<Vec<TransitionLogEntry>>,
    goal: Option<String>,
    plan_notes: Option<String>,
}

impl DistilledContextBuilder {
    fn new() -> Self {
        Self {
            usage_summary: None,
            task_tree: None,
            current_task: None,
            current_level: None,
            levels: None,
            transition_history: None,
            goal: None,
            plan_notes: None,
        }
    }

    pub fn usage_summary(mut self, usage_summary: String) -> Self {
        self.usage_summary = Some(usage_summary);
        self
    }

    pub fn task_tree(mut self, task_tree: Vec<TaskTreeNode>) -> Self {
        self.task_tree = Some(task_tree);
        self
    }

    pub fn current_task(mut self, current_task: Option<Task>) -> Self {
        self.current_task = current_task;
        self
    }

    pub fn current_level(mut self, current_level: Option<Level>) -> Self {
        self.current_level = current_level;
        self
    }

    pub fn levels(mut self, levels: Vec<Level>) -> Self {
        self.levels = Some(levels);
        self
    }

    pub fn transition_history(mut self, transition_history: Vec<TransitionLogEntry>) -> Self {
        self.transition_history = Some(transition_history);
        self
    }

    pub fn goal(mut self, goal: Option<String>) -> Self {
        self.goal = goal;
        self
    }

    pub fn plan_notes(mut self, plan_notes: Option<String>) -> Self {
        self.plan_notes = plan_notes;
        self
    }

    pub fn build(self) -> DistilledContext {
        DistilledContext {
            usage_summary: self.usage_summary.unwrap_or_default(),
            task_tree: self.task_tree.unwrap_or_default(),
            current_task: self.current_task,
            current_level: self.current_level,
            levels: self.levels.unwrap_or_default(),
            transition_history: self.transition_history.unwrap_or_default(),
            goal: self.goal,
            plan_notes: self.plan_notes,
        }
    }
}

/// A node in the task tree for the distilled context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Optional task notes
    pub notes: Option<String>,
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

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    /// Creates a new Core instance, initializing with a default plan.
    pub fn new() -> Self {
        // Create a broadcast channel for PlanId updates
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
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
        let context = plans.get_mut(id).ok_or(PlanError::PlanNotFound(*id))?;

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
        let context = plans.get(id).ok_or(PlanError::PlanNotFound(*id))?;

        // Apply the function
        let result = f(context);

        Ok(result)
    }

    /// Creates a new plan with the given goal and returns its unique ID (Lease).
    /// Handles potential collisions if a randomly generated u8 ID already exists.
    pub fn create_plan(&self, goal: String, notes: Option<String>) -> Result<PlanId, PlanError> {
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
        // Create a new plan with the provided goal and notes
        let plan = Plan::new(default_levels(), Some(goal), notes);
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
        notes: Option<String>,
    ) -> Result<PlanResponse<(Task, Index)>, PlanError> {
        self.with_plan_context(id, |context| {
            context.add_task(description, level_index, notes)
        })
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
                    eprintln!("Error completing task in plan {id:?}: {e}");
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

    /// Sets the notes for a specific task within a plan.
    pub fn set_task_notes(
        &self,
        id: &PlanId,
        index: Index,
        notes: String,
    ) -> Result<PlanResponse<Result<(), String>>, PlanError> {
        self.with_plan_context(id, |context| context.set_task_notes(index, notes))
    }

    /// Gets the notes for a specific task within a plan.
    /// Note: Logging is omitted in the Context::get_task_notes to keep it immutable.
    pub fn get_task_notes(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<Result<Option<String>, String>>, PlanError> {
        self.with_plan_context_read(id, |context| context.get_task_notes(index))
    }

    /// Deletes the notes for a specific task within a plan.
    pub fn delete_task_notes(
        &self,
        id: &PlanId,
        index: Index,
    ) -> Result<PlanResponse<Result<(), String>>, PlanError> {
        self.with_plan_context(id, |context| context.delete_task_notes(index))
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{Context, Core, Lease, Level, Plan, PlanError, TaskTreeNode}; // Ensure TaskTreeNode is imported
    use pretty_assertions::assert_eq; // Use pretty_assertions for better diffs

    // Helper function to create a basic context for testing build_task_tree
    fn setup_context() -> Context {
        let levels = vec![
            Level::new(
                "L0".to_string(),
                "Level 0 Focus".to_string(),
                vec!["Q0?".to_string()],
                "Guidance 0".to_string(),
            ),
            Level::new(
                "L1".to_string(),
                "Level 1 Focus".to_string(),
                vec!["Q1?".to_string()],
                "Guidance 1".to_string(),
            ),
            Level::new(
                "L2".to_string(),
                "Level 2 Focus".to_string(),
                vec!["Q2?".to_string()],
                "Guidance 2".to_string(),
            ),
        ];
        let plan = Plan::new(levels, Some("Test Goal".to_string()), None);
        Context::new_with_seed(plan, 0) // Use a fixed seed for reproducibility if needed
    }

    #[test]
    fn test_build_task_tree_empty() {
        let context = setup_context();
        let tree = context.build_task_tree();
        assert!(tree.is_empty(), "Tree should be empty for a new plan");
    }

    #[test]
    fn test_build_task_tree_single_task() {
        let mut context = setup_context();
        let (_, task_idx) = context.add_task("Task 0".to_string(), 0, None).into_inner(); // Add task at root
        context.move_to(task_idx.clone()).inner(); // Move to the task

        let tree = context.build_task_tree();

        assert_eq!(tree.len(), 1, "Tree should have one root node");
        assert_eq!(
            tree[0],
            TaskTreeNode {
                description: "Task 0".to_string(),
                index: vec![0],
                completed: false,
                is_current: true,
                completion_summary: None,
                notes: None,
                children: vec![],
            }
        );
    }

    #[test]
    fn test_build_task_tree_nested_tasks_cursor_at_root() {
        let mut context = setup_context();
        context.add_task("Task 0".to_string(), 0, None).into_inner();
        context.move_to(vec![0]).inner(); // Move to Task 0
        context
            .add_task("Task 0.0".to_string(), 1, None)
            .into_inner();
        context.move_to(vec![]).inner(); // Move back to root

        let tree = context.build_task_tree();

        assert_eq!(tree.len(), 1, "Tree should have one root node (Task 0)");
        assert_eq!(tree[0].description, "Task 0");
        assert_eq!(tree[0].index, vec![0]);
        assert_eq!(tree[0].is_current, false); // Cursor is at root
        assert_eq!(
            tree[0].children.len(),
            0,
            "Children of non-path nodes should not be included when cursor is at root"
        );
    }

    #[test]
    fn test_build_task_tree_nested_tasks_cursor_at_parent() {
        let mut context = setup_context();
        let (_, idx0) = context.add_task("Task 0".to_string(), 0, None).into_inner();
        context.move_to(idx0.clone()).inner(); // Move to Task 0
        let (_, idx00) = context
            .add_task("Task 0.0".to_string(), 1, None)
            .into_inner();
        context
            .add_task("Task 0.1".to_string(), 1, None)
            .into_inner();
        context.move_to(idx0.clone()).inner(); // Stay at Task 0

        let tree = context.build_task_tree();

        assert_eq!(tree.len(), 1, "Tree should have one root node (Task 0)");
        assert_eq!(tree[0].description, "Task 0");
        assert_eq!(tree[0].index, idx0);
        assert_eq!(tree[0].is_current, true); // Cursor is at Task 0
        assert_eq!(
            tree[0].children.len(),
            2,
            "Children of current node should be included"
        );

        assert_eq!(tree[0].children[0].description, "Task 0.0");
        assert_eq!(tree[0].children[0].index, idx00);
        assert_eq!(tree[0].children[0].is_current, false);
        assert_eq!(tree[0].children[0].children.len(), 0); // Grandchildren not included unless on path

        assert_eq!(tree[0].children[1].description, "Task 0.1");
        assert_eq!(tree[0].children[1].index, vec![0, 1]);
        assert_eq!(tree[0].children[1].is_current, false);
        assert_eq!(tree[0].children[1].children.len(), 0);
    }

    #[test]
    fn test_build_task_tree_nested_tasks_cursor_at_child() {
        let mut context = setup_context();
        let (_, idx0) = context.add_task("Task 0".to_string(), 0, None).into_inner();
        context.move_to(idx0.clone()).inner(); // Move to Task 0
        let (_, idx00) = context
            .add_task("Task 0.0".to_string(), 1, None)
            .into_inner();
        context.move_to(idx00.clone()).inner(); // Move to Task 0.0
        context
            .add_task("Task 0.0.0".to_string(), 2, None)
            .into_inner(); // Add a child to 0.0
        context.move_to(idx0.clone()).inner(); // Move back to Task 0
        context
            .add_task("Task 0.1".to_string(), 1, None)
            .into_inner(); // Add sibling Task 0.1
        context.move_to(idx00.clone()).inner(); // << Move cursor to Task 0.0

        let tree = context.build_task_tree();

        // Expected tree:
        // Task 0 [ ] (not current)
        //   Task 0.0 [ ] (current) -> Should have children
        //     Task 0.0.0 [ ] (not current)
        //   Task 0.1 [ ] (not current) -> Should NOT have children

        assert_eq!(tree.len(), 1, "Tree should have one root node (Task 0)");
        let node0 = &tree[0];
        assert_eq!(node0.description, "Task 0");
        assert_eq!(node0.index, idx0);
        assert_eq!(node0.is_current, false); // Cursor is at Task 0.0
        assert_eq!(
            node0.children.len(),
            2,
            "Parent of current node should show siblings"
        );

        // Check Task 0.0 (the current task)
        let node00 = &node0.children[0];
        assert_eq!(node00.description, "Task 0.0");
        assert_eq!(node00.index, idx00);
        assert_eq!(node00.is_current, true); // Cursor is here
        assert_eq!(
            node00.children.len(),
            1,
            "Current node should show its children"
        );
        assert_eq!(node00.children[0].description, "Task 0.0.0");
        assert_eq!(node00.children[0].index, vec![0, 0, 0]);
        assert_eq!(node00.children[0].is_current, false);
        assert_eq!(node00.children[0].children.len(), 0); // No children added to 0.0
        assert_eq!(node00.notes, None); // Check notes are None initially

        // Check Task 0.1 (sibling of current)
        let node01 = &node0.children[1];
        assert_eq!(node01.description, "Task 0.1");
        assert_eq!(node01.index, vec![0, 1]);
        assert_eq!(node01.is_current, false);
        assert_eq!(
            node01.children.len(),
            0,
            "Siblings of current node shouldn't show their children"
        );
    }

    #[test]
    fn test_build_task_tree_completed_task() {
        let mut context = setup_context();
        let (_, idx0) = context.add_task("Task 0".to_string(), 0, None).into_inner();
        context.move_to(idx0.clone()).inner(); // Move to Task 0
        context
            .complete_task(idx0.clone(), None, true, Some("Done".to_string()))
            .inner(); // Complete Task 0

        let tree = context.build_task_tree();

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].description, "Task 0");
        assert_eq!(tree[0].index, vec![0]);
        assert_eq!(tree[0].completed, true); // Should be completed
        assert_eq!(tree[0].is_current, true);
        assert_eq!(tree[0].completion_summary, Some("Done".to_string()));
        assert_eq!(tree[0].children.len(), 0);
    }

    #[test]
    fn test_build_task_tree_multiple_roots_cursor_set() {
        let mut context = setup_context();
        let (_, idx0) = context.add_task("Task 0".to_string(), 0, None).into_inner();
        context.move_to(idx0).inner();
        let (_, idx00) = context
            .add_task("Task 0.0".to_string(), 1, None)
            .into_inner();
        context.move_to(vec![]).inner(); // Back to root

        let (_, idx1) = context.add_task("Task 1".to_string(), 0, None).into_inner();
        context.move_to(idx1.clone()).inner(); // Move to Task 1
        context
            .add_task("Task 1.0".to_string(), 1, None)
            .into_inner();

        context.move_to(idx00.clone()).inner(); // << Set cursor to Task 0.0

        let tree = context.build_task_tree();

        // Expected:
        // Task 0 [ ]
        //   Task 0.0 [ ] -> Current
        // Task 1 [ ]

        assert_eq!(tree.len(), 2, "Should show both root tasks");

        let node0 = &tree[0];
        assert_eq!(node0.description, "Task 0");
        assert_eq!(node0.is_current, false);
        assert_eq!(
            node0.children.len(),
            1,
            "Path to current should show children"
        );
        assert_eq!(node0.children[0].description, "Task 0.0");
        assert_eq!(node0.children[0].is_current, true); // Current node
        assert_eq!(node0.children[0].children.len(), 0); // No children added to 0.0

        let node1 = &tree[1];
        assert_eq!(node1.description, "Task 1");
        assert_eq!(node1.is_current, false);
        assert_eq!(
            node1.children.len(),
            0,
            "Nodes not on path to current shouldn't show children"
        );
    }

    #[test]
    fn test_build_task_tree_path_expansion() {
        let mut context = setup_context();

        // Create structure:
        // RootA [0]
        //   ChildA1 [0, 0]
        //     GrandchildA1a [0, 0, 0] <-- CURSOR
        //     GrandchildA1b [0, 0, 1]
        //   ChildA2 [0, 1]
        //     GrandchildA2a [0, 1, 0] // Should NOT be shown in the slim tree
        // RootB [1]
        //   ChildB1 [1, 0]         // Should NOT be shown in the slim tree

        // RootA and children
        let (_, idx_root_a) = context.add_task("RootA".to_string(), 0, None).into_inner();
        context.move_to(idx_root_a.clone()).inner();
        let (_, idx_child_a1) = context
            .add_task("ChildA1".to_string(), 1, None)
            .into_inner();
        let (_, idx_child_a2) = context
            .add_task("ChildA2".to_string(), 1, None)
            .into_inner();

        // Grandchildren of ChildA1
        context.move_to(idx_child_a1.clone()).inner();
        let (_, idx_grandchild_a1a) = context
            .add_task("GrandchildA1a".to_string(), 2, None)
            .into_inner();
        context
            .add_task("GrandchildA1b".to_string(), 2, None)
            .into_inner();

        // Grandchildren of ChildA2
        context.move_to(idx_child_a2.clone()).inner();
        context
            .add_task("GrandchildA2a".to_string(), 2, None)
            .into_inner();

        // RootB and children
        context.move_to(vec![]).inner(); // Back to root
        let (_, idx_root_b) = context.add_task("RootB".to_string(), 0, None).into_inner();
        context.move_to(idx_root_b.clone()).inner();
        context
            .add_task("ChildB1".to_string(), 1, None)
            .into_inner();

        // << Set cursor to GrandchildA1a >>
        context.move_to(idx_grandchild_a1a.clone()).inner();

        // Build the tree
        let tree = context.build_task_tree();

        // --- Assertions ---

        // Root level
        assert_eq!(tree.len(), 2, "Should have RootA and RootB");
        let node_root_a = &tree[0];
        let node_root_b = &tree[1];
        assert_eq!(node_root_a.description, "RootA");
        assert_eq!(node_root_b.description, "RootB");
        assert!(!node_root_a.is_current);
        assert!(!node_root_b.is_current);

        // RootB is not on the path, should have no children shown
        assert_eq!(
            node_root_b.children.len(),
            0,
            "RootB children should not be expanded"
        );

        // RootA is on the path, check its children (ChildA1, ChildA2)
        assert_eq!(
            node_root_a.children.len(),
            2,
            "RootA should have 2 children"
        );
        let node_child_a1 = &node_root_a.children[0];
        let node_child_a2 = &node_root_a.children[1];
        assert_eq!(node_child_a1.description, "ChildA1");
        assert_eq!(node_child_a2.description, "ChildA2");
        assert!(!node_child_a1.is_current);
        assert!(!node_child_a2.is_current);

        // ChildA2 is not on the path, should have no children shown
        assert_eq!(
            node_child_a2.children.len(),
            0,
            "ChildA2 children should not be expanded"
        );

        // ChildA1 is on the path, check its children (GrandchildA1a, GrandchildA1b)
        assert_eq!(
            node_child_a1.children.len(),
            2,
            "ChildA1 should have 2 children"
        );
        let node_grandchild_a1a = &node_child_a1.children[0];
        let node_grandchild_a1b = &node_child_a1.children[1];
        assert_eq!(node_grandchild_a1a.description, "GrandchildA1a");
        assert_eq!(node_grandchild_a1b.description, "GrandchildA1b");

        // GrandchildA1a is the current node
        assert!(node_grandchild_a1a.is_current);
        assert!(!node_grandchild_a1b.is_current);

        // Grandchildren are leaf nodes in this path, should have no children shown
        assert_eq!(
            node_grandchild_a1a.children.len(),
            0,
            "GrandchildA1a children should be empty"
        );
        assert_eq!(
            node_grandchild_a1b.children.len(),
            0,
            "GrandchildA1b children should be empty"
        );
        assert_eq!(node_grandchild_a1a.notes, None); // Check notes initially
        assert_eq!(node_grandchild_a1b.notes, None); // Check notes initially
    }

    #[test]
    fn test_core_notes_crud() {
        let core = Core::new();
        let plan_id = core.create_plan("Test Plan".to_string(), None).unwrap();

        // 1. Add a task (initially no notes)
        let (_, task_index) = core
            .add_task(&plan_id, "Task with notes".to_string(), 0, None)
            .unwrap()
            .into_inner();
        assert_eq!(task_index, vec![0]);

        // 2. Get notes (should be None)
        let notes_response = core.get_task_notes(&plan_id, task_index.clone()).unwrap();
        assert_eq!(notes_response.into_inner().unwrap(), None);

        // 3. Set notes
        let set_response = core
            .set_task_notes(
                &plan_id,
                task_index.clone(),
                "These are my notes".to_string(),
            )
            .unwrap();
        assert!(set_response.into_inner().is_ok());

        // 4. Get notes (should have value)
        let notes_response = core.get_task_notes(&plan_id, task_index.clone()).unwrap();
        assert_eq!(
            notes_response.into_inner().unwrap(),
            Some("These are my notes".to_string())
        );

        // 5. Verify notes in TaskTreeNode within distilled_context
        let distilled_resp = core.distilled_context(&plan_id).unwrap();
        let distilled = distilled_resp.context();
        assert_eq!(distilled.task_tree.len(), 1);
        assert_eq!(
            distilled.task_tree[0].notes,
            Some("These are my notes".to_string())
        );

        // 6. Delete notes
        let delete_response = core
            .delete_task_notes(&plan_id, task_index.clone())
            .unwrap();
        assert!(delete_response.into_inner().is_ok());

        // 7. Get notes (should be None again)
        let notes_response = core.get_task_notes(&plan_id, task_index.clone()).unwrap();
        assert_eq!(notes_response.into_inner().unwrap(), None);

        // 8. Add a task *with* notes initially
        let (_, task_index_2) = core
            .add_task(
                &plan_id,
                "Task initially with notes".to_string(),
                0,
                Some("Initial notes".to_string()),
            )
            .unwrap()
            .into_inner();
        assert_eq!(task_index_2, vec![1]);
        let notes_response_2 = core.get_task_notes(&plan_id, task_index_2.clone()).unwrap();
        assert_eq!(
            notes_response_2.into_inner().unwrap(),
            Some("Initial notes".to_string())
        );

        // Test error cases
        let bad_index = vec![99];
        let get_err = core
            .get_task_notes(&plan_id, bad_index.clone())
            .unwrap()
            .into_inner();
        assert!(get_err.is_err());
        assert!(get_err.unwrap_err().contains("Task not found"));

        let set_err = core
            .set_task_notes(&plan_id, bad_index.clone(), "fail".to_string())
            .unwrap()
            .into_inner();
        assert!(set_err.is_err());
        assert!(set_err.unwrap_err().contains("Task not found"));

        let delete_err = core
            .delete_task_notes(&plan_id, bad_index.clone())
            .unwrap()
            .into_inner();
        assert!(delete_err.is_err());
        assert!(delete_err.unwrap_err().contains("Task not found"));

        // Test PlanNotFound error
        let bad_plan_id = Lease::new(99); // Assuming 99 is unlikely to be generated
        let get_plan_err = core.get_task_notes(&bad_plan_id, task_index.clone());
        assert!(matches!(get_plan_err, Err(PlanError::PlanNotFound(_))));
    }

    // ... existing tests ...
}
