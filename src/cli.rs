//! CLI module
//!
//! This module provides the command-line interface functionality for the scatterbrain tool.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::env;
use std::io; // Import env module

use crate::{
    api::{serve, Client, ClientConfig, ClientError, ServerConfig},
    models::{parse_index, Core, Current, PlanId, PlanResponse, TaskTreeNode},
};

// Environment variable for the plan token
const PLAN_ID_ENV_VAR: &str = "SCATTERBRAIN_PLAN_ID";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// API server URL
    #[arg(short, long, global = true, default_value = "http://localhost:3000")]
    server: String,

    /// Target plan ID (overrides SCATTERBRAIN_PLAN_ID env var)
    #[arg(long, global = true)]
    plan: Option<u8>,

    #[command(subcommand)]
    command: Commands,
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

    /// Plan management commands
    #[command(name = "plan", subcommand)] // Add plan subcommand
    PlanCmd(PlanCommands), // Use a different name to avoid conflict with the "Plan" viewing command
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

    /// Complete the current task or the task at the specified index
    Complete {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        #[arg(short, long)]
        index: String,

        /// The lease required to complete the task
        #[arg(long)]
        lease: Option<u8>,

        /// Force completion even if lease doesn't match
        #[arg(long, default_value_t = false)]
        force: bool,

        /// Optional summary for completing the task
        #[arg(long)]
        summary: Option<String>,
    },

    /// Change the abstraction level of the current task
    #[command(name = "change-level")]
    ChangeLevel {
        /// Level index (starting from 0)
        #[arg(help = "The level index to set (lower index = higher abstraction level)")]
        level_index: usize,
    },

    /// Generate a lease for the task at the given index
    Lease {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },

    /// Remove a task by its index
    Remove {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },

    /// Uncomplete a task by its index
    Uncomplete {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },
}

// Define PlanCommands Enum
#[derive(Subcommand)]
enum PlanCommands {
    /// Create a new plan and prints its ID (u8)
    Create,
    /// Delete a plan by its ID
    Delete {
        /// The ID (0-255) of the plan to delete
        id: u8,
    },
    /// List all available plan IDs
    List,
    /// Set the active plan ID (EXPERIMENTAL - might use env var instead)
    Set {
        /// The ID (0-255) to set as active
        id: u8,
    },
    /// Show the details of the current plan (tasks, levels)
    Show,
}

