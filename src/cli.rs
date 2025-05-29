//! CLI module
//!
//! This module provides the command-line interface functionality for the scatterbrain tool.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io; // Import env module // Import the Colorize trait

use crate::{
    api::{
        serve, Client, ClientConfig, ClientError, HttpClientImpl, ScatterbrainMcpServer,
        ServerConfig,
    },
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

    /// Start the scatterbrain MCP server
    Mcp {
        /// Populate with example task tree for testing
        #[arg(long)]
        example: bool,

        /// Optionally expose HTTP API server on the specified port
        #[arg(long)]
        expose: Option<u16>,
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
            println!("Starting scatterbrain API server on port {port}...");

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

        Commands::Mcp { example, expose } => {
            println!("Starting scatterbrain MCP server...");

            // Core::new() now initializes the default plan
            let core = Core::new();

            // Add example tasks if requested
            if *example {
                println!("Populating with example task tree for testing...");
                // Create a default plan first
                match core.create_plan(
                    "Example MCP Plan".to_string(),
                    Some("Example plan for testing MCP server functionality".to_string()),
                ) {
                    Ok(plan_id) => {
                        create_example_tasks_for_plan(&core, &plan_id);
                    }
                    Err(e) => {
                        eprintln!("Error creating default plan: {e}");
                    }
                }
            }

            // Create the MCP server
            let mcp_server = ScatterbrainMcpServer::new(core.clone());

            // If expose flag is provided, start HTTP server concurrently
            if let Some(port) = expose {
                println!("Also exposing HTTP API server on port {port}...");

                // Create server configuration
                let config = ServerConfig {
                    address: ([127, 0, 0, 1], *port).into(),
                };

                // Start both servers concurrently
                use rmcp::{transport::io::stdio, ServiceExt};
                let mcp_service = mcp_server.serve(stdio());
                let http_server = serve(core, config);

                println!("MCP server started with HTTP API exposed on port {port}. Waiting for connections...");

                // Run both servers concurrently
                let mcp_handle = tokio::spawn(async move {
                    match mcp_service.await {
                        Ok(service) => service
                            .waiting()
                            .await
                            .map(|_| ())
                            .map_err(|e| format!("MCP service error: {e}")),
                        Err(e) => Err(format!("MCP server error: {e}")),
                    }
                });

                let http_handle = tokio::spawn(async move {
                    http_server
                        .await
                        .map_err(|e| format!("HTTP server error: {e}"))
                });

                // Wait for either server to complete (or fail)
                tokio::select! {
                    result = mcp_handle => {
                        match result {
                            Ok(Ok(())) => println!("MCP server completed successfully"),
                            Ok(Err(e)) => eprintln!("MCP server error: {e}"),
                            Err(e) => eprintln!("MCP server task error: {e}"),
                        }
                    }
                    result = http_handle => {
                        match result {
                            Ok(Ok(())) => println!("HTTP server completed successfully"),
                            Ok(Err(e)) => eprintln!("HTTP server error: {e}"),
                            Err(e) => eprintln!("HTTP server task error: {e}"),
                        }
                    }
                }
            } else {
                // Start only the MCP server with stdio transport
                use rmcp::{transport::io::stdio, ServiceExt};
                let service = mcp_server.serve(stdio()).await?;

                println!("MCP server started. Waiting for connections...");
                service.waiting().await?;
            }
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
                        "Added task: \"{description}\" with level {level} at index: {index:?}"
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
                            eprintln!("Error parsing index: {e}");
                            return Err(e);
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
                            println!("Completed task at index: [{index_display}]");
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
                        println!("Changed level of current task to {level_index}");
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
                            println!("- {suggestion}");
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
                                Err(e) => {
                                    eprintln!("Server error removing task at index {index}: {e}")
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Client error removing task: {e}");
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
                                Ok(true) => println!("Uncompleted task at index: {index}"),
                                Ok(false) => {
                                    println!("Task at index {index} was already incomplete.")
                                }
                                Err(e) => eprintln!(
                                    "Server error uncompleting task at index {index}: {e}"
                                ),
                            });
                        }
                        Err(e) => {
                            eprintln!("Client error uncompleting task: {e}");
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
                                        println!("Notes for task at index {index}:\n{notes}");
                                    } else {
                                        println!("No notes found for task at index {index}.");
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error getting notes: {e}");
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
                                    println!("Notes for task at index {index} set successfully.")
                                }
                                Err(e) => {
                                    eprintln!("Error setting notes for task {index}: {e}")
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
                                    "Notes for task at index {index} deleted successfully."
                                ),
                                Err(e) => {
                                    eprintln!("Error deleting notes for task {index}: {e}")
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
                    match client.create_plan(prompt.clone(), notes.clone()).await {
                        Ok(lease) => {
                            let new_id = lease.value(); // lease is PlanId
                            println!("Created new plan with ID: {new_id}");
                            println!(
                                "\nIMPORTANT: Set the environment variable to work with this plan:"
                            );
                            println!("  export {PLAN_ID_ENV_VAR}={new_id}");
                            println!("\n---");
                            println!("You will update scatterbrain after each point of progress or new discovery. This includes initial surveying of relevant code, formulating a plan of attack, successful additions, realizations that you've hit a dead end, etc. Do not stray from this path; it is important to use the plan to drive your efforts and keep it as an auditable record. This will also help you discover patterns in your approach and improve your ability to reason about the problem space.");
                            println!("Do not stop unless you have questions you can't reasonably answer yourself. Use your best judgment, do not ask for my approval. Continue using scatterbrain to refine and adjust your approach and move forward with solving our goal.");
                            println!("---\n");
                            print_guide(); // Print the full guide
                        }
                        Err(e) => eprintln!("Error creating plan: {e}"),
                    }
                    Ok(())
                }
                PlanCommands::Delete { id } => {
                    // id is u8, convert to PlanId
                    let _plan_id_to_delete = PlanId::new(*id);
                    // Pass the u8 value *id* to client.delete_plan
                    match client.delete_plan(*id).await {
                        Ok(_) => println!("Deleted plan with ID: {id}"),
                        Err(ClientError::PlanNotFound(_)) => {
                            eprintln!("Error: Plan with ID '{id}' not found.")
                        }
                        Err(e) => eprintln!("Error deleting plan '{id}': {e}"),
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
                        Err(e) => eprintln!("Error listing plans: {e}"),
                    }
                    Ok(())
                }
                PlanCommands::Set { id } => {
                    println!("To set the active plan, use your shell's command:");
                    println!("  export {PLAN_ID_ENV_VAR}={id}");
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

fn create_client(server_url: &str) -> HttpClientImpl {
    let config = ClientConfig {
        base_url: server_url.to_string(),
    };
    HttpClientImpl::with_config(config)
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
            println!("  • {suggestion}");
        }
    }
    if let Some(reminder) = &response.reminder {
        println!("\nReminder: {reminder}");
    }
    print_distilled_context_response(response);
}

