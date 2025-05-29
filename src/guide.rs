//! Guide module for generating context-appropriate help content
//!
//! This module provides a unified way to generate guide content for different interfaces
//! (CLI vs MCP) while keeping the content DRY through string interpolation.

/// Mode for guide generation
#[derive(Debug, Clone, Copy)]
pub enum GuideMode {
    /// Command-line interface mode
    Cli,
    /// Model Context Protocol mode
    Mcp,
}

/// Generate a comprehensive guide string for the specified mode
pub fn get_guide_string(mode: GuideMode) -> String {
    let config = match mode {
        GuideMode::Cli => GuideConfig::cli(),
        GuideMode::Mcp => GuideConfig::mcp(),
    };

    format!(
        r#"=== {title} ===

{overview}

{getting_started}

{abstraction_levels}

{transitioning_levels}

{workflow_guide}

{command_reference}

{best_practices}

{additional_sections}

{closing_message}"#,
        title = config.title,
        overview = get_overview_section(),
        getting_started = config.getting_started,
        abstraction_levels = get_abstraction_levels_section(),
        transitioning_levels = get_transitioning_levels_section(),
        workflow_guide = config.workflow_guide,
        command_reference = config.command_reference,
        best_practices = get_best_practices_section(&config),
        additional_sections = config.additional_sections,
        closing_message = config.closing_message
    )
}

/// Configuration for different guide modes
struct GuideConfig {
    title: &'static str,
    getting_started: String,
    workflow_guide: String,
    command_reference: String,
    additional_sections: String,
    closing_message: &'static str,
    plan_management_specifics: String,
}

impl GuideConfig {
    fn cli() -> Self {
        let env_var = "SCATTERBRAIN_PLAN_ID";

        Self {
            title: "SCATTERBRAIN GUIDE",
            getting_started: format!(
                r#"== GETTING STARTED: PLANS ==

Scatterbrain organizes work into separate "plans". Each command needs to know which plan you're working on.

1. CREATE A PLAN FROM A PROMPT:
   $ scatterbrain plan create "My new project goal" [--notes <TEXT>]
   > Created new plan with ID: 42
   > Plan 42 created with goal: "My new project goal"
   > Use 'export {env_var}=42' to set this plan as default for your session.
   > --- Scatterbrain Guide ---
   > (The rest of this guide will be printed here)
   > --------------------------
   Tip: Keep the <prompt> concise (like a title). Use the optional --notes flag
   to add more detailed descriptions, context, or acceptance criteria, especially
   for non-trivial plans.

2. SPECIFY THE ACTIVE PLAN:
   You MUST tell scatterbrain which plan to use in one of two ways:

   a) ENVIRONMENT VARIABLE (Recommended for sessions):
      $ export {env_var}=42
      $ scatterbrain current  # Now works with plan 42

   b) --plan FLAG (Overrides env var for a single command):
      $ scatterbrain --plan=42 current

3. LIST PLANS:
   $ scatterbrain plan list"#
            ),
            workflow_guide: format!(
                r#"== WORKFLOW GUIDE ==

(Ensure {env_var}=42 is set or use --plan=<id> for each command)

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
   - Use completed tasks to validate your approach"#
            ),
            command_reference: r#"== COMMAND REFERENCE ==

GLOBAL FLAGS:
  --plan=<id>                                            Specify the plan ID for this command (overrides env var)
  --server=<url>                                         Specify the server URL (default: http://localhost:3000)

PLAN MANAGEMENT (scatterbrain plan ...):
  $ scatterbrain plan create "<prompt>" [--notes <TEXT>] Create a new plan. Use a short prompt/title and add details via --notes. Prints ID and guide.
  $ scatterbrain plan delete <id>                        Delete a plan by its ID
  $ scatterbrain plan list                               List available plan IDs
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
  $ scatterbrain <COMMAND> --help                        Show help for a specific command"#.to_string(),
            additional_sections: String::new(),
            closing_message: "",
            plan_management_specifics: format!(
                r#"• Use `export {env_var}=<id>` for most work within a shell session.
  • Use `--plan=<id>` for one-off commands targeting a different plan.
  • Regularly use `plan list` to see available plans."#
            ),
        }
    }

    fn mcp() -> Self {
        Self {
            title: "SCATTERBRAIN MCP GUIDE",
            getting_started: r#"== GETTING STARTED: PLANS ==

Scatterbrain organizes work into separate "plans". Each MCP tool call needs to specify which plan you're working on via the plan_id parameter.

1. CREATE A PLAN:
   Use: mcp_scatterbrain_create_plan
   Parameters: prompt (required), notes (optional)
   Returns: Plan ID (0-255)
   
   Example: Create a plan with prompt "Build web application" and optional notes
   The plan ID returned (e.g., 42) will be used in all subsequent operations.

2. LIST AVAILABLE PLANS:
   Use: mcp_scatterbrain_list_plans
   Returns: List of all plan IDs

3. DELETE A PLAN:
   Use: mcp_scatterbrain_delete_plan
   Parameters: plan_id"#.to_string(),
            workflow_guide: r#"== WORKFLOW GUIDE ==

1. CREATE AND STRUCTURE YOUR PLAN
   a) Create a plan:
      mcp_scatterbrain_create_plan(prompt="Your project goal", notes="Optional details")
      
   b) Add high-level tasks (Level 0):
      mcp_scatterbrain_add_task(plan_id=42, description="Design system architecture", level_index=0)
      
   c) Navigate to the task:
      mcp_scatterbrain_move_to(plan_id=42, index="0")
      
   d) Add subtasks at appropriate levels:
      mcp_scatterbrain_add_task(plan_id=42, description="Identify core components", level_index=1)