/// Run the CLI application
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve { port, example } => {
            println!("Starting scatterbrain API server on port {}...", port);

            // Core::new() now initializes the default plan
            let core = Core::new();

            // Add example tasks if requested (needs adjustment if Core API changes)
            if *example {
                println!("Populating default plan with example task tree for UI testing...");
                // Need to access the default context within core - requires Core modification or different approach
                // For now, let's skip example population if not easily doable
                // create_example_tasks(&mut context);
                eprintln!("Warning: --example flag currently only works if Core struct provides direct access to modify the default context, which may have changed.");
            }

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
            let id = get_plan_id(&cli)?; // id is PlanId

            let result = match command {
                TaskCommands::Add { description, level } => {
                    // Pass id.value() to client method
                    let response = client
                        .add_task(id.value(), description.clone(), *level)
                        .await?;
                    let (_task, index) = response.inner();
                    println!(
                        "Added task: \"{}\" with level {} at index: {:?}",
                        description, level, index
                    );
                    Ok(())
                }

                TaskCommands::Complete {
                    index,
                    lease,
                    force,
                    summary,
                } => {
                    // Determine the target index
                    let target_index = match parse_index(index) {
                        Ok(idx) => idx,
                        Err(e) => {
                            eprintln!("Error parsing index: {}", e);
                            return Err(e.into());
                        }
                    };

                    // Pass id.value() and lease (Option<u8>) to client method
                    let response = client
                        .complete_task(
                            id.value(),
                            target_index.clone(),
                            *lease,
                            *force,
                            summary.clone(),
                        )
                        .await?;

                    print_response(&response, |success| {
                        if *success {
                            let index_display = target_index
                                .iter()
                                .map(|i| i.to_string())
                                .collect::<Vec<_>>()
                                .join(",");
                            println!("Completed task at index: [{}]", index_display);
                        } else {
                            println!("Failed to complete task (lease mismatch? already complete? check server logs)");
                        }
                    });
                    Ok(())
                }

                TaskCommands::ChangeLevel { level_index } => {
                    // Get the current position for the active plan (id is PlanId)
                    let current_response = client.get_current(id.value()).await?;
                    let index = match current_response.inner().as_ref() {
                        Some(current) => current.index.clone(),
                        None => return Err("No current task selected".into()),
                    };

                    // Pass id.value() to client method
                    let response = client.change_level(id.value(), index, *level_index).await?;
                    print_response(&response, |_| {
                        println!("Changed level of current task to {}", level_index);
                    });
                    Ok(())
                }

                TaskCommands::Lease { index } => {
                    let parsed_index = parse_index(index)?;
                    // Pass id.value() to client method
                    let response = client.generate_lease(id.value(), parsed_index).await?;
                    let (lease, suggestions) = response.inner();
                    println!(
                        // Use lease.value() for printing
                        "Generated lease {} for task at index: {}",
                        lease.value(),
                        index
                    );
                    if !suggestions.is_empty() {
                        println!("\nVerification Suggestions:");
                        for suggestion in suggestions {
                            println!("- {}", suggestion);
                        }
                    }
                    Ok(())
                }

                TaskCommands::Remove { index } => {
                    let parsed_index = parse_index(index)?;
                    // Pass id.value() to client method
                    match client.remove_task(id.value(), parsed_index).await {
                        Ok(response) => {
                            // Handle the nested Result<Task, String>
                            print_response(&response, |result| match result {
                                Ok(removed_task) => println!(
                                    "Removed task: \"{}\" at index: {}",
                                    removed_task.description(),
                                    index // Use original string for display
                                ),
                                Err(e) => eprintln!(
                                    "Server error removing task at index {}: {}",
                                    index, e
                                ),
                            });
                        }
                        Err(e) => {
                            eprintln!("Client error removing task: {}", e);
                        }
                    };
                    Ok(())
                }

                TaskCommands::Uncomplete { index } => {
                    let parsed_index = parse_index(index)?;
                    // Pass id.value() to client method
                    match client.uncomplete_task(id.value(), parsed_index).await {
                        Ok(response) => {
                            print_response(&response, |result| match result {
                                Ok(true) => println!("Uncompleted task at index: {}", index),
                                Ok(false) => {
                                    println!("Task at index {} was already incomplete.", index)
                                }
                                Err(e) => eprintln!(
                                    "Server error uncompleting task at index {}: {}",
                                    index, e
                                ),
                            });
                        }
                        Err(e) => {
                            eprintln!("Client error uncompleting task: {}", e);
                        }
                    };
                    Ok(())
                }
            };
            result
        }

        Commands::Move { index } => {
            let client = create_client(&cli.server);
            let id = get_plan_id(&cli)?; // id is PlanId
            let parsed_index = parse_index(index)?;

            // Pass id.value() to client method
            let response = client.move_to(id.value(), parsed_index).await?;
            print_response(&response, |description: &Option<String>| {
                println!(
                    "Moved to task: \"{}\" at index: {}",
                    description.as_deref().unwrap_or("Unknown"),
                    index
                );
            });
            Ok(())
        }

        Commands::Current => {
            let client = create_client(&cli.server);
            let id = get_plan_id(&cli)?; // id is PlanId
            let response = client.get_current(id.value()).await?;
            print_response(&response, |current: &Option<Current>| {
                if let Some(current) = current {
                    println!("Current Task for Plan ID: {}", id.value()); // Use id.value() for display
                    println!("  Description: {}", current.task.description());
                    println!("  Completed: {}", current.task.is_completed());
                    println!("  Level: {}", current.level.description());
                    println!("  Index: {:?}", current.index);
                    if !current.task.subtasks().is_empty() {
                        println!("\nSubtasks:");
                        for (i, subtask) in current.task.subtasks().iter().enumerate() {
                            println!(
                                "  {}. {} (completed: {})",
                                i,
                                subtask.description(),
                                subtask.is_completed()
                            );
                        }
                    }
                } else {
                    println!("No current task selected in this plan. Use 'move' to select a task.");
                }
            });
            Ok(())
        }

        Commands::Distilled => {
            let client = create_client(&cli.server);
            let id = get_plan_id(&cli)?; // id is PlanId
            let response = client.get_distilled_context(id.value()).await?;
            print_distilled_context_response(&response);
            Ok(())
        }

        Commands::Guide => {
            print_guide();
            // Attempt to get context for default plan ID 0 using the flag logic
            let default_id_cli = Cli {
                command: Commands::Guide, // Placeholder
                server: cli.server.clone(),
                plan: Some(0), // Specify default ID 0
            };
            let client = create_client(&cli.server);
            match get_plan_id(&default_id_cli) {
                // Use the modified cli struct with PlanId
                Ok(default_id) => {
                    match client.get_distilled_context(default_id.value()).await {
                        // Pass PlanId
                        Ok(distilled_response) => {
                            println!(
                                "\nDistilled Context for Default Plan (ID {}):\n",
                                default_id.value()
                            );
                            print_distilled_context_response(&distilled_response);
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Could not fetch context for default plan {}: {}",
                                default_id.value(),
                                e
                            );
                        }
                    }
                }
                Err(_) => { /* Error handled by get_plan_id */ }
            }
            Ok(())
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(*shell, &mut cmd, bin_name, &mut io::stdout());
            Ok(())
        }

        Commands::PlanCmd(plan_command) => {
            let client = create_client(&cli.server);
            match plan_command {
                PlanCommands::Create => {
                    match client.create_plan().await {
                        Ok(lease) => {
                            let new_id = lease.value(); // lease is PlanId
                            println!("Created new plan with ID: {}", new_id);
                            println!(
                                "Set the environment variable: export {}={}",
                                PLAN_ID_ENV_VAR, new_id
                            );
                        }
                        Err(e) => eprintln!("Error creating plan: {}", e),
                    }
                    Ok(())
                }
                PlanCommands::Delete { id } => {
                    // id is u8, convert to PlanId
                    let _plan_id_to_delete = PlanId::new(*id);
                    // Pass the u8 value *id* to client.delete_plan
                    match client.delete_plan(*id).await {
                        Ok(_) => println!("Deleted plan with ID: {}", id),
                        Err(ClientError::PlanNotFound(_)) => {
                            eprintln!("Error: Plan with ID '{}' not found.", id)
                        }
                        Err(e) => eprintln!("Error deleting plan '{}': {}", id, e),
                    }
                    Ok(())
                }
                PlanCommands::List => {
                    match client.list_plans().await {
                        Ok(ids) => {
                            println!("Available plan IDs:");
                            if ids.is_empty() {
                                println!("  (No plans found - use 'plan create' to start)");
                            } else {
                                for lease in ids {
                                    // lease is PlanId
                                    println!("  - {}", lease.value());
                                }
                            }
                        }
                        Err(e) => eprintln!("Error listing plans: {}", e),
                    }
                    Ok(())
                }
                PlanCommands::Set { id } => {
                    println!("To set the active plan, use your shell's command:");
                    println!("  export {}={}", PLAN_ID_ENV_VAR, id);
                    println!("Note: This only affects the current shell session.");
                    Ok(())
                }
                PlanCommands::Show => {
                    // Handler for Show
                    let client = create_client(&cli.server);
                    let id = get_plan_id(&cli)?; // id is PlanId
                    let response = client.get_plan(id.value()).await?;
                    print_plan_response(&response);
                    Ok(())
                }
            }
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
    print_inner(response.inner());
    if !response.suggested_followups.is_empty() {
        println!("\nSuggested next steps:");
        for suggestion in &response.suggested_followups {
            println!("  • {}", suggestion);
        }
    }
    if let Some(reminder) = &response.reminder {
        println!("\nReminder: {}", reminder);
    }
    print_distilled_context_response(response);
}

