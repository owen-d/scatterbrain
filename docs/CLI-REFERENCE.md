# Scatterbrain CLI Reference

**Complete command-line interface documentation for scatterbrain**

## Table of Contents

- [Overview](#overview)
- [Global Options](#global-options)
- [Environment Variables](#environment-variables)
- [Plan Management](#plan-management)
- [Task Management](#task-management)
- [Navigation & Context](#navigation--context)
- [Server Commands](#server-commands)
- [Utility Commands](#utility-commands)
- [Examples](#examples)
- [Tips & Best Practices](#tips--best-practices)

## Overview

The scatterbrain CLI provides complete access to all functionality through a hierarchical command structure. Commands are organized into logical groups for plan management, task operations, navigation, and server control.

### Command Structure

```
scatterbrain [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS] [ARGS]
```

### Getting Help

```bash
# General help
scatterbrain --help

# Command-specific help
scatterbrain plan --help
scatterbrain task --help
scatterbrain task add --help
```

## Global Options

These options apply to all commands:

### `--server <URL>`
Specify the API server URL (default: `http://localhost:3000`)

```bash
scatterbrain --server http://localhost:8080 plan list
```

### `--plan <ID>`
Override the plan ID for this command (overrides `SCATTERBRAIN_PLAN_ID` environment variable)

```bash
scatterbrain --plan 42 current
scatterbrain --plan 1 task add --level 0 "New task" --notes "Important task"
```

## Environment Variables

### `SCATTERBRAIN_PLAN_ID`
Set the default plan ID for CLI operations. Highly recommended for workflow efficiency.

```bash
export SCATTERBRAIN_PLAN_ID=1
scatterbrain current  # Uses plan 1
scatterbrain task add --level 0 "Task" --notes "Notes"  # Adds to plan 1
```

### `RUST_LOG`
Control logging verbosity:

```bash
export RUST_LOG=debug  # Verbose logging
export RUST_LOG=info   # Standard logging
export RUST_LOG=warn   # Minimal logging
```

## Plan Management

All plan management commands use the `plan` subcommand:

### `plan create <PROMPT> [--notes <TEXT>]`
Create a new plan from a high-level prompt.

```bash
# Basic plan creation
scatterbrain plan create "Build a web application"

# With detailed notes
scatterbrain plan create "Implement user authentication" \
  --notes "Requirements: JWT tokens, password hashing, email verification, role-based access control"
```

**Output**: Displays the new plan ID and prints the usage guide.

### `plan list`
List all available plans with their IDs.

```bash
scatterbrain plan list
```

**Output**:
```
Available plans:
  Plan 0: Example plan
  Plan 1: Build a web application
  Plan 2: Implement user authentication
```

### `plan show`
Display the complete structure of the current plan.

```bash
# Show current plan (from SCATTERBRAIN_PLAN_ID)
scatterbrain plan show

# Show specific plan
scatterbrain --plan 2 plan show
```

### `plan delete <ID>`
Permanently delete a plan.

```bash
scatterbrain plan delete 2
```

**⚠️ Warning**: This action cannot be undone.

## Task Management

All task operations use the `task` subcommand:

### `task add --level <LEVEL> --notes <TEXT> "<DESCRIPTION>"`
Add a new task to the current plan.

**Required Parameters**:
- `--level <LEVEL>`: Abstraction level (0-3)
- `--notes <TEXT>`: Task notes (required)
- `<DESCRIPTION>`: Task description

**Abstraction Levels**:
- **0 (Planning)**: High-level goals and architecture
- **1 (Isolation)**: Independent components and boundaries
- **2 (Ordering)**: Sequence and dependencies
- **3 (Implementation)**: Concrete, actionable tasks

```bash
# High-level architectural task
scatterbrain task add --level 0 "Design system architecture" \
  --notes "Define overall system structure, technology stack, and major components"

# Implementation task
scatterbrain task add --level 3 "Install Express.js framework" \
  --notes "npm install express, set up basic server structure"
```

### `task complete --index <INDEX> [OPTIONS]`
Mark a task as completed.

**Required Parameters**:
- `--index <INDEX>`: Task index (e.g., "0", "0,1", "0,1,2")

**Optional Parameters**:
- `--lease <ID>`: Completion lease for coordination
- `--force`: Force completion without lease or summary
- `--summary <TEXT>`: Completion summary (recommended)

```bash
# Complete with summary
scatterbrain task complete --index 0,1 \
  --summary "Successfully implemented JWT authentication with bcrypt password hashing"

# Complete with lease (for collaborative environments)
scatterbrain task complete --index 0,1,2 --lease 123 \
  --summary "Database schema created and migrated"

# Force completion (use sparingly)
scatterbrain task complete --index 0 --force
```

### `task uncomplete <INDEX>`
Mark a completed task as incomplete.

```bash
scatterbrain task uncomplete 0,1
```

### `task remove <INDEX>`
Delete a task from the plan.

```bash
scatterbrain task remove 0,1,2
```

### `task change-level <LEVEL_INDEX>`
Change the abstraction level of the current task.

```bash
scatterbrain task change-level 2
```

### `task lease <INDEX>`
Generate a coordination lease for a task.

```bash
scatterbrain task lease 0,1,2
```

**Output**: Returns a lease ID that can be used with `task complete --lease`.

### Task Notes Management

#### `task notes view <INDEX>`
View notes for a specific task.

```bash
scatterbrain task notes view 0,1
```

#### `task notes set <INDEX> "<NOTES>"`
Set or update notes for a task.

```bash
scatterbrain task notes set 0,1 "Updated requirements: add OAuth2 support"
```

#### `task notes delete <INDEX>`
Remove notes from a task.

```bash
scatterbrain task notes delete 0,1
```

## Navigation & Context

### `move <INDEX>`
Navigate to a specific task in the plan.

```bash
# Move to root level task
scatterbrain move 0

# Move to nested task
scatterbrain move 0,1,2
```

### `current`
Display details of the currently focused task.

```bash
scatterbrain current
```

**Output**: Shows task description, notes, completion status, and subtasks.

### `distilled`
Get a focused summary of the current planning state.

```bash
scatterbrain distilled
```

**Output**: Provides high-level context and current focus area.

## Server Commands

### `serve [--port <PORT>] [--example]`
Start the HTTP API server.

```bash
# Default port (3000)
scatterbrain serve

# Custom port
scatterbrain serve --port 8080

# With example data
scatterbrain serve --port 3000 --example
```

**Access**: Web UI available at `http://localhost:<PORT>`

### `mcp [--example] [--expose <PORT>]`
Start the MCP (Model Context Protocol) server.

```bash
# Basic MCP server
scatterbrain mcp

# With example data for testing
scatterbrain mcp --example

# MCP server + HTTP API on specified port
scatterbrain mcp --expose 8080
```

**Usage**: Configure AI assistants to connect to this MCP server.

## Utility Commands

### `guide`
Display the interactive usage guide.

```bash
scatterbrain guide
```

### `completions <SHELL>`
Generate shell completions.

```bash
# Bash
scatterbrain completions bash > ~/.bash_completion.d/scatterbrain

# Zsh
scatterbrain completions zsh > ~/.zsh/completions/_scatterbrain

# Fish
scatterbrain completions fish > ~/.config/fish/completions/scatterbrain.fish
```

**Supported shells**: bash, zsh, fish, powershell

## Examples

### Complete Workflow Example

```bash
# 1. Set up environment
export SCATTERBRAIN_PLAN_ID=1

# 2. Create a plan
scatterbrain plan create "Build a REST API for task management" \
  --notes "Requirements: CRUD operations, authentication, data persistence, API documentation"

# 3. Add high-level tasks
scatterbrain task add --level 0 "Design API architecture" \
  --notes "Define endpoints, data models, authentication strategy"

scatterbrain task add --level 0 "Implement core functionality" \
  --notes "Database setup, CRUD operations, business logic"

scatterbrain task add --level 0 "Add authentication & security" \
  --notes "JWT tokens, input validation, rate limiting"

scatterbrain task add --level 0 "Documentation & deployment" \
  --notes "API docs, deployment scripts, monitoring"

# 4. Navigate and refine
scatterbrain move 0  # Focus on API architecture
scatterbrain task add --level 1 "Define data models" \
  --notes "User, Task, Project entities with relationships"

scatterbrain task add --level 1 "Design REST endpoints" \
  --notes "RESTful URL structure, HTTP methods, response formats"

# 5. Add implementation details
scatterbrain move 0,0  # Focus on data models
scatterbrain task add --level 3 "Create User model" \
  --notes "Fields: id, email, password_hash, created_at, updated_at"

scatterbrain task add --level 3 "Create Task model" \
  --notes "Fields: id, title, description, status, user_id, created_at"

# 6. Track progress
scatterbrain task complete --index 0,0,0 \
  --summary "User model created with validation and password hashing"

# 7. Review status
scatterbrain current
scatterbrain distilled
```

### Multi-Plan Workflow

```bash
# Work with multiple plans
scatterbrain plan create "Frontend development"
scatterbrain plan create "Backend development"
scatterbrain plan list

# Switch between plans
export SCATTERBRAIN_PLAN_ID=1
scatterbrain task add --level 0 "Set up React app" --notes "Create React app with TypeScript"

export SCATTERBRAIN_PLAN_ID=2
scatterbrain task add --level 0 "Set up Express server" --notes "Basic Express setup with middleware"

# Or use --plan flag for one-off operations
scatterbrain --plan 1 current
scatterbrain --plan 2 current
```

## Tips & Best Practices

### Environment Setup

1. **Always set SCATTERBRAIN_PLAN_ID** for active work:
   ```bash
   export SCATTERBRAIN_PLAN_ID=1
   ```

2. **Use shell completions** for faster command entry:
   ```bash
   scatterbrain completions bash >> ~/.bashrc
   ```

3. **Set up aliases** for frequently used commands:
   ```bash
   alias sb='scatterbrain'
   alias sbc='scatterbrain current'
   alias sbd='scatterbrain distilled'
   ```

### Task Management Strategy

1. **Start broad, refine progressively**:
   - Level 0: Major project phases
   - Level 1: Independent features
   - Level 2: Ordered implementation steps
   - Level 3: Specific actions

2. **Use descriptive notes**:
   - Capture requirements and context
   - Document decisions and rationale
   - Include acceptance criteria

3. **Regular navigation**:
   - Use `move` to maintain focus
   - Use `current` to check context
   - Use `distilled` for big picture

### Collaboration

1. **Use leases for coordination**:
   ```bash
   scatterbrain task lease 0,1,2
   scatterbrain task complete --index 0,1,2 --lease 123 --summary "Completed"
   ```

2. **Provide completion summaries**:
   - Document what was accomplished
   - Note any issues or changes
   - Reference relevant commits or PRs

3. **Keep plans focused**:
   - One plan per major project or sprint
   - Archive completed plans
   - Split large plans when they become unwieldy

### Performance

1. **Use the web UI for large plans**:
   ```bash
   scatterbrain serve --port 8080
   ```

2. **Regular cleanup**:
   - Complete finished tasks
   - Remove obsolete tasks
   - Archive old plans

3. **Efficient navigation**:
   - Learn task indexing (0,1,2 format)
   - Use `current` to check position
   - Bookmark important task indices

---

**Next**: Check out the [MCP Integration Guide](MCP-GUIDE.md) for AI assistant integration or [Examples & Patterns](EXAMPLES.md) for more real-world scenarios. 