//! Scatterbrain library crate
//!
//! This library provides functionality for the scatterbrain tool.

/// Represents an abstraction level for the LLM to work through
#[derive(Debug, Clone, Copy)]
pub struct Level {
    pub description: &'static str,
    pub questions: &'static [&'static str],
}

pub const PLAN: Level = Level {
    description: "high level planning; identifying architecture, scope, and approach",
    questions: &[
        "Is this approach simple?",
        "Is this approach extensible?",
        "Does this approach provide good, minimally leaking abstractions?",
    ],
};

pub const ISOLATION: Level = Level {
    description: "Identifying discrete parts of the plan which can be completed independently",
    questions: &[
        "If possible, can each part be completed and verified independently",
        "Are the boundaries between pieces modular and extensible?",
    ],
};

pub const ORDERING: Level = Level {
    description: "Ordering the parts of the plan",
    questions: &[
        "Do we move from foundational building blocks to more complex concepts?",
        "Do we follow idiomatic design patterns?",
    ],
};

pub const IMPLEMENTATION: Level = Level {
    description: "Turning each part into an ordered list of tasks",
    questions: &[
        "Can each task be completed independently?",
        "Is each task complimentary to, or does it build upon, the previous tasks?",
        "Does each task minimize the execution risk of the other tasks?",
    ],
};

pub const DEFAULT_LEVELS: &[Level] = &[PLAN, ISOLATION, ORDERING, IMPLEMENTATION];

/// Represents a task in the LLM's work
#[derive(Debug, Clone)]
pub struct Task {
    pub level: usize,
    pub description: String,
    pub completed: bool,
    pub subtasks: Vec<Task>,
}

impl Task {
    /// Creates a new task with the given level and description
    pub fn new(level: usize, description: String) -> Self {
        Self {
            level,
            description,
            completed: false,
            subtasks: Vec::new(),
        }
    }

    /// Adds a subtask to this task
    pub fn add_subtask(&mut self, subtask: Task) {
        self.subtasks.push(subtask);
    }

    /// Marks this task as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// Returns true if this task and all its subtasks are completed
    pub fn is_fully_completed(&self) -> bool {
        self.completed && self.subtasks.iter().all(|t| t.is_fully_completed())
    }
}

pub struct Plan {
    pub task: Task,
    pub levels: Vec<Level>,
}

impl Plan {
    /// Creates a new plan with a root task and the given levels
    pub fn new(root_task: Task, levels: Vec<Level>) -> Self {
        Self {
            task: root_task,
            levels,
        }
    }
}

// shorthand for the index of a task in the plan tree
type Index = Vec<usize>;

/// Context for managing the planning process
pub struct Context {
    plan: Plan,
    current_level: usize,
    cursor: Index,
}

impl Context {
    /// Creates a new context with the given plan
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            current_level: 0,
            cursor: Vec::new(), // Start at root
        }
    }

    // Task creation and navigation
    pub fn add_task(&mut self, description: String) -> Index {
        let task = Task::new(self.current_level, description);
        let new_index;

        if self.cursor.is_empty() {
            // Adding to root task
            self.plan.task.add_subtask(task);
            new_index = vec![0, self.plan.task.subtasks.len() - 1];
        } else {
            // Navigate to the current task
            let mut current = &mut self.plan.task;
            for &index in &self.cursor[1..] {
                current = &mut current.subtasks[index];
            }

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

    pub fn move_to(&mut self, index: Index) -> bool {
        // Validate the index
        if index.is_empty() {
            self.cursor = index;
            return true;
        }

        // Check if the index is valid
        if let Some(_) = self.get_task(index.clone()) {
            self.cursor = index;
            true
        } else {
            false
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
            return Some(&self.plan.task);
        }

        let mut current = &self.plan.task;
        for &idx in &index[1..] {
            if idx >= current.subtasks.len() {
                return None;
            }
            current = &current.subtasks[idx];
        }

        Some(current)
    }

    pub fn get_task_mut(&mut self, index: Index) -> Option<&mut Task> {
        if index.is_empty() {
            return Some(&mut self.plan.task);
        }

        let mut current = &mut self.plan.task;
        for &idx in &index[1..] {
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
        self.current_level
    }

    pub fn set_current_level(&mut self, level: usize) {
        if level < self.plan.levels.len() {
            self.current_level = level;
        }
    }

    // Tree traversal
    pub fn get_tasks_at_level(&self, level: usize) -> Vec<(Index, &Task)> {
        let mut result = Vec::new();
        self.collect_tasks_at_level(&self.plan.task, vec![0], level, &mut result);
        result
    }

    fn collect_tasks_at_level<'a>(
        &'a self,
        task: &'a Task,
        index: Index,
        target_level: usize,
        result: &mut Vec<(Index, &'a Task)>,
    ) {
        if task.level == target_level {
            result.push((index.clone(), task));
        }

        for (i, subtask) in task.subtasks.iter().enumerate() {
            let mut new_index = index.clone();
            new_index.push(i);
            self.collect_tasks_at_level(subtask, new_index, target_level, result);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation_and_navigation() {
        // Create a plan with default levels
        let root_task = Task::new(0, "Root task".to_string());
        let plan = Plan::new(root_task, DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);

        // Add a task at the root level
        let task1_index = context.add_task("Task 1".to_string());
        assert_eq!(task1_index, vec![0, 0]);

        // Add another task at the root level
        let task2_index = context.add_task("Task 2".to_string());
        assert_eq!(task2_index, vec![0, 1]);

        // Move to the first task
        assert!(context.move_to(task1_index.clone()));

        // Add a subtask to the first task
        let subtask1_index = context.add_task("Subtask 1".to_string());
        assert_eq!(subtask1_index, vec![0, 0, 0]);

        // Move to the second task
        assert!(context.move_to(task2_index.clone()));
        assert_eq!(context.get_current_index(), &vec![0, 1]);

        // Move to subtask 1
        assert!(context.move_to(subtask1_index.clone()));
        assert_eq!(context.get_current_index(), &vec![0, 0, 0]);
    }

    #[test]
    fn test_task_completion() {
        // Create a plan with default levels
        let root_task = Task::new(0, "Root task".to_string());
        let plan = Plan::new(root_task, DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);

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
    fn test_get_tasks_at_level() {
        // Create a plan with default levels
        let root_task = Task::new(0, "Root task".to_string());
        let plan = Plan::new(root_task, DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);

        // Add tasks at different levels
        context.set_current_level(1);
        let task1_index = context.add_task("Task 1".to_string());

        context.set_current_level(2);
        let task2_index = context.add_task("Task 2".to_string());

        // Get tasks at level 1
        let level1_tasks = context.get_tasks_at_level(1);
        assert_eq!(level1_tasks.len(), 1);
        assert_eq!(level1_tasks[0].0, task1_index);

        // Get tasks at level 2
        let level2_tasks = context.get_tasks_at_level(2);
        assert_eq!(level2_tasks.len(), 1);
        assert_eq!(level2_tasks[0].0, task2_index);
    }

    #[test]
    fn test_get_subtasks() {
        // Create a plan with default levels
        let root_task = Task::new(0, "Root task".to_string());
        let plan = Plan::new(root_task, DEFAULT_LEVELS.to_vec());
        let mut context = Context::new(plan);

        // Add tasks
        let task1_index = context.add_task("Task 1".to_string());
        let task2_index = context.add_task("Task 2".to_string());

        // Move to the first task and add subtasks
        context.move_to(task1_index.clone());
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
}