fn print_plan_response(response: &crate::models::PlanResponse<crate::models::Plan>) {
    let plan = response.inner();
    println!("Scatterbrain Plan:");
    println!("Levels: {}", plan.levels().len());
    println!("\nRoot Tasks:");
    if plan.root().subtasks().is_empty() {
        println!("  No tasks yet. Add some with 'scatterbrain task add'");
    } else {
        for (i, task) in plan.root().subtasks().iter().enumerate() {
            print_task(task, vec![i]);
        }
    }
    println!("\nAvailable Levels:");
    for (i, level) in plan.levels().iter().enumerate() {
        println!("  {}. {}", i, level.get_guidance());
    }
    print_distilled_context_response(response);
}

/// Recursively prints a task and its subtasks with proper indentation
fn print_task(task: &crate::models::Task, index: Vec<usize>) {
    let indent = "  ".repeat(index.len());
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

    println!(
        "{}(index: [{}]) ({}), {} (completed: {})",
        indent,
        index_str,
        level_str,
        task.description(),
        task.is_completed()
    );

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
- Manage multiple, separate plans simultaneously

== GETTING STARTED: PLANS ==

Scatterbrain organizes work into separate "plans". Each command needs to know which plan you're working on.

1. CREATE A PLAN:
   $ scatterbrain plan create
   > Created new plan with ID: 42
   > Set the environment variable: export {}={}

2. SPECIFY THE ACTIVE PLAN:
   You MUST tell scatterbrain which plan to use in one of two ways:
   
   a) ENVIRONMENT VARIABLE (Recommended for sessions):
      $ export {}={}
      $ scatterbrain current  # Now works with plan 42

   b) --plan FLAG (Overrides env var for a single command):
      $ scatterbrain --plan=42 current

