//! CLI module
//!
//! This module provides the command-line interface functionality for the scatterbrain tool.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;

use crate::{
    api::{serve, Client, ClientConfig, ClientError, ServerConfig},
    models::{default_levels, parse_index, Context, Core, Plan, PlanResponse, TaskTreeNode},
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API server URL
    #[arg(short, long, default_value = "http://localhost:3000")]
    server: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the scatterbrain API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,

        /// Populate with example task tree for UI testing
        #[arg(long)]
        example: bool,
    },

    /// Task management commands
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },

    /// Move to a task at the given index
    Move {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },

    /// Get the plan
    Plan,

    /// Get the current task
    Current,

    /// Get a distilled context of the current planning state
    Distilled,

    /// Interactive guide on how to use this tool
    Guide,

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Add a new task
    Add {
        /// Task description
        description: String,

        /// Level index (starting from 0, lower index = higher abstraction level)
        #[arg(short, long)]
        level: usize,
    },

    /// Complete the current task
    Complete,

    /// Change the abstraction level of the current task
    #[command(name = "change-level")]
    ChangeLevel {
        /// Level index (starting from 0)
        #[arg(help = "The level index to set (lower index = higher abstraction level)")]
        level_index: usize,
    },
}

/// Run the CLI application
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve { port, example } => {
            println!("Starting scatterbrain API server on port {}...", port);

            // Create a default plan with the default levels
            let plan = Plan::new(default_levels());
            let mut context = Context::new(plan);

            // Add example tasks if requested
            if *example {
                println!("Populating with example task tree for UI testing...");
                create_example_tasks(&mut context);
            }

            let core = Core::new(context);

            // Create a server configuration with the specified port
            let config = ServerConfig {
                address: ([127, 0, 0, 1], *port).into(),
            };

            // Start the API server
            serve(core, config).await?;
            Ok(())
        }

        Commands::Task { command } => {
            let client = create_client(&cli.server);
            let result = match command {
                TaskCommands::Add { description, level } => {
                    // Add the task, passing the level
                    let response = client.add_task(description.clone(), *level).await?;

                    // Get the task and index from the response
                    let (_task, index) = response.inner();

                    // No longer need to manually change the level here, it's set during add
                    println!(
                        "Added task: \"{}\" with level {} at index: {:?}",
                        description, level, index
                    );

                    Ok(())
                }

                TaskCommands::Complete => {
                    let response = client.complete_task().await?;

                    print_response(&response, |_| {
                        println!("Completed the current task");
                    });

                    Ok(())
                }

                TaskCommands::ChangeLevel { level_index } => {
                    // Get the current position
                    let current_response = client.get_current().await?;

                    // Safely extract the inner value
                    if current_response.inner().is_none() {
                        return Err("No current task selected".into());
                    }

                    // Clone the index to avoid borrowing issues
                    let index = current_response.inner().as_ref().unwrap().index.clone();

                    // Change the level
                    let response = client.change_level(index, *level_index).await?;

                    print_response(&response, |_| {
                        println!(
                            "Changed the abstraction level of the current task to {}",
                            level_index
                        );
                    });

                    Ok(())
                }
            };

            result
        }

        Commands::Move { index } => {
            let client = create_client(&cli.server);

            // Parse the index string (format: 0 or 0,1,2)
            let parsed_index = parse_index(index)?;

            let response = client.move_to(parsed_index).await?;

            print_response(&response, |description| {
                println!(
                    "Moved to task: \"{}\" at index: {}",
                    description.as_deref().unwrap_or("Unknown"),
                    index
                );
            });

            Ok(())
        }

        Commands::Plan => {
            let client = create_client(&cli.server);

            let plan_response = client.get_plan().await?;
            print_plan_response(&plan_response);

            Ok(())
        }

        Commands::Current => {
            let client = create_client(&cli.server);

            let result = match client.get_current().await {
                Ok(current_response) => {
                    print_response(&current_response, |current| {
                        // Check if we have a current task
                        if let Some(current) = current {
                            // Print current task info
                            let current_clone = current.clone();

                            println!("Current Task:");
                            println!("  Description: {}", current_clone.task.description());
                            println!("  Completed: {}", current_clone.task.is_completed());
                            println!("  Level: {}", current_clone.level.description());
                            println!("  Index: {:?}", current_clone.index);

                            if !current_clone.task.subtasks().is_empty() {
                                println!("\nSubtasks:");
                                for (i, subtask) in current_clone.task.subtasks().iter().enumerate()
                                {
                                    println!(
                                        "  {}. {} (completed: {})",
                                        i,
                                        subtask.description(),
                                        subtask.is_completed()
                                    );
                                }
                            }
                        } else {
                            println!("No current task selected. Use 'move' to select a task.");
                        }
                    });

                    Ok(())
                }
                Err(ClientError::Api(msg)) if msg.contains("Current task not found") => {
                    println!("No current task selected. Use 'move' to select a task.");
                    Ok(())
                }
                Err(e) => Err(e.into()),
            };

            result
        }

        Commands::Distilled => {
            let client = create_client(&cli.server);

            let distilled_response = client.get_distilled_context().await?;
            print_distilled_context_response(&distilled_response);
            Ok(())
        }

        Commands::Guide => {
            print_guide();

            // Only display distilled context if a server is running
            let client = create_client(&cli.server);
            if let Ok(distilled_response) = client.get_distilled_context().await {
                print_distilled_context_response(&distilled_response);
            }

            Ok(())
        }

        Commands::Completions { shell } => {
            // Generate completions for the specified shell
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(*shell, &mut cmd, bin_name, &mut io::stdout());
            Ok(())
        }
    }
}

