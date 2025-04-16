//! Core models for the scatterbrain library
//!
//! This module contains the core data types and business logic for the scatterbrain tool.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Re-export levels from the levels module
pub use crate::levels::{default_levels, Level};

/// Represents a task in the LLM's work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub completed: bool,
    pub subtasks: Vec<Task>,
    pub level_index: Option<usize>,
}

impl Task {
    /// Creates a new task with the given level and description
    pub fn new(description: String) -> Self {
        Self {
            description,
            completed: false,
            subtasks: Vec::new(),
            level_index: None,
        }
    }

    /// Creates a new task with a specific level index
    pub fn with_level(description: String, level_index: usize) -> Self {
        Self {
            description,
            completed: false,
            subtasks: Vec::new(),
            level_index: Some(level_index),
        }
    }

    /// Adds a subtask to this task
    pub fn add_subtask(&mut self, subtask: Task) {
        self.subtasks.push(subtask);
    }

    /// Marks this task as completed
    pub fn complete(&mut self) {
        self.completed = true;

        // Recursively complete all subtasks
        for subtask in &mut self.subtasks {
            subtask.complete();
        }
    }

    /// Returns true if this task and all its subtasks are completed
    pub fn is_fully_completed(&self) -> bool {
        self.completed && self.subtasks.iter().all(|t| t.is_fully_completed())
    }

    /// Sets the level index for this task
    pub fn set_level(&mut self, level_index: usize) {
        self.level_index = Some(level_index);
    }

