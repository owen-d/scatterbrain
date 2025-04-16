use scatterbrain::models::{default_levels, Context, Plan, Task};

#[test]
fn test_task_creation_with_level() {
    // Test creating a task with a specific level
    let task = Task::with_level("Test task".to_string(), 2);
    assert_eq!(task.description, "Test task");
    assert_eq!(task.level_index, Some(2));
    assert!(!task.completed);
    assert!(task.subtasks.is_empty());
}

#[test]
fn test_task_level_setting() {
    // Test setting the level on an existing task
    let mut task = Task::new("Test task".to_string());
    assert_eq!(task.level_index, None); // Default is None

    task.set_level(1);
    assert_eq!(task.level_index, Some(1));

    // Change level
    task.set_level(3);
    assert_eq!(task.level_index, Some(3));
}

#[test]
fn test_effective_level_calculation() {
    // Test the effective level calculation logic

    // Task with explicit level
    let task_with_level = Task::with_level("Explicit level".to_string(), 2);
    assert_eq!(task_with_level.get_effective_level(0), 2); // Explicit level overrides depth
    assert_eq!(task_with_level.get_effective_level(3), 2); // Explicit level overrides depth

    // Task without explicit level
    let task_without_level = Task::new("Implicit level".to_string());
    assert_eq!(task_without_level.get_effective_level(0), 0); // Uses depth
    assert_eq!(task_without_level.get_effective_level(3), 3); // Uses depth
}

#[test]
fn test_context_level_constraints() {
    // Create a plan with default levels
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add a root task and set its level
    let root_task_index = context.add_task("Root task".to_string());
    assert!(context.get_task(root_task_index.clone()).is_some());

    // Move to the root task
    assert!(context.move_to(root_task_index.clone()).is_some());

    // Add a subtask
    let subtask_index = context.add_task("Subtask".to_string());

    // Set levels directly on the root task
    {
        let root_task = context.get_task_mut(root_task_index.clone()).unwrap();
        root_task.set_level(2);
        assert_eq!(root_task.level_index, Some(2));
    }

    // Set subtask level lower than parent (higher abstraction)
    {
        let subtask = context.get_task_mut(subtask_index.clone()).unwrap();
        subtask.set_level(1);
        assert_eq!(subtask.level_index, Some(1));
    }

    // Add a sub-subtask
    assert!(context.move_to(subtask_index.clone()).is_some());
    let sub_subtask_index = context.add_task("Sub-subtask".to_string());

    // Set level equal to parent
    {
        let sub_subtask = context.get_task_mut(sub_subtask_index.clone()).unwrap();
        sub_subtask.set_level(1);
        assert_eq!(sub_subtask.level_index, Some(1));
    }
}

#[test]
fn test_plan_level_inheritance() {
    // Test how levels are handled in the plan's get_with_history method
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add tasks with and without explicit levels
    let task1_index = context.add_task("Task 1".to_string());
    {
        let task1 = context.get_task_mut(task1_index.clone()).unwrap();
        task1.set_level(0); // Explicitly set to highest abstraction
    }

    let task2_index = context.add_task("Task 2".to_string());
    // Leave task2's level as None (implicit)

    // Get history for task1
    let (level1, task1_clone, _history1) =
        context.get_plan().get_with_history(task1_index).unwrap();
    assert_eq!(task1_clone.level_index, Some(0));
    assert_eq!(level1.description, default_levels()[0].description);

    // Get history for task2
    let (level2, task2_clone, _history2) =
        context.get_plan().get_with_history(task2_index).unwrap();
    assert_eq!(task2_clone.level_index, None);
    // Should use position-based level (index.len() - 1)
    assert_eq!(level2.description, default_levels()[0].description);
}

#[test]
fn test_nested_task_level_calculation() {
    // Test effective level calculation in a nested structure
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Create a hierarchy with mixed explicit and implicit levels
    let level0_task_index = context.add_task("Level 0 task".to_string());
    {
        let level0_task = context.get_task_mut(level0_task_index.clone()).unwrap();
        level0_task.set_level(0);
    }

    context.move_to(level0_task_index.clone()).unwrap();

    // Child with implicit level
    let child1_index = context.add_task("Child 1 (implicit)".to_string());

    // Child with explicit level
    let child2_index = context.add_task("Child 2 (explicit)".to_string());
    {
        let child2 = context.get_task_mut(child2_index.clone()).unwrap();
        child2.set_level(0); // Same level as parent
    }

    // Check effective levels
    let child1 = context.get_task(child1_index).unwrap();
    let child2 = context.get_task(child2_index).unwrap();

    // Child1 should use depth-based level
    assert_eq!(child1.level_index, None);
    assert_eq!(child1.get_effective_level(2), 2);

    // Child2 should use explicit level
    assert_eq!(child2.level_index, Some(0));
    assert_eq!(child2.get_effective_level(2), 0);
}

#[test]
fn test_level_bounds_validation() {
    // Test that we can't set a level that doesn't exist
    let plan = Plan::new(default_levels());
    let context = Context::new(plan);
    let core = scatterbrain::models::Core::new(context);

    // Add a root task
    let root_index = core.add_task("Root task".to_string()).unwrap();

    // Get number of available levels
    let level_count = core.get_plan().unwrap().levels.len();
    assert!(level_count > 0);

    // Valid level (should succeed)
    assert!(core.change_level(root_index.clone(), 0).is_ok());

    // Out of bounds level (should fail)
    let result = core.change_level(root_index.clone(), level_count);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.contains("out of bounds"),
        "Unexpected error: {}",
        error
    );
}

#[test]
fn test_task_level_hierarchy_constraints() {
    // Test setting task levels in a hierarchy with appropriate constraints
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add a parent task
    let parent_index = context.add_task("Parent task".to_string());

    // Set parent to level 2
    let parent = context.get_task_mut(parent_index.clone()).unwrap();
    parent.set_level(2);

    // Move to parent
    context.move_to(parent_index.clone()).unwrap();

    // Add a child task
    let child_index = context.add_task("Child task".to_string());

    {
        // Child can have the same level as parent
        let child = context.get_task_mut(child_index.clone()).unwrap();
        child.set_level(2);
        assert_eq!(child.level_index, Some(2));
    }

    {
        // Child can have higher abstraction (lower number) than parent
        let child = context.get_task_mut(child_index.clone()).unwrap();
        child.set_level(1);
        assert_eq!(child.level_index, Some(1));

        // Child can have even higher abstraction
        child.set_level(0);
        assert_eq!(child.level_index, Some(0));
    }

    // Move to the child
    context.move_to(child_index.clone()).unwrap();

    // Add a grandchild
    let grandchild_index = context.add_task("Grandchild task".to_string());

    {
        // Grandchild inherits level from depth if not specified
        let grandchild = context.get_task(grandchild_index.clone()).unwrap();
        assert_eq!(grandchild.level_index, None);
        // Effective level is depth-based (index.len() - 1)
        assert_eq!(grandchild.get_effective_level(3), 3);
    }

    {
        // Grandchild can be explicitly set to parent's level
        let grandchild = context.get_task_mut(grandchild_index.clone()).unwrap();
        grandchild.set_level(0);
        assert_eq!(grandchild.level_index, Some(0));
    }
}
