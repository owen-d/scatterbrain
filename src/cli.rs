//! CLI module
//!
//! This module provides the command-line interface functionality for the scatterbrain tool.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io; // Import env module // Import the Colorize trait

use crate::{
    api::{serve, Client, ClientConfig, ClientError, ServerConfig},
    models::{parse_index, Core, Current, PlanError, PlanId, DEFAULT_PLAN_ID},
};

// Define the constant here
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

        /// Optional notes for the task
        #[arg(long)]
        notes: String,
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

    /// Manage notes for a specific task
    Notes {
        #[command(subcommand)]
        command: TaskNotesSubcommand,
    },
}

// Define TaskNotesSubcommand Enum
#[derive(Subcommand)]
enum TaskNotesSubcommand {
    /// View notes for a task
    View {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },
    /// Set notes for a task
    Set {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
        /// The notes content
        notes: String,
    },
    /// Delete notes for a task
    Delete {
        /// Task index (e.g., 0 or 0,1,2 for nested tasks)
        index: String,
    },
}

// Define PlanCommands Enum
#[derive(Subcommand)]
enum PlanCommands {
    /// Create a new plan from a prompt and prints its ID and usage guide.
    #[command(arg_required_else_help = true)] // Require the prompt argument
    Create {
        /// The initial high-level goal or prompt for the plan
        #[arg(index = 1)]
        prompt: String,
        /// Optional longer-form notes or description for the plan
        #[arg(long)] // Add the optional notes argument
        notes: Option<String>,
    },
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
                println!("Populating with example task tree for UI testing...");
                create_example_tasks(&core);
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
                TaskCommands::Add {
                    description,
                    level,
                    notes,
                } => {
                    // Pass id.value() and notes.clone() to client method
                    let response = client
                        .add_task(id.value(), description.clone(), *level, Some(notes.clone()))
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

                TaskCommands::Notes { command } => {
                    match command {
                        TaskNotesSubcommand::View { index } => {
                            let parsed_index = parse_index(index)?;
                            // Call client.get_task_notes directly
                            match client.get_task_notes(id.value(), parsed_index).await {
                                Ok(notes_opt) => {
                                    if let Some(notes) = notes_opt {
                                        println!("Notes for task at index {}:\n{}", index, notes);
                                    } else {
                                        println!("No notes found for task at index {}.", index);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error getting notes: {}", e);
                                }
                            }
                            Ok(())
                        }
                        TaskNotesSubcommand::Set { index, notes } => {
                            let parsed_index = parse_index(index)?;
                            let response = client
                                .set_task_notes(id.value(), parsed_index, notes.clone())
                                .await?;
                            // Handle the Result<(), String> within PlanResponse
                            print_response(&response, |res| match res {
                                Ok(_) => {
                                    println!("Notes for task at index {} set successfully.", index)
                                }
                                Err(e) => {
                                    eprintln!("Error setting notes for task {}: {}", index, e)
                                }
                            });
                            Ok(())
                        }
                        TaskNotesSubcommand::Delete { index } => {
                            let parsed_index = parse_index(index)?;
                            let response =
                                client.delete_task_notes(id.value(), parsed_index).await?;
                            // Handle the Result<(), String> within PlanResponse
                            print_response(&response, |res| match res {
                                Ok(_) => println!(
                                    "Notes for task at index {} deleted successfully.",
                                    index
                                ),
                                Err(e) => {
                                    eprintln!("Error deleting notes for task {}: {}", index, e)
                                }
                            });
                            Ok(())
                        }
                    }
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
                PlanCommands::Create { prompt, notes } => {
                    // Pass the prompt and notes to the updated client method
                    match client
                        .create_plan(Some(prompt.clone()), notes.clone())
                        .await
                    {
                        Ok(lease) => {
                            let new_id = lease.value(); // lease is PlanId
                            println!("Created new plan with ID: {}", new_id);
                            println!(
                                "\nIMPORTANT: Set the environment variable to work with this plan:"
                            );
                            println!("  export {}={}", PLAN_ID_ENV_VAR, new_id);
                            println!("\n---");
                            println!("You will update scatterbrain after each point of progress or new discovery. This includes initial surveying of relevant code, formulating a plan of attack, successful additions, realizations that you've hit a dead end, etc. Do not stray from this path; it is important to use the plan to drive your efforts and keep it as an auditable record. This will also help you discover patterns in your approach and improve your ability to reason about the problem space.");
                            println!("Do not stop unless you have questions you can't reasonably answer yourself. Use your best judgment, do not ask for my approval. Continue using scatterbrain to refine and adjust your approach and move forward with solving our goal.");
                            println!("---\n");
                            print_guide(); // Print the full guide
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

    // Print notes if they exist
    if let Some(notes) = task.notes() {
        let notes_indent = "  ".repeat(index.len() + 1); // Extra indent for notes
        println!(
            "{}{}",
            notes_indent,
            notes.replace('\n', &format!("\n{}", notes_indent))
        ); // Indent multi-line notes
    }

    for (i, subtask) in task.subtasks().iter().enumerate() {
        let mut subtask_index = index.clone();
        subtask_index.push(i);
        print_task(subtask, subtask_index);
    }
}

/// Generates the guide string with formatted values.
fn get_guide_string() -> String {
    format!(
        r#"
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

1. CREATE A PLAN FROM A PROMPT:
   $ scatterbrain plan create "My new project goal"
   > Created new plan with ID: 42
   > Plan 42 created with goal: "My new project goal"
   > Use 'export {}=42' to set this plan as default for your session.
   > --- Scatterbrain Guide ---
   > (The rest of this guide will be printed here)
   > --------------------------

2. SPECIFY THE ACTIVE PLAN:
   You MUST tell scatterbrain which plan to use in one of two ways:

   a) ENVIRONMENT VARIABLE (Recommended for sessions):
      $ export {}={}
      $ scatterbrain current  # Now works with plan 42

   b) --plan FLAG (Overrides env var for a single command):
      $ scatterbrain --plan={} current

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
  $ scatterbrain plan create "<prompt>"                  Create a new plan from a prompt, print its ID and the guide
  $ scatterbrain plan delete <id>                        Delete a plan by its ID
  $ scatterbrain plan list                               List available plan IDs
  $ scatterbrain plan set <id>                           (Info only) Shows how to set the environment variable
  $ scatterbrain plan show                               View the full plan with all tasks

TASK MANAGEMENT (scatterbrain task ...):
  $ scatterbrain task add --level <LEVEL> --notes <TEXT> "Description" Create new task (level required, notes required)
                                                         Note: Adding a subtask marks parents incomplete.
  $ scatterbrain task complete --index <INDEX> [--lease <ID>] [--force] [--summary <TEXT>] Complete task at specified index (summary required unless --force)
  $ scatterbrain task change-level <LEVEL_INDEX>         Change current task's abstraction level
  $ scatterbrain task lease <INDEX>                      Generate a lease for a task
  $ scatterbrain task remove <INDEX>                     Remove a task by its index (e.g., 0,1,2)
  $ scatterbrain task uncomplete <INDEX>                 Uncomplete a task by its index
  $ scatterbrain task notes view <INDEX>                 View notes for a specific task
  $ scatterbrain task notes set <INDEX> "<NOTES>"        Set notes for a specific task
  $ scatterbrain task notes delete <INDEX>               Delete notes for a specific task

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
  • Use `distilled` to stay focused on the current context.
  • Regularly review your plan and adjust as needed.
  • Use `move` to navigate between tasks and levels.
  • Use `complete` to mark tasks as done.
  • Use `uncomplete` to reopen tasks.
  • Use `remove` to remove tasks that are no longer needed.
  • Use `change-level` to adjust the abstraction level of a task.
  • Use `lease` to ensure exclusive access to tasks.

COMMON MISTAKES TO AVOID:
  • Forgetting to set {}={} or use --plan=<id>.
  • Premature implementation detail: Diving into code specifics at Level 0
  • Inconsistent abstractions: Mixing high-level and low-level concerns
  • Ignoring dependencies: Assuming tasks can be completed in any order
  • Neglecting to validate: Assuming completed tasks are correct
  • Over-complicating: Adding unnecessary complexity to the plan
  • Under-planning: Skipping important steps in the planning process
"#,
        PLAN_ID_ENV_VAR, // 1. For 'export {}=42' example (line ~606)
        PLAN_ID_ENV_VAR, // 2. For 'export {}={}' explanation (line ~615a)
        "<id>",          // 3. For 'export {}={}' explanation (line ~615a)
        "<id>",          // 4. For '--plan={}' explanation (line ~619b)
        PLAN_ID_ENV_VAR, // 5. For '(Ensure {}={} is set' in Workflow (line ~687)
        "<id>",          // 6. For '(Ensure {}={} is set' in Workflow (line ~687)
        PLAN_ID_ENV_VAR, // 7. For 'Use `export {}=<id>`' in Best Practices (line ~775)
        PLAN_ID_ENV_VAR, // 8. For 'Forgetting to set {}={}' in Mistakes (line ~792)
        "<id>"           // 9. For 'Forgetting to set {}={}' in Mistakes (line ~792)
    )
}

fn print_guide() {
    let guide_text = get_guide_string();
    println!("{}", guide_text);
}

/// Print a distilled context from any PlanResponse
fn print_distilled_context_response<T>(response: &crate::models::PlanResponse<T>) {
    let context = &response.distilled_context;

    println!("\n--- Current Context ---");

    // Find the current node in the tree to get its index string
    let current_node_opt = find_current_node(&context.task_tree);

    // Print the overall plan goal if it exists
    if let Some(goal) = &context.goal {
        println!("Goal: {}", goal.bright_blue());
    }

    // Current Task/Level Info
    if let Some(task) = &context.current_task {
        print!(
            "Current Task: [{}] {}",
            current_node_opt.map_or_else(|| "?".to_string(), |node| format_index(&node.index)), // Get index from tree node
            task.description()
        );
        if let Some(level) = task.level_index() {
            print!(" (level: {})", level);
        }
        println!();
    } else {
        println!("No current task selected");
    }

    if let Some(level_info) = &context.current_level {
        // Find the index of this level in the main levels list
        let level_index = context
            .levels
            .iter()
            .position(|l| l.name() == level_info.name());
        if let Some(idx) = level_index {
            println!(
                "CURRENT LEVEL DETAILS (Level {}: {}):",
                idx,
                level_info.name()
            );
            println!("  Focus: {}", level_info.abstraction_focus());

            let questions = level_info.questions();
            if !questions.is_empty() {
                println!("  Sample Questions:");
                for q in questions {
                    println!("    - {}", q);
                }
            }

            println!("  Guidance: {}", level_info.get_guidance());
        } else {
            // Fallback if the current_level isn't found in the main list (shouldn't happen)
            println!(
                "CURRENT LEVEL DETAILS (Unknown Index: {}):",
                level_info.name()
            );
            println!("  Focus: {}", level_info.abstraction_focus());
        }
    } else {
        // This case handles when there's no specific current task, but we might be at root
        // or the current task doesn't have an explicit level set.
        // We rely on context.current_level which should be set even at root.
        println!(
            "CURRENT LEVEL DETAILS: No specific level context available for the current task."
        );
    }

    println!("\n");

    println!("TASK TREE (slim, see `plan show` for full tree):");
    // Helper function to find the current node recursively
    fn find_current_node(
        nodes: &[crate::models::TaskTreeNode],
    ) -> Option<&crate::models::TaskTreeNode> {
        for node in nodes {
            if node.is_current {
                return Some(node);
            }
            if let Some(found) = find_current_node(&node.children) {
                return Some(found);
            }
        }
        None
    }
    print_task_tree(&context.task_tree, 0);
    println!("\n");

    println!("AVAILABLE LEVELS (more level information availabe via the `plan` command):");
    let level_summary = context
        .levels
        .iter()
        .enumerate()
        .map(|(idx, level)| format!("{}:{}", idx, level.name()))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("  {}", level_summary);
    println!("\n");

    if !response.suggested_followups.is_empty() {
        println!("Suggested next steps:");
        for followup in &response.suggested_followups {
            println!("  • {}", followup);
        }
        println!("\n");
    }

    if let Some(reminder) = &response.reminder {
        println!("Reminder: {}", reminder);
        println!("\n");
    }
}

fn print_task_tree(_nodes: &[crate::models::TaskTreeNode], indent: usize) {
    for node in _nodes {
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

        // Print notes if they exist
        if let Some(notes) = &node.notes {
            let notes_indent = "  ".repeat(indent + 1); // Extra indent for notes
            println!(
                "{}> {}",
                notes_indent,
                notes.replace('\n', &format!("\n{}> ", notes_indent))
            ); // Indent multi-line notes
        }

        if !node.children.is_empty() {
            print_task_tree(&node.children, indent + 1);
        }
    }
}

/// Creates an example task tree for UI testing, operating on the default plan within the Core.
fn create_example_tasks(core: &Core) {
    // Access the context for the default plan ID
    let result: Result<Result<(), PlanError>, PlanError> =
        core.with_plan_context(&*DEFAULT_PLAN_ID, |context| {
            // Create top-level tasks (level 0 - Business Strategy)
            let result = context.add_task("Build Web Application".to_string(), 0, None);
            let (_, idx_root) = result.into_inner(); // Keep root index
            context.move_to(idx_root.clone()).inner();

            // Level 1 - Project Planning
            let result = context.add_task("Implement Frontend".to_string(), 1, None);
            let (_, idx_frontend) = result.into_inner();
            context.move_to(idx_frontend.clone()).inner();

            // Level 2 - Implementation
            let result = context.add_task("Design UI Components".to_string(), 2, None);
            let (_, idx_ui_components) = result.into_inner();
            context.move_to(idx_ui_components.clone()).inner();

            // Level 3 - Implementation Details
            let result = context.add_task("Implement User Authentication UI".to_string(), 3, None);
            let (_, idx_auth_ui) = result.into_inner();
            // -- Complete this task --
            context
                .complete_task(idx_auth_ui, None, true, Some("Auth UI done.".to_string()))
                .inner();

            // Move back up to "Implement Frontend"
            context.move_to(idx_frontend.clone()).inner();

            // Add another subtask to "Implement Frontend"
            let result = context.add_task("Set up State Management".to_string(), 2, None);
            let (_, idx_state_mgmt) = result.into_inner(); // Keep this index for final cursor

            // Move back to root
            context.move_to(idx_root.clone()).inner();

            // Add "Implement Backend" as subtask of "Build Web Application"
            let result = context.add_task("Implement Backend".to_string(), 1, None);
            let (_, idx_backend) = result.into_inner();
            context.move_to(idx_backend.clone()).inner();

            // Add backend tasks
            let result = context.add_task("Set up Database".to_string(), 2, None);
            let (_, idx_db) = result.into_inner();
            context.move_to(idx_db.clone()).inner();

            // Add some API endpoint tasks
            let result = context.add_task("Create API Endpoints".to_string(), 3, None);
            let (_, idx_api) = result.into_inner();
            // -- Complete this task --
            context
                .complete_task(
                    idx_api,
                    None,
                    true,
                    Some("Basic CRUD endpoints added.".to_string()),
                )
                .inner();

            context
                .add_task("Implement Authentication Logic".to_string(), 3, None)
                .into_inner();
            context
                .add_task("Create Data Models".to_string(), 3, None)
                .into_inner();

            // Move back to "Set up Database"
            context.move_to(idx_db.clone()).inner();

            // Add database schema tasks
            let result = context.add_task("Product Model".to_string(), 3, None);
            let (_, idx_prod_model) = result.into_inner();
            context.move_to(idx_prod_model.clone()).inner();

            // Add some fields
            context
                .add_task("Define Product Fields".to_string(), 3, None)
                .into_inner();
            context
                .add_task("Implement Relationships".to_string(), 3, None)
                .into_inner();

            // Move back to root level
            context.move_to(idx_root.clone()).inner();

            // Add a few more top level tasks
            context
                .add_task("Write Documentation".to_string(), 0, None)
                .into_inner();
            context
                .add_task("Test Application".to_string(), 0, None)
                .into_inner();

            // Set final cursor position to the incomplete "Set up State Management" task
            context.move_to(idx_state_mgmt).inner();

            Ok::<(), PlanError>(()) // Specify the full type for Ok variant
        });

    // Handle potential error from with_plan_context (e.g., PlanNotFound)
    if let Err(e) = result {
        eprintln!("Error creating example tasks: {}", e);
    }
}

// Re-add get_plan_id function definition here
fn get_plan_id(cli: &Cli) -> Result<PlanId, Box<dyn std::error::Error>> {
    if let Some(plan_id_val) = cli.plan {
        // If --plan flag is used (as u8), convert it to PlanId
        return Ok(PlanId::new(plan_id_val));
    }

    // Otherwise, check the environment variable
    let id_str = std::env::var(PLAN_ID_ENV_VAR).map_err(|_| {
        format!(
            "Error: Plan ID not specified. Use the --plan=<id> flag or set the {} environment variable (e.g., export {}=<id>). Use 'scatterbrain plan list' to see available IDs.",
            PLAN_ID_ENV_VAR, PLAN_ID_ENV_VAR
        )
    })?;

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

/// Helper function to format an index vector like [0, 1, 2] into "0.1.2"
fn format_index(index: &[usize]) -> String {
    index
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_get_guide_string_formatting() {
        let guide = get_guide_string();
        let env_var = PLAN_ID_ENV_VAR; // Use the constant value directly

        // Check specific formatted parts by searching for the final expected string
        assert!(
            guide.contains(&format!("Use 'export {}=42'", env_var)),
            "Check export example format"
        );
        assert!(
            guide.contains(&format!("export {}={}", env_var, "<id>")),
            "Check export explanation format"
        );
        assert!(
            guide.contains("$ scatterbrain --plan=<id> current"),
            "Check --plan flag example format"
        );
        assert!(
            guide.contains(&format!("(Ensure {}={} is set", env_var, "<id>")),
            "Check workflow guide format"
        );
        assert!(
            guide.contains(&format!("Use `export {}=<id>`", env_var)),
            "Check best practices format"
        );
        assert!(
            guide.contains(&format!("Forgetting to set {}={}", env_var, "<id>")),
            "Check common mistakes format"
        );

        // Check that the total number of placeholders matches the arguments provided (9)
        // This is implicitly tested by the format! macro itself, but we check key instances.
    }

    // Helper function to parse CLI args for testing
    fn try_parse_args(args: &[&str]) -> Result<Cli, clap::error::Error> {
        Cli::try_parse_from(args)
    }

    #[test]
    fn test_cli_task_notes_parsing() {
        // Test task add with notes
        let args_add = vec![
            "scatterbrain",
            "task",
            "add",
            "New task desc",
            "--level",
            "0",
            "--notes",
            "Some notes here",
        ];
        let cli_add = try_parse_args(&args_add).unwrap();
        match cli_add.command {
            Commands::Task { command } => match command {
                TaskCommands::Add {
                    description,
                    level,
                    notes,
                } => {
                    assert_eq!(description, "New task desc");
                    assert_eq!(level, 0);
                    assert_eq!(notes, "Some notes here");
                }
                _ => panic!("Expected TaskCommands::Add"),
            },
            _ => panic!("Expected Commands::Task"),
        }

        // Test task notes view
        let args_view = vec!["scatterbrain", "task", "notes", "view", "0,1"];
        let cli_view = try_parse_args(&args_view).unwrap();
        match cli_view.command {
            Commands::Task { command } => match command {
                TaskCommands::Notes { command: notes_cmd } => match notes_cmd {
                    TaskNotesSubcommand::View { index } => {
                        assert_eq!(index, "0,1");
                    }
                    _ => panic!("Expected TaskNotesSubcommand::View"),
                },
                _ => panic!("Expected TaskCommands::Notes"),
            },
            _ => panic!("Expected Commands::Task"),
        }

        // Test task notes set
        let args_set = vec![
            "scatterbrain",
            "task",
            "notes",
            "set",
            "1",
            "New notes content",
        ];
        let cli_set = try_parse_args(&args_set).unwrap();
        match cli_set.command {
            Commands::Task { command } => match command {
                TaskCommands::Notes { command: notes_cmd } => match notes_cmd {
                    TaskNotesSubcommand::Set { index, notes } => {
                        assert_eq!(index, "1");
                        assert_eq!(notes, "New notes content");
                    }
                    _ => panic!("Expected TaskNotesSubcommand::Set"),
                },
                _ => panic!("Expected TaskCommands::Notes"),
            },
            _ => panic!("Expected Commands::Task"),
        }

        // Test task notes delete
        let args_delete = vec!["scatterbrain", "task", "notes", "delete", "0,0,0"];
        let cli_delete = try_parse_args(&args_delete).unwrap();
        match cli_delete.command {
            Commands::Task { command } => match command {
                TaskCommands::Notes { command: notes_cmd } => match notes_cmd {
                    TaskNotesSubcommand::Delete { index } => {
                        assert_eq!(index, "0,0,0");
                    }
                    _ => panic!("Expected TaskNotesSubcommand::Delete"),
                },
                _ => panic!("Expected TaskCommands::Notes"),
            },
            _ => panic!("Expected Commands::Task"),
        }
    }

    // TODO: Add tests for CLI handler logic (requires mocking Client or test server)
}