2. TRACK PROGRESS AND STAY FOCUSED
   a) View your current position:
      mcp_scatterbrain_get_current(plan_id=42)
      
   b) Get distilled context:
      mcp_scatterbrain_get_distilled_context(plan_id=42)
      
   c) View the full plan:
      mcp_scatterbrain_get_plan(plan_id=42)

3. COMPLETE TASKS AND MANAGE WORKFLOW
   a) Generate a lease for task completion (when required):
      mcp_scatterbrain_generate_lease(plan_id=42, index="0,1,2")
      
   b) Complete tasks:
      mcp_scatterbrain_complete_task(plan_id=42, index="0,1,2", lease=123, summary="Task completed")
      
   c) Navigate between tasks:
      mcp_scatterbrain_move_to(plan_id=42, index="1,2")
      
   d) Change task abstraction levels:
      mcp_scatterbrain_change_level(plan_id=42, index="0,1", level_index=2)

4. MANAGE TASK NOTES
   a) Add notes to tasks:
      mcp_scatterbrain_set_task_notes(plan_id=42, index="0,1", notes="Implementation details...")
      
   b) View task notes:
      mcp_scatterbrain_get_task_notes(plan_id=42, index="0,1")
      
   c) Delete task notes:
      mcp_scatterbrain_delete_task_notes(plan_id=42, index="0,1")"#.to_string(),
            command_reference: r#"== MCP TOOL REFERENCE ==

PLAN MANAGEMENT:
  mcp_scatterbrain_create_plan(prompt, notes?)     Create a new plan with required prompt and optional notes
  mcp_scatterbrain_delete_plan(plan_id)           Delete a plan by its ID
  mcp_scatterbrain_list_plans()                   List all available plan IDs
  mcp_scatterbrain_get_plan(plan_id)              Get full plan details

NAVIGATION & VIEWING:
  mcp_scatterbrain_get_current(plan_id)           Get details of the current task
  mcp_scatterbrain_get_distilled_context(plan_id) Get distilled context of the plan
  mcp_scatterbrain_move_to(plan_id, index)        Navigate to a specific task (e.g., "0,1,2")

TASK MANAGEMENT:
  mcp_scatterbrain_add_task(plan_id, description, level_index, notes?) Create new task at specified level
  mcp_scatterbrain_complete_task(plan_id, index, lease?, force?, summary?) Complete a task
  mcp_scatterbrain_uncomplete_task(plan_id, index) Uncomplete a task
  mcp_scatterbrain_remove_task(plan_id, index)    Remove a task by its index
  mcp_scatterbrain_change_level(plan_id, index, level_index) Change task's abstraction level
  mcp_scatterbrain_generate_lease(plan_id, index) Generate a lease token for task completion

NOTES MANAGEMENT:
  mcp_scatterbrain_get_task_notes(plan_id, index) Get notes for a specific task
  mcp_scatterbrain_set_task_notes(plan_id, index, notes) Set notes for a specific task
  mcp_scatterbrain_delete_task_notes(plan_id, index) Delete notes for a specific task

HELP:
  mcp_scatterbrain_get_guide()                    Show this comprehensive guide"#.to_string(),
            additional_sections: r#"== INDEX FORMAT ==

Task indices use comma-separated format to represent the hierarchical path:
- "0" = First top-level task
- "0,1" = Second subtask of the first top-level task
- "0,1,2" = Third subtask of the second subtask of the first top-level task

== TASK COMPLETION AND LEASES ==

Some tasks may require a 'lease' token for completion, ensuring proper coordination:

1. Generate a lease:
   mcp_scatterbrain_generate_lease(plan_id=42, index="0,1,2")
   Returns: lease token (e.g., 123) and verification suggestions

2. Complete with lease:
   mcp_scatterbrain_complete_task(plan_id=42, index="0,1,2", lease=123, summary="Completed task")