    /// Gets the effective level index (explicit or derived from position)
    pub fn get_effective_level(&self, depth: usize) -> usize {
        self.level_index.unwrap_or(depth)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Plan {
    pub root: Task,
    pub levels: Vec<Level>,
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
    pub fn get_with_history(&self, index: Index) -> Option<(Level, Task, Vec<String>)> {
        let mut current = &self.root;
        let mut history = Vec::new();

        for &i in &index {
            if i >= current.subtasks.len() {
                return None;
            }
            current = &current.subtasks[i];

            // Only add the description after descending (to avoid the implicit root)
            // and only if there are more levels to descend into (to avoid the final leaf which is included in full)
            if i < index.len() - 1 {
                history.push(current.description.clone());
            }
        }

        // Check if index is empty to avoid subtraction overflow
        if index.is_empty() {
            return None;
        }

        // Use the task's explicit level_index if set, otherwise fallback to position-based level
        let level_idx = current.level_index.unwrap_or(index.len() - 1);
        self.levels
            .get(level_idx)
            .cloned()
            .map(|level| (level, current.clone(), history))
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

/// Context for managing the planning process
pub struct Context {
    plan: Plan,
    cursor: Index,
}

impl Context {
    /// Creates a new context with the given plan
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            cursor: Vec::new(), // Start at root
        }
    }

    // Task creation and navigation
    pub fn add_task(&mut self, description: String) -> Index {
        let task = Task::new(description);
        let new_index;

        if self.cursor.is_empty() {
            // Adding to root task, special case
            self.plan.root.add_subtask(task);
            new_index = vec![self.plan.root.subtasks.len() - 1];
        } else {
            // Navigate to the current task
            let current = self.get_current_task_mut().unwrap();

            // Add the new task
            current.add_subtask(task);
            let task_index = current.subtasks.len() - 1;

            // Create the new index
            let mut new_index = self.cursor.clone();
            new_index.push(task_index);
            return new_index;
        }

        new_index
    }

    pub fn move_to(&mut self, index: Index) -> Option<String> {
        // Validate the index
        if index.is_empty() {
            self.cursor = index;
            return Some("root".to_string());
        }

        // Check if the index is valid
        if let Some(task) = self.get_task(index.clone()) {
            let description = task.description.clone();
            self.cursor = index;
            Some(description)
        } else {
            None
        }
    }

    // Task state management
    pub fn complete_task(&mut self, index: Index) -> bool {
        if let Some(task) = self.get_task_mut(index) {
            task.complete();
            true
        } else {
            false
        }
    }

    // Information retrieval
    pub fn get_task(&self, index: Index) -> Option<&Task> {
        if index.is_empty() {
            return Some(&self.plan.root);
        }

        let mut current = &self.plan.root;
        for &idx in &index {
            if idx >= current.subtasks.len() {
                return None;
            }
            current = &current.subtasks[idx];
        }

        Some(current)
    }

    pub fn get_task_mut(&mut self, index: Index) -> Option<&mut Task> {
        if index.is_empty() {
            return Some(&mut self.plan.root);
        }

        let mut current = &mut self.plan.root;
        for &idx in &index {
            if idx >= current.subtasks.len() {
                return None;
            }
            current = &mut current.subtasks[idx];
        }

        Some(current)
    }

    pub fn get_current_task(&self) -> Option<&Task> {
        self.get_task(self.cursor.clone())
    }

    pub fn get_current_task_mut(&mut self) -> Option<&mut Task> {
        self.get_task_mut(self.cursor.clone())
    }

    pub fn get_current_index(&self) -> &Index {
        &self.cursor
    }

    pub fn get_current_level(&self) -> usize {
        self.cursor.len()
    }

    pub fn set_current_level(&mut self, level: usize) {
        while self.cursor.len() > level {
            self.cursor.pop();
        }
    }

    pub fn get_subtasks(&self, index: Index) -> Vec<(Index, &Task)> {
        if let Some(task) = self.get_task(index.clone()) {
            let mut result = Vec::new();
            for (i, subtask) in task.subtasks.iter().enumerate() {
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
    pub fn get_plan(&self) -> &Plan {
        &self.plan
    }

    pub fn get_plan_mut(&mut self) -> &mut Plan {
        &mut self.plan
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Current {
    pub index: Index,
    pub level: Level,
    pub task: Task,
    pub history: Vec<String>,
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

    pub fn get_plan(&self) -> Option<Plan> {
        match self.inner.lock() {
            Ok(guard) => Some(guard.get_plan().clone()),
            Err(poisoned) => {
                // Recover from a poisoned mutex by getting the guard anyway
                let guard = poisoned.into_inner();
                Some(guard.get_plan().clone())
            }
        }
    }

    pub fn current(&self) -> Option<Current> {
        let context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let index = context.get_current_index();
        if let Some((level, task, history)) = context.plan.get_with_history(index.clone()) {
            Some(Current {
                index: index.clone(),
                level,
                task,
                history,
            })
        } else {
            None
        }
    }

    pub fn add_task(&self, description: String) -> Option<Index> {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let result = context.add_task(description);

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        Some(result)
    }

    pub fn complete_task(&self) -> bool {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let index = context.get_current_index().clone();
        let result = context.complete_task(index.clone());

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        result
    }

    pub fn move_to(&self, index: Index) -> Option<String> {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let result = context.move_to(index.clone());

        // Notify all observers about the state change
        let _ = self.update_tx.send(());

        result
    }

    // Subscribe to state updates
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.update_tx.subscribe()
    }

    /// Changes the level of a task at the given index
    pub fn change_level(&self, index: Index, level_index: usize) -> Result<(), String> {
        let mut context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Validate: the level must exist
        if level_index >= context.plan.levels.len() {
            return Err(format!("Level index {} is out of bounds", level_index));
        }

        // Validate parent-child level relationship
        if !index.is_empty() {
            // This isn't the root task, so check parent level
            let parent_index = index[0..index.len() - 1].to_vec();
            if let Some(parent) = context.get_task(parent_index.clone()) {
                let parent_level = parent.level_index.unwrap_or(parent_index.len());
                if level_index > parent_level {
                    return Err(format!(
                        "Child task cannot have a higher abstraction level ({}) than its parent ({})",
                        level_index, parent_level
                    ));
                }
            }
        }

        // Define a recursive function to check all child levels
        fn check_children(task: &Task, depth: usize, max_level: usize) -> Result<(), String> {
            for subtask in &task.subtasks {
                let subtask_level = subtask.level_index.unwrap_or(depth + 1);
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
        if let Some(task) = context.get_task(index.clone()) {
            if let Err(e) = check_children(task, index.len(), level_index) {
                return Err(e);
            }
        }

        // Apply the change
        if let Some(task) = context.get_task_mut(index) {
            // Set the level
            task.set_level(level_index);

            // Notify observers about the state change
            let _ = self.update_tx.send(());

            Ok(())
        } else {
            Err("Task not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::ordering_level;

    #[test]
    fn test_task_creation_and_navigation() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        assert_eq!(context.get_current_index(), &Vec::<usize>::new());
        // Add a task at the root level
        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(task1_index, vec![0]);

        // Add another task at the root level
        let task2_index = context.add_task("Task 2".to_string());
        assert_eq!(task2_index, vec![1]);

        // Move to the first task
        assert_eq!(
            context.move_to(task1_index.clone()),
            Some("Task 1".to_string())
        );

        // Add a subtask to the first task
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(subtask1_index, vec![0, 0]);

        // Move to the second task
        assert_eq!(
            context.move_to(task2_index.clone()),
            Some("Task 2".to_string())
        );
        assert_eq!(context.get_current_index(), &vec![1]);

        // Move to subtask 1
        assert_eq!(
            context.move_to(subtask1_index.clone()),
            Some("Subtask 1".to_string())
        );
        assert_eq!(context.get_current_index(), &vec![0, 0]);
    }

    #[test]
    fn test_task_completion() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string());
        let task2_index = context.add_task("Task 2".to_string());

        // Complete a task
        assert!(context.complete_task(task1_index.clone()));

        // Verify the task is completed
        let task = context.get_task(task1_index).unwrap();
        assert!(task.completed);

        // Verify the other task is not completed
        let task = context.get_task(task2_index).unwrap();
        assert!(!task.completed);
    }

    #[test]
    fn test_get_subtasks() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string());
        let task2_index = context.add_task("Task 2".to_string());

        // Move to the first task and add subtasks
        assert!(context.move_to(task1_index.clone()).is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string());
        let subtask2_index = context.add_task("Subtask 2".to_string());

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
        let root_index = context.add_task("Root task".to_string());
        assert_eq!(
            context.move_to(root_index.clone()),
            Some("Root task".to_string())
        );

        assert_eq!(context.get_current_index(), &vec![0]);

        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(
            context.move_to(task1_index.clone()),
            Some("Task 1".to_string())
        );
        assert_eq!(context.get_current_index(), &vec![0, 0]);

        let task2_index = context.add_task("Task 2".to_string());
        assert_eq!(context.get_current_index(), &vec![0, 0]);
        assert_eq!(
            context.move_to(task2_index.clone()),
            Some("Task 2".to_string())
        );
        assert_eq!(context.get_current_index(), &vec![0, 0, 0]);
    }

    #[test]
    fn test_get_with_history() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let mut context = Context::new(plan);
        let root_index = context.add_task("Root task".to_string());
        assert!(context.move_to(root_index.clone()).is_some());

        // Add sibling tasks
        let task1_index = context.add_task("Task 1".to_string());
        let _ = context.add_task("Task 2".to_string());

        // Move to the first task and add a subtask
        assert!(context.move_to(task1_index.clone()).is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(subtask1_index, vec![0, 0, 0]);

        // Test getting history for the subtask
        let history = context
            .plan
            .get_with_history(subtask1_index.clone())
            .unwrap();
        let (level, task, task_history) = history;

        // Verify the level is correct (3rd level is ordering)
        assert_eq!(level.description, ordering_level().description);
        assert_eq!(level.abstraction_focus, ordering_level().abstraction_focus);

        // Verify the task is correct
        assert_eq!(task.description, "Subtask 1");

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
        context.add_task("Root task".to_string());

        // Root level (empty cursor) should be 0
        assert_eq!(context.get_current_level(), 0);

        // Add a task at root level
        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(context.get_current_level(), 0);

        // Move to task1 (level 1)
        assert!(context.move_to(task1_index.clone()).is_some());
        assert_eq!(context.get_current_level(), 1);

        // Add a subtask to task1
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(context.get_current_level(), 1);

        // Move to subtask1 (level 2)
        assert!(context.move_to(subtask1_index.clone()).is_some());
        assert_eq!(context.get_current_level(), 2);

        // Set level back to 1
        context.set_current_level(1);
        assert_eq!(context.get_current_level(), 1);
        assert_eq!(context.get_current_index(), &task1_index);

        // Set level back to 0 (root)
        context.set_current_level(0);
        assert_eq!(context.get_current_level(), 0);
        assert!(context.get_current_index().is_empty());
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
}