fn create_client(server_url: &str) -> Client {
    let config = ClientConfig {
        base_url: server_url.to_string(),
    };

    Client::with_config(config)
}

/// Generic function to print any PlanResponse<T>
/// Takes a closure to handle printing the inner value
fn print_response<T, F>(response: &crate::models::PlanResponse<T>, print_inner: F)
where
    F: FnOnce(&T),
{
    // Print the inner value using the provided closure
    print_inner(response.inner());

    // Print follow-up suggestions if any
    if !response.suggested_followups.is_empty() {
        println!("\nSuggested next steps:");
        for suggestion in &response.suggested_followups {
            println!("  • {}", suggestion);
        }
    }

    // Print reminder if any
    if let Some(reminder) = &response.reminder {
        println!("\nReminder: {}", reminder);
    }

    // Print the distilled context
    print_distilled_context_response(response);
}

fn print_plan_response(response: &crate::models::PlanResponse<crate::models::Plan>) {
    print_response(response, |plan| {
        println!("Scatterbrain Plan:");

        println!("Levels: {}", plan.levels().len());

        println!("\nRoot Tasks:");
        if plan.root().subtasks().is_empty() {
            println!("  No tasks yet. Add some with 'scatterbrain task add'");
        } else {
            // Recursively print the task tree
            for (i, task) in plan.root().subtasks().iter().enumerate() {
                print_task(task, vec![i]);
            }
        }

        println!("\nAvailable Levels:");
        for (i, level) in plan.levels().iter().enumerate() {
            println!("  {}. {}", i + 1, level.get_guidance());
        }
    });
}

/// Recursively prints a task and its subtasks with proper indentation
fn print_task(task: &crate::models::Task, index: Vec<usize>) {
    // Calculate indentation (2 spaces per level)
    let indent = "  ".repeat(index.len());

    // Format the index as a string (e.g., "0.1.2")
    let index_str = index
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(".");

    let level_str = if let Some(level_index) = task.level_index() {
        format!("level: {}", level_index)
    } else {
        "level: unknown".to_string()
    };

    // Print the current task
    println!(
        "{}(index: [{}]) ({}), {} (completed: {})",
        indent,
        index_str,
        level_str,
        task.description(),
        task.is_completed()
    );

    // Recursively print subtasks
    for (i, subtask) in task.subtasks().iter().enumerate() {
        let mut subtask_index = index.clone();
        subtask_index.push(i);
        print_task(subtask, subtask_index);
    }
}

