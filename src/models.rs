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
    description: String,
    completed: bool,
    subtasks: Vec<Task>,
    level_index: Option<usize>,
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
    /// Adds a new task with the given description
    pub fn add_task(&mut self, description: String) -> PlanResponse<(Task, Index)> {
        let task = Task::new(description);
        let new_index;
        let task_clone = task.clone();

        if self.cursor.is_empty() {
            // Adding to root task, special case
            self.plan.root_mut().add_subtask(task);
            new_index = vec![self.plan.root().subtasks().len() - 1];
        } else {
            // Navigate to the current task
            let current = self.get_current_task_mut().unwrap();

            // Add the new task
            current.add_subtask(task);
            let task_index = current.subtasks().len() - 1;

            // Create the new index
            let mut task_index_vec = self.cursor.clone();
            task_index_vec.push(task_index);
            new_index = task_index_vec;
        }

        PlanResponse::new(
            (task_clone, new_index),
            vec![
                "move to the new task".to_string(),
                "add a subtask".to_string(),
            ],
            Some(
                "Next, consider breaking this task down further or organizing its level"
                    .to_string(),
            ),
        )
    }

    /// Moves to the task at the given index
    pub fn move_to(&mut self, index: Index) -> PlanResponse<Option<String>> {
        // Validate the index
        if index.is_empty() {
            self.cursor = Vec::new();
            return PlanResponse::new(
                Some("root".to_string()),
                vec!["add a task".to_string(), "view the plan".to_string()],
                Some("You're at the root level of your plan".to_string()),
            );
        }

        // Check if the index is valid
        let task_opt = self.get_task(index.clone());
        if let Some(task) = task_opt {
            let description = task.description().to_string();

            // Create followup suggestions based on task state
            let mut followups = vec!["add a subtask".to_string()];
            if !task.is_completed() {
                followups.push("complete this task".to_string());
            }
            if !task.subtasks().is_empty() {
                followups.push("move to a subtask".to_string());
            }

            // Create reminder
            let reminder = if !task.is_completed() && !task.subtasks().is_empty() {
                Some("Remember to complete subtasks before marking this task as done".to_string())
            } else if !task.is_completed() {
                Some("Focus on this task until it's completed".to_string())
            } else {
                Some("This task is already completed".to_string())
            };

            // Set cursor after we're done with task operations
            self.cursor = index;

            PlanResponse::new(Some(description), followups, reminder)
        } else {
            PlanResponse::new(
                None,
                vec!["view the plan".to_string()],
                Some("Invalid task index. Try viewing the plan to find a valid task.".to_string()),
            )
        }
    }

    // Task state management
    /// Completes the task at the given index
    pub fn complete_task(&mut self, index: Index) -> PlanResponse<bool> {
        let success = if let Some(task) = self.get_task_mut(index) {
            task.complete();
            true
        } else {
            false
        };

        PlanResponse::new(
            success,
            vec![
                "move to the next task".to_string(),
                "add a related task".to_string(),
            ],
            Some("Great progress! What's next on your plan?".to_string()),
        )
    }

    /// Changes the level of a task at the given index
    pub fn change_level(
        &mut self,
        index: Index,
        level_index: usize,
    ) -> PlanResponse<Result<(), String>> {
        // Validate: the level must exist
        if level_index >= self.plan.level_count() {
            return PlanResponse::new(
                Err(format!("Level index {} is out of bounds", level_index)),
                vec!["view the plan".to_string()],
                Some(format!(
                    "Available levels are 0-{}",
                    self.plan.level_count() - 1
                )),
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
                        vec!["change parent level first".to_string()],
                        Some("Remember that parent tasks should have equal or higher abstraction levels than their children".to_string()),
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
                return PlanResponse::new(
                    Err(e),
                    vec!["change child levels first".to_string()],
                    Some(
                        "Change the levels of child tasks before changing the parent task level"
                            .to_string(),
                    ),
                );
            }
        }

        // Apply the change
        if let Some(task) = self.get_task_mut(index) {
            // Set the level
            task.set_level(level_index);

            // Generate followups based on the new level
            let followup_suggestions = vec![
                "continue working on this task".to_string(),
                "move to another task".to_string(),
            ];

            // Create a level-specific reminder
            let reminder = match level_index {
                0 => Some("This is now a high-level strategic task".to_string()),
                1 => Some("This is now a task focused on component isolation".to_string()),
                2 => Some("This is now a task focused on ordering and sequence".to_string()),
                3 => Some("This is now an implementation-level task".to_string()),
                _ => Some(format!("Task is now at level {}", level_index)),
            };

            PlanResponse::new(Ok(()), followup_suggestions, reminder)
        } else {
            PlanResponse::new(
                Err("Task not found".to_string()),
                vec!["view the plan".to_string()],
                Some("Check the plan to find a valid task index".to_string()),
            )
        }
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
        PlanResponse::new(
            self.cursor.clone(),
            vec![
                "move to different task".to_string(),
                "add a subtask".to_string(),
            ],
            Some("Consider where you are in the task hierarchy".to_string()),
        )
    }

    /// Gets the current level based on cursor depth
    pub fn get_current_level(&self) -> usize {
        self.get_current_index().inner().len()
    }

    /// Sets the current level by trimming the cursor
    pub fn set_current_level(&mut self, level: usize) {
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
        PlanResponse::new(
            self.plan.clone(),
            vec!["Add tasks to build your plan".to_string()],
            Some("Focus on one level of abstraction at a time".to_string()),
        )
    }

    /// Gets the current task with history
    pub fn get_current_with_history(&self) -> Option<(Level, Task, Vec<String>)> {
        self.plan.get_with_history(self.cursor.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResponse<T> {
    pub res: T,
    pub suggested_followups: Vec<String>,
    pub reminder: Option<String>,
}

impl<T> PlanResponse<T> {
    pub fn new(res: T, suggested_followups: Vec<String>, reminder: Option<String>) -> Self {
        Self {
            res,
            suggested_followups,
            reminder,
        }
    }

    pub fn inner(&self) -> &T {
        &self.res
    }

    pub fn into_inner(self) -> T {
        self.res
    }

    pub fn replace<B>(self, res: B) -> PlanResponse<B> {
        PlanResponse::new(res, self.suggested_followups, self.reminder)
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

    pub fn get_plan(&self) -> PlanResponse<Option<Plan>> {
        let context = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let plan_response = context.get_plan();
        let followups = plan_response.suggested_followups.clone();
        let reminder = plan_response.reminder.clone();
        let plan = Some(plan_response.into_inner());

        PlanResponse::new(plan, followups, reminder)
    }

    pub fn current(&self) -> PlanResponse<Option<Current>> {
        self.with_context(|context| {
            let index_response = context.get_current_index();
            let index = index_response.inner().clone();

            let followups = index_response.suggested_followups.clone();
            let reminder = index_response.reminder.clone();

            let current_opt = context
                .get_current_with_history()
                .map(|(level, task, history)| Current {
                    index,
                    level,
                    task,
                    history,
                });

            PlanResponse::new(current_opt, followups, reminder)
        })
    }

    pub fn add_task(&self, description: String) -> PlanResponse<Option<(Task, Index)>> {
        self.with_context(|context| {
            let response = context.add_task(description);
            let followups = response.suggested_followups.clone();
            let reminder = response.reminder.clone();
            PlanResponse::new(Some(response.into_inner()), followups, reminder)
        })
    }

    pub fn complete_task(&self) -> PlanResponse<bool> {
        self.with_context(|context| {
            // Get the current index as PlanResponse<Index>
            let index_response = context.get_current_index();
            // Extract just the index value
            let index = index_response.inner().clone();
            // Complete the task
            context.complete_task(index)
        })
    }

    pub fn move_to(&self, index: Index) -> PlanResponse<Option<String>> {
        self.with_context(|context| context.move_to(index))
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
        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        assert_eq!(task1_index, vec![0]);

        // Add another task at the root level
        let task2_index = context.add_task("Task 2".to_string()).into_inner().1;
        assert_eq!(task2_index, vec![1]);

        // Move to the first task
        let move_response = context.move_to(task1_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 1".to_string()));

        // Add a subtask to the first task
        let subtask1_index = context.add_task("Subtask 1".to_string()).into_inner().1;
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
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        let task2_index = context.add_task("Task 2".to_string()).into_inner().1;

        // Complete a task
        assert!(*context.complete_task(task1_index.clone()).inner());

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
        context.add_task("Root task".to_string());

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        let task2_index = context.add_task("Task 2".to_string()).into_inner().1;

        // Move to the first task and add subtasks
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string()).into_inner().1;
        let subtask2_index = context.add_task("Subtask 2".to_string()).into_inner().1;

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
        let root_index = context.add_task("Root task".to_string()).into_inner().1;
        let move_response = context.move_to(root_index.clone());
        assert_eq!(move_response.inner(), &Some("Root task".to_string()));

        assert_eq!(*context.get_current_index().inner(), vec![0]);

        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        let move_response = context.move_to(task1_index.clone());
        assert_eq!(move_response.inner(), &Some("Task 1".to_string()));
        assert_eq!(*context.get_current_index().inner(), vec![0, 0]);

        let task2_index = context.add_task("Task 2".to_string()).into_inner().1;
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
        let root_index = context.add_task("Root task".to_string()).into_inner().1;
        let move_response = context.move_to(root_index.clone());
        assert!(move_response.inner().is_some());

        // Add sibling tasks
        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        let _ = context.add_task("Task 2".to_string());

        // Move to the first task and add a subtask
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        let subtask1_index = context.add_task("Subtask 1".to_string()).into_inner().1;
        assert_eq!(subtask1_index, vec![0, 0, 0]);

        // Test getting history for the subtask
        let move_response = context.move_to(subtask1_index.clone());
        assert!(move_response.inner().is_some());
        let (level, task, task_history) = context.get_current_with_history().unwrap();

        // Verify the level is correct (we're at depth 3, so using isolation level)
        assert_eq!(level.description(), default_levels()[2].description());
        assert_eq!(
            level.abstraction_focus(),
            default_levels()[2].abstraction_focus()
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
        context.add_task("Root task".to_string());

        // Root level (empty cursor) should be 0
        assert_eq!(context.get_current_level(), 0);

        // Add a task at root level
        let task1_index = context.add_task("Task 1".to_string()).into_inner().1;
        assert_eq!(context.get_current_level(), 0);

        // Move to task1 (level 1)
        let move_response = context.move_to(task1_index.clone());
        assert!(move_response.inner().is_some());
        assert_eq!(context.get_current_level(), 1);

        // Add a subtask to task1
        let subtask1_index = context.add_task("Subtask 1".to_string()).into_inner().1;
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
        let task_index = context.add_task("Task 1".to_string()).into_inner().1;

        // Change the level
        let result = context.change_level(task_index.clone(), 0);
        assert!(result.inner().is_ok());

        // Verify the level was changed
        let task = context.get_task(task_index).unwrap();
        assert_eq!(task.level_index(), Some(0));
    }

    #[test]
    fn test_core_with_context() {
        // Create a plan with default levels
        let plan = Plan::new(default_levels());
        let context = Context::new(plan);
        let core = Core::new(context);

        // Use add_task through Core
        let response = core.add_task("Task through Core".to_string());
        let task_index = response.into_inner().unwrap().1;

        // Move to the task
        let move_response = core.move_to(task_index.clone());
        assert_eq!(
            move_response.inner(),
            &Some("Task through Core".to_string())
        );

        // Complete the task
        assert!(*core.complete_task().inner());

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
}