3. LIST PLANS:
   $ scatterbrain plan list

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

(Ensure {}={} is set or use --plan=<id> for each command)

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
     $ scatterbrain plan show

   - Focus on your current task:
     $ scatterbrain current

   - Get a distilled context:
     $ scatterbrain distilled

   - Complete tasks when finished:
     $ scatterbrain task complete --summary "Implemented the feature"

   - Complete tasks requiring a lease:
     Some tasks require a 'lease' token for completion, ensuring only one agent
     attempts completion at a time.
     1. Generate the lease for the task:
        $ scatterbrain task lease <INDEX>  # e.g., scatterbrain task lease 0,1,2
        > Generated lease 123 for task at index: 0,1,2
     2. Complete the task using the generated lease ID and provide a summary:
        $ scatterbrain task complete --lease 123 --summary "Completed task with lease"

     Note: If the lease doesn't match, completion will fail unless you use --force.
     Using --force bypasses both lease and summary checks; use it sparingly.
     $ scatterbrain task complete --force

   - Move between tasks to adapt to changing priorities:
     $ scatterbrain move 1,2

3. PROGRESSIVE REFINEMENT
   - Start with broad strokes at Level 0
   - Refine details as you move to deeper levels
   - Complete higher-level tasks only when all subtasks are done
   - Use completed tasks to validate your approach

== COMMAND REFERENCE ==

