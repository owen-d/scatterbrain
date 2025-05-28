# Claude <> Scatterbrain MCP Integration Protocol

**Objective:** To ensure all development activities are driven by and tracked within the `scatterbrain` task management system via MCP. Treat `scatterbrain` as the "foreman" – it dictates the current task and context. **Do not deviate from this protocol.**

**Core Principle:** Your actions (code analysis, generation, edits, commands) MUST correspond directly to the currently active task in the `scatterbrain` plan.

**Workflow:**

1.  **Session Initialization:**
    *   At the beginning of a session, determine the active `scatterbrain` plan using `mcp_scatterbrain_list_plans` and `mcp_scatterbrain_get_plan`.
    *   If no suitable plan exists, create a new one using `mcp_scatterbrain_create_plan` with the user's goal as the prompt.
    *   If multiple plans exist, collaborate with the user to select the appropriate one.
    * Create a finalizer task to be completed at the end which verifies you've achieved your goal according to the plan. Usually this will contain steps to ensure compilation, test coverage, documentation, etc.

2.  **Determine Current Task:**
    *   Before taking *any* action, always verify the current task context using:
        *   `mcp_scatterbrain_get_current` for the immediate current task
        *   `mcp_scatterbrain_get_distilled_context` for broader context
        *   `mcp_scatterbrain_get_plan` for full plan visibility
    *   Your primary focus is the task indicated by these functions.

3.  **Task Evaluation & Breakdown:**
    *   Evaluate the current task retrieved in Step 2.
    *   **If the task is high-level (e.g., Level 0-2) and requires further refinement:** Propose breaking it down into smaller, more actionable sub-tasks using `mcp_scatterbrain_add_task`. Collaborate with the user to ensure the breakdown aligns with the `scatterbrain` abstraction levels (Architecture -> Isolation -> Ordering -> Implementation). Aim to reach Level 3/4 tasks.
    *   **If the task is actionable (typically Level 3 or 4):** Proceed to Step 4.

4.  **Execute Action:**
    *   Perform the specific action required to fulfill the *current* `scatterbrain` task (e.g., write code, edit a file, run a command, analyze output).
    *   Ensure the action *directly* contributes to completing this task.
    *   Use `mcp_scatterbrain_set_task_notes` to document progress, findings, or implementation details as you work.

5.  **Task Completion:**
    *   Upon successful completion of the action for the current task:
        *   If the task requires exclusive access or coordination, first attempt to acquire a lease using `mcp_scatterbrain_generate_lease`
        *   Mark the task as complete using `mcp_scatterbrain_complete_task`, providing a concise summary of the action taken
        *   Use `force: true` *only* as a last resort if lease/summary mechanisms fail unexpectedly and after consulting the user.

6.  **Navigation:**
    *   Use `mcp_scatterbrain_move_to` when:
        *   Explicitly instructed by the user.
        *   The current task is completed, and you are moving to the next logical task (e.g., a sibling or back to the parent).
    *   Do not jump between unrelated tasks arbitrarily. Maintain focus determined by the plan structure.

**Rules of Engagement:**

*   **Strict Adherence:** The `scatterbrain` plan is the single source of truth for what needs to be done. Do not perform actions unrelated to the current task.
*   **Context is Key:** Always use `mcp_scatterbrain_get_current` or `mcp_scatterbrain_get_distilled_context` to ground your actions.
*   **Proactive Breakdown:** If a task is too vague, initiate the breakdown process (Step 3) using `mcp_scatterbrain_add_task`.
*   **User Collaboration:** If ambiguity arises in the plan, task interpretation, or next steps, consult the user.
*   **Error Reporting:** If MCP function calls fail or the plan state seems inconsistent, report the issue immediately to the user and await guidance. Do not attempt to force inconsistent states.
*   **Documentation:** Use `mcp_scatterbrain_set_task_notes` liberally to maintain context and progress tracking.


**Goal:** Maintain a clear, traceable, and structured development process managed entirely through `scatterbrain` MCP integration.

# Structure

## CLAUDE Directory
The `CLAUDE/` directory contains language-specific advice and guidelines. Each file is named for it's content and should be read/considered accordingly. E.g. `CLAUDE/rust.md` contains advice specific to Rust projects.