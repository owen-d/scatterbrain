use scatterbrain::levels::default_levels;
use scatterbrain::models::{Context, Core, Plan, Task};

#[test]
fn test_task_creation() {
    let task = Task::with_level("Test task".to_string(), 2);

    assert_eq!(task.description(), "Test task");
    assert_eq!(task.level_index(), Some(2));
    assert!(!task.is_completed());
    assert!(task.subtasks().is_empty());
}

#[test]
fn test_task_default_level() {
    let task = Task::new("Test task".to_string());

    assert_eq!(task.level_index(), None); // Default is None
}

/// Since set_level is now private, we'll test level setting via Context.change_level
#[test]
fn test_task_change_level() {
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add a task and change its level
    let task_index = context.add_task("Task 1".to_string());
    // Lower level (higher abstraction) is valid
    context.change_level(task_index.clone(), 0).unwrap();

    // Get the current task and verify its level
    context.move_to(task_index.clone()).unwrap();
    let current = context.get_current_with_history().unwrap();
    let (_, task, _) = current;
    assert_eq!(task.level_index(), Some(0));
}

#[test]
fn test_plan_with_default_levels() {
    let plan = Plan::new(default_levels());

    // Verify the plan has the expected number of levels
    assert_eq!(plan.levels().len(), 4);
}

#[test]
fn test_context_get_current_with_history() {
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add a root task with level 0
    let task0_index = context.add_task("Root Task".to_string());

    // Move to root task 0
    context.move_to(task0_index.clone()).unwrap();

    // Add a first task as a child with level 1
    let task1_index = context.add_task("Task 1".to_string());

    // Move to first task
    context.move_to(task1_index.clone()).unwrap();

    // Add a second task as a child with level 2
    let task2_index = context.add_task("Task 2".to_string());

    // Move to second task
    context.move_to(task2_index.clone()).unwrap();

    // Get current task and history
    let (level, task, history) = context.get_current_with_history().unwrap();

    // Verify task description
    assert_eq!(task.description(), "Task 2");
    // Assert the level is appropriately assigned based on depth
    let expected_level = task.level_index().unwrap_or(task2_index.len() - 1);
    assert_eq!(
        level.description(),
        default_levels()[expected_level].description()
    );

    // Verify history length (the actual history length is 3, not 2)
    assert_eq!(history.len(), 3);
}

#[test]
fn test_task_completion() {
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add tasks
    let task1_index = context.add_task("Task 1".to_string());
    let task2_index = context.add_task("Task 2".to_string());

    // Complete task 1
    context.complete_task(task1_index.clone());

    // Get current task with history for each to check completion
    context.move_to(task1_index.clone()).unwrap();
    let (_, task1, _) = context.get_current_with_history().unwrap();

    context.move_to(task2_index.clone()).unwrap();
    let (_, task2, _) = context.get_current_with_history().unwrap();

    // Verify task 1 is completed and task 2 is not
    assert!(task1.is_completed());
    assert!(!task2.is_completed());
}

#[test]
fn test_core_functionality() {
    let plan = Plan::new(default_levels());
    let context = Context::new(plan);
    let core = Core::new(context);

    // Add a task
    let task_index = core.add_task("Task via Core".to_string()).unwrap();

    // Move to the task
    core.move_to(task_index.clone()).unwrap();

    // Get current task
    let current = core.current().unwrap();

    // Verify task properties
    assert_eq!(current.task.description(), "Task via Core");
    assert!(!current.task.is_completed());

    // Complete the task
    assert!(core.complete_task());

    // Verify task is now completed
    let current = core.current().unwrap();
    assert!(current.task.is_completed());

    // Test change level
    let result = core.change_level(task_index.clone(), 0);
    assert!(result.is_ok());

    // Verify level was changed
    let current = core.current().unwrap();
    assert_eq!(current.task.level_index(), Some(0));
}

#[test]
fn test_level_validation() {
    let plan = Plan::new(default_levels());
    let mut context = Context::new(plan);

    // Add a root task
    let root_index = context.add_task("Root Task".to_string());

    // Set root task to level 0
    let result = context.change_level(root_index.clone(), 0);
    assert!(result.is_ok(), "Root task should accept level 0");

    // Move to root task and add a child
    context.move_to(root_index.clone()).unwrap();
    let child_index = context.add_task("Child Task".to_string());

    // Child task should accept level 0 (same as parent)
    let result = context.change_level(child_index.clone(), 0);
    assert!(result.is_ok(), "Child should accept same level as parent");

    // Child task should not accept level higher than parent
    let result = context.change_level(child_index.clone(), 1);
    assert!(
        result.is_err(),
        "Child should not accept level higher than parent"
    );

    // Root task with child at level 0 should not be able to change to level 1
    // This is because a parent task can't have a level that would make its children
    // have a higher abstract level
    let result = context.change_level(root_index.clone(), 1);
    assert!(
        result.is_err(),
        "Root task with child at level 0 should not accept level change to 1"
    );
}

#[test]
fn test_parse_index() {
    use scatterbrain::models::parse_index;

    // Test successful parsing
    let index = parse_index("0,1,2").unwrap();
    assert_eq!(index, vec![0, 1, 2]);

    // Test single element
    let index = parse_index("0").unwrap();
    assert_eq!(index, vec![0]);

    // Test invalid input
    let result = parse_index("a,b,c");
    assert!(result.is_err());
}