fn print_guide() {
    let guide = r#"
=== SCATTERBRAIN GUIDE ===

Scatterbrain is a hierarchical planning and task management tool designed to help
break down complex projects into manageable tasks through multiple abstraction levels.

== OVERVIEW ==

Scatterbrain helps you:
- Structure complex tasks in a logical hierarchy
- Navigate between different levels of abstraction
- Track progress and maintain focus
- Adapt your plan as work progresses

== ABSTRACTION LEVELS EXPLAINED ==

Scatterbrain uses a multi-level approach to planning:

1. High-level planning: Identifying architecture, scope, and approach
   - Focus on simplicity, extensibility, and good abstractions
   - Set the overall direction and boundaries of your project
   - Ask: "What's the overall architecture?" "Which approach should we take?"

2. Isolation: Breaking down the plan into discrete, independent parts
   - Define boundaries between components
   - Establish interfaces and contracts
   - Ensure each part can be completed and verified independently
   - Ask: "What are the interfaces?" "How should we divide this into parts?"

3. Ordering: Sequencing the parts in a logical flow
   - Start with foundational building blocks
   - Identify dependencies between tasks
   - Plan the critical path
   - Ask: "What order should we implement these?" "Which parts come first?"

4. Implementation: Converting each part into specific, actionable tasks
   - Define concrete, actionable steps
   - Detail exact implementation requirements
   - Make tasks independently completable
   - Ask: "What specific changes are needed?" "What files need modification?"

== TRANSITIONING BETWEEN LEVELS ==

MOVING DOWN:
  Level 1 → Level 2:
  • When your high-level approach is clear
  • When you're ready to define component boundaries
  • When you need to establish contracts between components
  
  Level 2 → Level 3:
  • When component boundaries are well-defined
  • When you need to determine implementation sequence
  • When you're ready to identify dependencies
  
  Level 3 → Level 4:
  • When the implementation sequence is clear
  • When you're ready to define specific tasks
  • When you're prepared to execute the implementation plan

MOVING UP:
  Level 4 → Level 3:
  • When you've completed implementation tasks
  • When you need to reorganize remaining task sequence
  • When you need to reprioritize work
  
  Level 3 → Level 2:
  • When you discover issues with component interfaces
  • When integration is more complex than expected
  • When you need to redefine component boundaries
  
  Level 2 → Level 1:
  • When you find fundamental flaws in the approach
  • When components don't form a coherent system
  • When you need to rethink the entire architecture

== WORKFLOW GUIDE ==

1. CREATE A PLAN AND NAVIGATE THE LEVELS
   - Begin at Level 0 with high-level planning:
     $ scatterbrain task add --level 0 "Design system architecture"
     $ scatterbrain move 0
     
   - Add subtasks at appropriate levels:
     $ scatterbrain task add --level 1 "Identify core components"
     $ scatterbrain move 0,0
     
   - Continue adding more granular tasks at deeper levels

2. STAY ON TRACK
   - Regularly review your plan:
     $ scatterbrain plan
     
   - Focus on your current task:
     $ scatterbrain current
     
   - Get a distilled context:
     $ scatterbrain distilled
     
   - Complete tasks when finished:
     $ scatterbrain task complete
     
   - Move between tasks to adapt to changing priorities:
     $ scatterbrain move 1,2

3. PROGRESSIVE REFINEMENT
   - Start with broad strokes at Level 0
   - Refine details as you move to deeper levels
   - Complete higher-level tasks only when all subtasks are done
   - Use completed tasks to validate your approach

== COMMAND REFERENCE ==

TASK MANAGEMENT:
  $ scatterbrain task add --level <LEVEL> "Task description"    Create new task (level is required)
  $ scatterbrain task complete                                 Complete current task
  $ scatterbrain task change-level <LEVEL_INDEX>               Change current task's abstraction level
  
NAVIGATION:
  $ scatterbrain move <INDEX>                                  Navigate to a task (e.g., 0 or 0,1,2)
  
VIEWING:
  $ scatterbrain plan                                          View the full plan with all tasks
  $ scatterbrain current                                       View details of the current task
  $ scatterbrain distilled                                     View a distilled context of your plan
  
SERVER MANAGEMENT:
  $ scatterbrain serve                                         Start the API server on default port 3000
  $ scatterbrain serve --port <PORT>                           Start the API server on a custom port
  $ scatterbrain serve --example                               Start with example task tree for testing
  
HELP & UTILITIES:
  $ scatterbrain guide                                         Show this guide
  $ scatterbrain completions <SHELL>                           Generate shell completions
  $ scatterbrain <COMMAND> --help                              Show help for a specific command
  $ scatterbrain --server <URL>                                Connect to a custom server URL

== BEST PRACTICES ==

PRODUCTIVITY TECHNIQUES:
  • Focus on one task at a time
  • Use 'current' and 'distilled' to maintain context
  • Complete tasks before moving to another
  • Revisit higher levels when assumptions change

LEVEL USAGE:
  • Use Level 0 for "why" questions
  • Use Level 1 for "what" questions
  • Use Level 2 for "when" questions
  • Use Level 3 for "how" questions

COMMON MISTAKES TO AVOID:
  • Premature implementation detail: Diving into code specifics at Level 0
  • Inconsistent abstractions: Mixing high-level and low-level concerns
  • Abstraction resistance: Staying too high-level when details are needed
  • Abstraction abandonment: Getting lost in details and forgetting the big picture
  • Level skipping: Jumping from Level 0 to Level 3 without proper planning

== TIPS ==

- When stuck, move up a level and reconsider your approach
- Keep tasks small and focused for easier tracking
- Use consistent naming patterns for related tasks
- Review completed tasks to learn what works
- Balance breadth vs. depth in your planning
- Recognize when to transition between levels
"#;

    println!("{}", guide);
}