fn print_plan_response(response: &crate::models::PlanResponse<crate::models::Plan>) {
    let plan = response.inner();
    println!("Scatterbrain Plan:");
    // Print Goal if it exists
    if let Some(goal) = &plan.goal {
        // Access goal directly
        println!("Goal: {}", goal.bright_blue());
    }
    // Print Notes if they exist
    if let Some(notes) = &plan.notes {
        // Access notes directly
        println!("Notes:\n{notes}");
        println!("---"); // Add a separator
    }

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
        format!("level: {level_index}")
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
            notes.replace('\n', &format!("\n{notes_indent}"))
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
    crate::guide::get_guide_string(crate::guide::GuideMode::Cli)
}

fn print_guide() {
    let guide_text = get_guide_string();
    println!("{guide_text}");
}

/// Print a distilled context from any PlanResponse
fn print_distilled_context_response<T>(response: &crate::models::PlanResponse<T>) {
    let context = &response.distilled_context;
    let truncation_limit = 400;

    println!("\n--- Current Context ---");

    // Find the current node in the tree to get its index string
    let current_node_opt = find_current_node(&context.task_tree);

    // Print the overall plan goal if it exists
    if let Some(goal) = &context.goal {
        println!("Goal: {}", goal.bright_blue());
    }

    // Print Plan Notes (truncated)
    if let Some(notes) = &context.plan_notes {
        print!("Plan Notes: ");
        if notes.len() > truncation_limit {
            // Truncate and add indicator
            let truncated_notes: String = notes.chars().take(truncation_limit).collect();
            println!(
                "{}... (use 'plan show' for full notes)",
                truncated_notes.trim()
            );
        } else {
            // Print full notes if short enough
            println!("{notes}");
        }
    }

    // Current Task/Level Info
    if let Some(task) = &context.current_task {
        print!(
            "Current Task: [{}] {}",
            current_node_opt.map_or_else(|| "?".to_string(), |node| format_index(&node.index)), // Get index from tree node
            task.description()
        );
        if let Some(level) = task.level_index() {
            print!(" (level: {level})");
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
                    println!("    - {q}");
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
    println!("  {level_summary}");
    println!("\n");

    if !response.suggested_followups.is_empty() {
        println!("Suggested next steps:");
        for followup in &response.suggested_followups {
            println!("  • {followup}");
        }
        println!("\n");
    }

    if let Some(reminder) = &response.reminder {
        println!("Reminder: {reminder}");
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
                notes.replace('\n', &format!("\n{notes_indent}> "))
            ); // Indent multi-line notes
        }

        if !node.children.is_empty() {
            print_task_tree(&node.children, indent + 1);
        }
    }
}