GLOBAL FLAGS:
  --plan=<id>                                            Specify the plan ID for this command (overrides env var)
  --server=<url>                                         Specify the server URL (default: http://localhost:3000)

PLAN MANAGEMENT (scatterbrain plan ...):
  $ scatterbrain plan create                             Create a new plan and print its ID
  $ scatterbrain plan delete <id>                        Delete a plan by its ID
  $ scatterbrain plan list                               List available plan IDs
  $ scatterbrain plan set <id>                           (Info only) Shows how to set the environment variable
  $ scatterbrain plan show                               View the full plan with all tasks

TASK MANAGEMENT (scatterbrain task ...):
  $ scatterbrain task add --level <LEVEL> "Description"  Create new task (level required)
                                                         Note: Adding a subtask marks parents incomplete.
  $ scatterbrain task complete --index <INDEX> [--lease <ID>] [--force] [--summary <TEXT>] Complete task at specified index (summary required unless --force)
  $ scatterbrain task change-level <LEVEL_INDEX>         Change current task's abstraction level
  $ scatterbrain task lease <INDEX>                      Generate a lease for a task
  $ scatterbrain task remove <INDEX>                     Remove a task by its index (e.g., 0,1,2)
  $ scatterbrain task uncomplete <INDEX>                 Uncomplete a task by its index

NAVIGATION & VIEWING (scatterbrain ...):
  $ scatterbrain move <INDEX>                            Navigate to a task (e.g., 0 or 0,1,2)
  $ scatterbrain current                                 View details of the current task
  $ scatterbrain distilled                               View a distilled context of your plan

SERVER MANAGEMENT (scatterbrain serve ...):
  $ scatterbrain serve                                   Start API server (default port 3000)
  $ scatterbrain serve --port <PORT>                     Start API server on a custom port
  $ scatterbrain serve --example                         Start with example task tree (plan ID 0)

HELP & UTILITIES (scatterbrain ...):
  $ scatterbrain guide                                   Show this guide
  $ scatterbrain completions <SHELL>                     Generate shell completions
  $ scatterbrain <COMMAND> --help                        Show help for a specific command

== BEST PRACTICES ==

PLAN MANAGEMENT:
  • Use `export {}=<id>` for most work within a shell session.
  • Use `--plan=<id>` for one-off commands targeting a different plan.
  • Regularly use `plan list` to see available plans.

PRODUCTIVITY TECHNIQUES:
  • Focus on one task at a time
  • Use 'current' and 'distilled' to maintain context
  • Complete tasks before moving to another (providing a summary!)
  • Revisit higher levels when assumptions change

LEVEL USAGE:
  • Use Level 0 for "why" questions
  • Use Level 1 for "what" questions
  • Use Level 2 for "when" questions
  • Use Level 3 for "how" questions

COMMON MISTAKES TO AVOID:
  • Forgetting to set {}={} or use --plan=<id>.
  • Premature implementation detail: Diving into code specifics at Level 0
  • Inconsistent abstractions: Mixing high-level and low-level concerns
  • Abstraction resistance: Staying too high-level when details are needed
  • Abstraction abandonment: Getting lost in details and forgetting the big picture
  • Level skipping: Jumping from Level 0 to Level 3 without proper planning
  • Forcing completion: Overusing --force bypasses safety checks and summary requirements.

== TIPS ==

- When stuck, move up a level and reconsider your approach
- Keep tasks small and focused for easier tracking
- Use consistent naming patterns for related tasks
- Review completed tasks and their summaries to learn what works
- Balance breadth vs. depth in your planning
- Recognize when to transition between levels
"#;

    println!("{}", guide);
}

