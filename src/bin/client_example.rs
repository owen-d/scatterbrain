//! Example client for the scatterbrain API

use scatterbrain::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with default configuration (localhost:3000)
    // You can customize with ClientConfig if needed
    let client = Client::new();
    println!("Scatterbrain API Client Example");
    println!("-------------------------------");

    // Get the plan
    println!("\nFetching plan...");
    let plan = client.get_plan().await?;
    println!("Plan has {} levels", plan.levels.len());

    // Add a task
    println!("\nAdding a task...");
    let task_index = client.add_task("Client example task".to_string()).await?;
    println!("Added task at index: {:?}", task_index);

    // Move to the task
    println!("\nMoving to the task...");
    client.move_to(task_index).await?;
    println!("Moved to task");

    // Get current task
    println!("\nFetching current task...");
    let current = client.get_current().await?;
    println!(
        "Current task: {} (completed: {})",
        current.task.description, current.task.completed
    );

    // Complete the task
    println!("\nCompleting the task...");
    client.complete_task().await?;
    println!("Task completed");

    // Verify completion
    println!("\nVerifying task completion...");
    let current = client.get_current().await?;
    println!(
        "Current task: {} (completed: {})",
        current.task.description, current.task.completed
    );

    println!("\nAll operations completed successfully!");
    Ok(())
}