/// Creates an example task tree for UI testing, operating on the default plan within the Core.
fn create_example_tasks(core: &Core) {
    create_example_tasks_for_plan(core, &DEFAULT_PLAN_ID);
}

/// Creates an example task tree for the specified plan ID.
fn create_example_tasks_for_plan(core: &Core, plan_id: &PlanId) {
    // Access the context for the specified plan ID
    let result: Result<Result<(), PlanError>, PlanError> =
        core.with_plan_context(plan_id, |context| {
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
        eprintln!("Error creating example tasks: {e}");
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
            "Error: Plan ID not specified. Use the --plan=<id> flag or set the {PLAN_ID_ENV_VAR} environment variable (e.g., export {PLAN_ID_ENV_VAR}=<id>). Use 'scatterbrain plan list' to see available IDs."
        )
    })?;

    // Parse the env var string to u8
    let id_val = id_str.parse::<u8>().map_err(|e| {
        format!(
            "Invalid value in {PLAN_ID_ENV_VAR}: '{id_str}'. Must be a number between 0 and 255. Error: {e}"
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
            guide.contains(&format!("Use 'export {env_var}=42'")),
            "Check export example format"
        );
        assert!(
            guide.contains(&format!("export {}={}", env_var, "42")),
            "Check export explanation format"
        );
        assert!(
            guide.contains("$ scatterbrain --plan=42 current"),
            "Check --plan flag example format"
        );
        assert!(
            guide.contains(&format!("(Ensure {}={} is set", env_var, "42")),
            "Check workflow guide format"
        );
        assert!(
            guide.contains(&format!("Use `export {env_var}=<id>`")),
            "Check best practices format"
        );

        // Check that the guide contains the main sections
        assert!(guide.contains("== OVERVIEW =="), "Check overview section");
        assert!(
            guide.contains("== ABSTRACTION LEVELS EXPLAINED =="),
            "Check abstraction levels section"
        );
        assert!(
            guide.contains("== WORKFLOW GUIDE =="),
            "Check workflow section"
        );
        assert!(
            guide.contains("== COMMAND REFERENCE =="),
            "Check command reference section"
        );
        assert!(
            guide.contains("== BEST PRACTICES =="),
            "Check best practices section"
        );
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

    #[test]
    fn test_cli_mcp_expose_flag() {
        // Test MCP command without expose flag
        let args_no_expose = vec!["scatterbrain", "mcp", "--example"];
        let cli_no_expose = try_parse_args(&args_no_expose).unwrap();
        match cli_no_expose.command {
            Commands::Mcp { example, expose } => {
                assert!(example);
                assert_eq!(expose, None);
            }
            _ => panic!("Expected Commands::Mcp"),
        }

        // Test MCP command with expose flag
        let args_with_expose = vec!["scatterbrain", "mcp", "--example", "--expose", "8080"];
        let cli_with_expose = try_parse_args(&args_with_expose).unwrap();
        match cli_with_expose.command {
            Commands::Mcp { example, expose } => {
                assert!(example);
                assert_eq!(expose, Some(8080));
            }
            _ => panic!("Expected Commands::Mcp"),
        }

        // Test MCP command with only expose flag
        let args_only_expose = vec!["scatterbrain", "mcp", "--expose", "3001"];
        let cli_only_expose = try_parse_args(&args_only_expose).unwrap();
        match cli_only_expose.command {
            Commands::Mcp { example, expose } => {
                assert!(!example);
                assert_eq!(expose, Some(3001));
            }
            _ => panic!("Expected Commands::Mcp"),
        }
    }

    // TODO: Add tests for CLI handler logic (requires mocking Client or test server)
}