3. Force completion (bypass lease and summary requirements):
   mcp_scatterbrain_complete_task(plan_id=42, index="0,1,2", force=true)

Note: Use force completion sparingly, as it bypasses important coordination mechanisms.

== GETTING HELP ==

- Use mcp_scatterbrain_get_guide() to view this guide anytime
- Use mcp_scatterbrain_get_distilled_context() to understand your current planning state
- Each tool provides detailed error messages for invalid parameters or operations"#.to_string(),
            closing_message: r#"Remember: Scatterbrain is designed to help you think systematically about complex problems.
Use the abstraction levels to guide your thinking from high-level architecture down to
specific implementation tasks."#,
            plan_management_specifics: r#"• Keep plan prompts concise but descriptive
  • Use notes for detailed requirements and context
  • Create separate plans for different projects or major features"#.to_string(),
        }
    }
}

/// Get the overview section (shared between CLI and MCP)
fn get_overview_section() -> &'static str {
    r#"Scatterbrain is a hierarchical planning and task management tool designed to help
break down complex projects into manageable tasks through multiple abstraction levels.

== OVERVIEW ==

Scatterbrain helps you:
- Structure complex tasks in a logical hierarchy
- Navigate between different levels of abstraction
- Track progress and maintain focus
- Adapt your plan as work progresses
- Manage multiple, separate plans simultaneously"#
}

/// Get the abstraction levels section (shared between CLI and MCP)
fn get_abstraction_levels_section() -> &'static str {
    r#"== ABSTRACTION LEVELS EXPLAINED ==

Scatterbrain uses a multi-level approach to planning:

Level 0 - High-level planning: Identifying architecture, scope, and approach
   - Focus on simplicity, extensibility, and good abstractions
   - Set the overall direction and boundaries of your project
   - Ask: "What's the overall architecture?" "Which approach should we take?"

Level 1 - Isolation: Breaking down the plan into discrete, independent parts
   - Define boundaries between components
   - Establish interfaces and contracts
   - Ensure each part can be completed and verified independently
   - Ask: "What are the interfaces?" "How should we divide this into parts?"

Level 2 - Ordering: Sequencing the parts in a logical flow
   - Start with foundational building blocks
   - Identify dependencies between tasks
   - Plan the critical path
   - Ask: "What order should we implement these?" "Which parts come first?"

Level 3 - Implementation: Converting each part into specific, actionable tasks
   - Define concrete, actionable steps
   - Detail exact implementation requirements
   - Make tasks independently completable
   - Ask: "What specific changes are needed?" "What files need modification?""#
}

/// Get the transitioning levels section (shared between CLI and MCP)
fn get_transitioning_levels_section() -> &'static str {
    r#"== TRANSITIONING BETWEEN LEVELS ==

MOVING DOWN (Higher to Lower Level Numbers):
  Level 0 → Level 1:
  • When your high-level approach is clear
  • When you're ready to define component boundaries
  • When you need to establish contracts between components

  Level 1 → Level 2:
  • When component boundaries are well-defined
  • When you need to determine implementation sequence
  • When you're ready to identify dependencies

  Level 2 → Level 3:
  • When the implementation sequence is clear
  • When you're ready to define specific tasks
  • When you're prepared to execute the implementation plan

MOVING UP (Lower to Higher Level Numbers):
  Level 3 → Level 2:
  • When you've completed implementation tasks
  • When you need to reorganize remaining task sequence
  • When you need to reprioritize work

  Level 2 → Level 1:
  • When you discover issues with component interfaces
  • When integration is more complex than expected
  • When you need to redefine component boundaries

  Level 1 → Level 0:
  • When you find fundamental flaws in the approach
  • When components don't form a coherent system
  • When you need to rethink the entire architecture"#
}

/// Get the best practices section (shared between CLI and MCP with mode-specific additions)
fn get_best_practices_section(config: &GuideConfig) -> String {
    format!(
        r#"== BEST PRACTICES ==

PLAN MANAGEMENT:
  {plan_specifics}

TASK ORGANIZATION:
  • Start with broad, high-level tasks (Level 0)
  • Break down into components and interfaces (Level 1)
  • Define implementation sequence (Level 2)
  • Create specific, actionable tasks (Level 3)

WORKFLOW:
  • Use current task and distilled context views regularly to stay oriented
  • Complete higher-level tasks only when all subtasks are done
  • Use task notes to capture important implementation details
  • Generate leases for critical tasks to ensure proper completion tracking

PROGRESSIVE REFINEMENT:
  • Start with broad strokes at higher abstraction levels
  • Refine details as you move to lower levels
  • Use completed tasks to validate your approach
  • Adapt the plan as you discover new requirements or constraints"#,
        plan_specifics = config.plan_management_specifics
    )
}
