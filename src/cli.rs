//! CLI module
//!
//! This module provides the command-line interface functionality for the scatterbrain tool.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;

use crate::{
    api::{serve, Client, ClientConfig, ClientError, ServerConfig},
    models::{default_levels, parse_index, Context, Core, Plan},
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
    },

    /// Complete the current task
    Complete,
}

/// Run the CLI application
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve { port } => {
            println!("Starting scatterbrain API server on port {}...", port);

            // Create a default plan with the default levels
            let plan = Plan::new(default_levels());
            let context = Context::new(plan);
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

            match command {
                TaskCommands::Add { description } => {
                    let index = client.add_task(description.clone()).await?;
                    println!("Added task: \"{}\" at index: {:?}", description, index);
                    Ok(())
                }

                TaskCommands::Complete => {
                    client.complete_task().await?;
                    println!("Completed the current task");
                    Ok(())
                }
            }
        }

        Commands::Move { index } => {
            let client = create_client(&cli.server);

            // Parse the index string (format: 0 or 0,1,2)
            let parsed_index = parse_index(index)?;

            client.move_to(parsed_index).await?;
            println!("Moved to task at index: {}", index);
            Ok(())
        }

        Commands::Plan => {
            let client = create_client(&cli.server);

            let plan = client.get_plan().await?;
            print_plan(&plan);
            Ok(())
        }

        Commands::Current => {
            let client = create_client(&cli.server);

            match client.get_current().await {
                Ok(current) => {
                    println!("Current Task:");
                    println!("  Description: {}", current.task.description);
                    println!("  Completed: {}", current.task.completed);
                    println!("  Level: {}", current.level.description);
                    println!("  Index: {:?}", current.index);

                    if !current.task.subtasks.is_empty() {
                        println!("\nSubtasks:");
                        for (i, subtask) in current.task.subtasks.iter().enumerate() {
                            println!(
                                "  {}. {} (completed: {})",
                                i, subtask.description, subtask.completed
                            );
                        }
                    }

                    Ok(())
                }
                Err(ClientError::Api(msg)) if msg.contains("Current task not found") => {
                    println!("No current task selected. Use 'move' to select a task.");
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        }

        Commands::Guide => {
            print_guide();
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

fn print_plan(plan: &crate::models::Plan) {
    println!("Scatterbrain Plan:");
    println!("Levels: {}", plan.levels.len());

    println!("\nRoot Tasks:");
    if plan.root.subtasks.is_empty() {
        println!("  No tasks yet. Add some with 'scatterbrain task add'");
    } else {
        for (i, task) in plan.root.subtasks.iter().enumerate() {
            println!(
                "  {}. {} (completed: {})",
                i, task.description, task.completed
            );

            // Print first level of subtasks if any
            if !task.subtasks.is_empty() {
                for (j, subtask) in task.subtasks.iter().enumerate() {
                    println!(
                        "    {}.{}. {} (completed: {})",
                        i, j, subtask.description, subtask.completed
                    );
                }
            }
        }
    }

    println!("\nAvailable Levels:");
    for (i, level) in plan.levels.iter().enumerate() {
        println!("  {}. {}", i + 1, level.description);
    }
}

fn print_guide() {
    let guide = r#"
=== SCATTERBRAIN GUIDE ===

Scatterbrain is a hierarchical planning and task management tool designed to help agents
systematically work through complex projects by breaking them down into manageable tasks.

== CONCEPTUAL MODEL ==

Scatterbrain uses a multi-level approach to planning:

1. High-level planning: Identifying architecture, scope, and approach
   - Focus on simplicity, extensibility, and good abstractions
   - Set the overall direction and boundaries of your project

2. Isolation: Breaking down the plan into discrete, independent parts
   - Ensure each part can be completed and verified independently
   - Create modular boundaries between pieces

3. Ordering: Sequencing the parts in a logical flow
   - Start with foundational building blocks
   - Progress toward more complex concepts
   - Follow idiomatic patterns for the domain

4. Implementation: Converting each part into specific, actionable tasks
   - Make tasks independently completable
   - Ensure tasks build upon each other
   - Minimize execution risk between tasks

== WORKFLOW FOR AGENTS ==

1. START THE SERVER
   $ scatterbrain serve

2. CREATE A PLAN AND NAVIGATE THE LEVELS
   - Begin at Level 1 with high-level planning:
     $ scatterbrain task add "Design system architecture"
     $ scatterbrain move 0
     
   - Add subtasks at Level 2 to break down the approach:
     $ scatterbrain task add "Identify core components"
     $ scatterbrain move 0,0
     
   - Continue adding more granular tasks at deeper levels

3. STAY ON TRACK
   - Regularly review your plan:
     $ scatterbrain plan
     
   - Focus on your current task:
     $ scatterbrain current
     
   - Complete tasks when finished:
     $ scatterbrain task complete
     
   - Move between tasks to adapt to changing priorities:
     $ scatterbrain move 1,2

4. PROGRESSIVE REFINEMENT
   - Start with broad strokes at Level 1
   - Refine details as you move to deeper levels
   - Complete higher-level tasks only when all subtasks are done
   - Use completed tasks to validate your approach

== AGENT PRODUCTIVITY TECHNIQUES ==

1. FOCUS MANAGEMENT
   - Work on one task at a time
   - Use 'current' to maintain context between sessions
   - Complete the current task before moving to another

2. STRUCTURED THINKING
   - Use Level 1 for "why" questions
   - Use Level 2 for "what" questions
   - Use Level 3 for "when" questions
   - Use Level 4 for "how" questions

3. ADAPTIVE PLANNING
   - Revisit and adjust higher levels when assumptions change
   - Add new tasks as you discover them
   - Move between different branches as needed

4. PROGRESS TRACKING
   - Mark tasks as complete to see visible progress
   - Use the plan view to identify stuck areas
   - Balance work across different branches of the plan

== COMMAND REFERENCE ==

SERVER OPERATIONS:
  $ scatterbrain serve [--port <PORT>]     Start the server

TASK MANAGEMENT:
  $ scatterbrain task add "Task description"    Create new task
  $ scatterbrain task complete                  Complete current task
  
NAVIGATION:
  $ scatterbrain move <INDEX>                   Navigate to a task
                                               (e.g., 0 or 0,1,2)
VIEWING:
  $ scatterbrain plan                           View the full plan
  $ scatterbrain current                        View current task
  
HELP:
  $ scatterbrain guide                          Show this guide
  $ scatterbrain <COMMAND> --help               Show command help

== TIPS FOR AGENTS ==

- When stuck, move up a level and reconsider your approach
- Keep tasks small and focused for easier tracking
- Use consistent naming patterns for related tasks
- Review completed tasks to learn what works
- Balance breadth vs. depth in your planning
"#;

    println!("{}", guide);
}