/// Creates an example task tree for UI testing
fn create_example_tasks(context: &mut Context) {
    // Create top-level tasks (level 0 - Business Strategy)
    let result = context.add_task("Build Web Application".to_string(), 0);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Level 1 - Project Planning
    // Add subtasks to "Build Web Application"
    let result = context.add_task("Implement Frontend".to_string(), 1);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Level 2 - Implementation
    // Add subtasks to "Implement Frontend"
    let result = context.add_task("Design UI Components".to_string(), 2);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Level 3 - Implementation Details
    // Add subtasks to "Design UI Components"
    context
        .add_task("Implement User Authentication UI".to_string(), 3)
        .into_inner();

    // Move back up to "Implement Frontend"
    context.move_to(vec![0, 0, 0]).inner();

    // Back to parent
    context.move_to(vec![0, 0]).inner();

    // Add another subtask to "Implement Frontend"
    context
        .add_task("Set up State Management".to_string(), 2)
        .into_inner();

    // Move back to root
    context.move_to(vec![0]).inner();

    // Add "Implement Backend" as subtask of "Build Web Application"
    let result = context.add_task("Implement Backend".to_string(), 1);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Add backend tasks
    let result = context.add_task("Set up Database".to_string(), 2);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Add some API endpoint tasks
    context
        .add_task("Create API Endpoints".to_string(), 3)
        .into_inner();
    context
        .add_task("Implement Authentication Logic".to_string(), 3)
        .into_inner();
    context
        .add_task("Create Data Models".to_string(), 3)
        .into_inner();

    // Move back to "Set up Database"
    context.move_to(vec![0, 1, 0]).inner();

    // Add database schema tasks
    let result = context.add_task("Product Model".to_string(), 3);
    let (_, idx) = result.into_inner();
    context.move_to(idx).inner();

    // Add some fields
    context
        .add_task("Define Product Fields".to_string(), 3)
        .into_inner();
    context
        .add_task("Implement Relationships".to_string(), 3)
        .into_inner();

    // Move back to root level
    context.move_to(vec![0]).inner();

    // Add a few more top level tasks
    context
        .add_task("Write Documentation".to_string(), 0)
        .into_inner();
    context
        .add_task("Test Application".to_string(), 0)
        .into_inner();

    // Reset to root
    context.move_to(vec![]).inner();
}

/// Print a distilled context from any PlanResponse
fn print_distilled_context_response<T>(response: &PlanResponse<T>) {
    let context = &response.distilled_context;

    println!("\n=== DISTILLED CONTEXT ===\n");

    // Print usage summary
    println!("USAGE SUMMARY:");
    println!("{}", context.usage_summary);
    println!("");

    // Print current task and level
    println!("CURRENT POSITION:");
    if let Some(task) = &context.current_task {
        println!("  Current task: \"{}\"", task.description());
        println!(
            "  Completed: {}",
            if task.is_completed() { "Yes" } else { "No" }
        );
    } else {
        println!("  At root level (no task selected)");
    }

    if let Some(level) = &context.current_level {
        println!(
            "  Current abstraction level: {} - {}",
            level.name(),
            level.description()
        );
        println!("  Focus: {}", level.abstraction_focus());
    }
    println!("");

    // Print available levels
    println!("AVAILABLE ABSTRACTION LEVELS:");
    for (idx, level) in context.levels.iter().enumerate() {
        println!(
            "  Level {}: {} - {}",
            idx,
            level.name(),
            level.description()
        );
        println!("    Focus: {}", level.abstraction_focus());

        // Print a couple of sample questions for each level
        let questions = level.questions();
        if !questions.is_empty() {
            println!("    Sample questions:");
            for (_q_idx, question) in questions.iter().enumerate().take(2) {
                println!("      • {}", question);
            }

            // Indicate if there are more questions
            if questions.len() > 2 {
                println!("      • ... and {} more", questions.len() - 2);
            }
        }
        println!("");
    }

    // Print task tree
    println!("TASK TREE:");
    print_task_tree(&context.task_tree, 0);

    // Print followups and reminder
    if !response.suggested_followups.is_empty() {
        println!("\nSuggested next steps:");
        for followup in &response.suggested_followups {
            println!("  • {}", followup);
        }
    }

    if let Some(reminder) = &response.reminder {
        println!("\nReminder: {}", reminder);
    }

    println!("");
}

fn print_task_tree(nodes: &[TaskTreeNode], indent: usize) {
    for node in nodes {
        // Create indentation
        let indent_str = "  ".repeat(indent);

        // Create indicator for current task
        let current_indicator = if node.is_current { "→ " } else { "  " };

        // Create completion status
        let completion_status = if node.completed { "[✓]" } else { "[ ]" };

        // Print the task with appropriate formatting
        println!(
            "{}{}{}{}",
            indent_str, current_indicator, completion_status, node.description
        );

        // Recursively print children with increased indentation
        if !node.children.is_empty() {
            print_task_tree(&node.children, indent + 1);
        }
    }
}