/// Print a distilled context from any PlanResponse
fn print_distilled_context_response<T>(response: &PlanResponse<T>) {
    let context = &response.distilled_context;

    // Removed plan_id display as it's not available in PlanResponse<T>
    println!("\n=== DISTILLED CONTEXT ===\n");

    println!("Usage Summary: {}", context.usage_summary);
    println!("");

    fn find_current_node(nodes: &[TaskTreeNode]) -> Option<&TaskTreeNode> {
        for node in nodes {
            if node.is_current {
                return Some(node);
            }
            if !node.children.is_empty() {
                if let Some(found) = find_current_node(&node.children) {
                    return Some(found);
                }
            }
        }
        None
    }

    println!("CURRENT POSITION:");
    let mut current_level_index: Option<usize> = None;

    if let Some(current_node) = find_current_node(&context.task_tree) {
        let index_str = current_node
            .index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(".");
        println!(
            "  Task: \"{}\" {}",
            current_node.description,
            if current_node.completed {
                "[✓]"
            } else {
                "[ ]"
            }
        );
        println!("  Index: {}", index_str);

        if let Some(task) = &context.current_task {
            current_level_index = task.level_index();
        }

        if current_level_index.is_none() && !current_node.index.is_empty() {
            current_level_index = Some(current_node.index.len() - 1);
        }
    } else {
        println!("  At root level (no task selected)");
        if !context.levels.is_empty() {
            current_level_index = Some(0);
        }
    }

    if let Some(idx) = current_level_index {
        if let Some(level) = context.levels.get(idx) {
            println!("  Level: {} ({})", idx, level.name());
        } else {
            println!("  Level: {} (Unknown - Index out of bounds)", idx);
            current_level_index = None;
        }
    } else {
        println!("  Level: Unknown");
    }
    println!("");

    println!("TASK TREE:");
    print_task_tree(&context.task_tree, 0);
    println!("");

    println!("AVAILABLE LEVELS (more level information availabe via the `plan` command):");
    let level_summary = context
        .levels
        .iter()
        .enumerate()
        .map(|(idx, level)| format!("{}:{}", idx, level.name()))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("  {}", level_summary);
    println!("");

    if let Some(idx) = current_level_index {
        if let Some(level) = context.levels.get(idx) {
            println!("CURRENT LEVEL DETAILS (Level {}: {}):", idx, level.name());
            println!("  Focus: {}", level.abstraction_focus());

            let questions = level.questions();
            if !questions.is_empty() {
                println!("  Sample Questions:");
                for question in questions.iter().take(2) {
                    println!("    • {}", question);
                }
                if questions.len() > 2 {
                    println!("    • ... and {} more", questions.len() - 2);
                }
            }
            println!("");
        }
    }

    if !response.suggested_followups.is_empty() {
        println!("Suggested next steps:");
        for followup in &response.suggested_followups {
            println!("  • {}", followup);
        }
        println!("");
    }

    if let Some(reminder) = &response.reminder {
        println!("Reminder: {}", reminder);
        println!("");
    }
}

fn print_task_tree(nodes: &[TaskTreeNode], indent: usize) {
    for node in nodes {
        let index_str = node
            .index
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(".");

        let indent_str = "  ".repeat(indent);

        let current_indicator = if node.is_current { "→ " } else { "  " };

        let completion_status = if node.completed { "[✓]" } else { "[ ]" };

        println!(
            "{}{}{} {} {}",
            indent_str, current_indicator, completion_status, index_str, node.description
        );

        if !node.children.is_empty() {
            print_task_tree(&node.children, indent + 1);
        }
    }
}

fn get_plan_id(cli: &Cli) -> Result<PlanId, Box<dyn std::error::Error>> {
    if let Some(plan_id_val) = cli.plan {
        // If --plan flag is used (as u8), convert it to PlanId
        return Ok(PlanId::new(plan_id_val));
    }

    // Otherwise, check the environment variable
    let id_str = env::var(PLAN_ID_ENV_VAR).map_err(|_|
        format!(
            "Error: Plan ID not specified. Use the --plan=<id> flag or set the {} environment variable (e.g., export {}=<id>). Use 'scatterbrain plan list' to see available IDs.",
            PLAN_ID_ENV_VAR, PLAN_ID_ENV_VAR
        )
    )?;

    // Parse the env var string to u8
    let id_val = id_str.parse::<u8>().map_err(|e| {
        format!(
            "Invalid value in {}: '{}'. Must be a number between 0 and 255. Error: {}",
            PLAN_ID_ENV_VAR, id_str, e
        )
    })?;

    // Convert u8 to PlanId and return
    Ok(PlanId::new(id_val))
}
