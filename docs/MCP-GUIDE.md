# Scatterbrain MCP Integration Guide

**Complete guide to using scatterbrain as a Model Context Protocol (MCP) server with AI assistants**

## Table of Contents

- [Overview](#overview)
- [Installation & Setup](#installation--setup)
- [AI Assistant Configuration](#ai-assistant-configuration)
- [Available MCP Tools](#available-mcp-tools)
- [Workflow Examples](#workflow-examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The Model Context Protocol (MCP) allows AI assistants to interact with external tools and services. Scatterbrain's MCP server exposes 17 specialized tools that enable AI assistants to:

- Create and manage hierarchical plans
- Add and organize tasks across abstraction levels
- Navigate complex project structures
- Maintain context and notes
- Track progress and completion

### Why Use Scatterbrain with AI Assistants?

- **Structured Thinking**: Break down complex problems systematically
- **Persistent Context**: Maintain planning state across conversations
- **Hierarchical Organization**: Natural progression from goals to implementation
- **Collaborative Planning**: AI assists with task breakdown and organization

## Installation & Setup

### Prerequisites

- Rust toolchain (1.70+)
- AI assistant that supports MCP (Cursor, Claude Desktop, etc.)

### Build Scatterbrain

```bash
# Clone the repository
git clone https://github.com/your-username/scatterbrain.git
cd scatterbrain

# Build the project
cargo build --release

# Optional: Install globally
cargo install --path .
```

### Verify Installation

```bash
scatterbrain --version
scatterbrain mcp --help
```

## AI Assistant Configuration

### Cursor Configuration

1. Open Cursor settings
2. Navigate to MCP configuration
3. Add scatterbrain server:

```json
{
  "mcpServers": {
    "scatterbrain": {
      "command": "scatterbrain",
      "args": ["mcp"]
    }
  }
}
```

*[Screenshot placeholder: Cursor MCP settings page]*

### Claude Desktop Configuration

1. Locate Claude configuration file:
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

2. Add scatterbrain configuration:

```json
{
  "mcpServers": {
    "scatterbrain": {
      "command": "scatterbrain",
      "args": ["mcp", "--example"]
    }
  }
}
```

*[Screenshot placeholder: Claude Desktop with scatterbrain tools visible]*

### Other MCP-Compatible Assistants

For any MCP-compatible AI assistant, use these connection details:

- **Command**: `scatterbrain`
- **Args**: `["mcp"]`
- **Optional Args**: `["mcp", "--example"]` (includes sample data)

## Available MCP Tools

Scatterbrain provides 17 MCP tools organized by functionality:

### Plan Management

#### `create_plan`
Create a new plan from a high-level prompt.

**Parameters:**
- `prompt` (string): The main goal or objective
- `notes` (optional string): Additional context or requirements

**Example:**
```
Create a plan for "Build a web application for task management"
```

#### `get_plan`
Retrieve complete plan details including all tasks and structure.

**Parameters:**
- `plan_id` (number): The plan identifier

#### `list_plans`
Get all available plans.

**Returns:** List of plan IDs and basic information

#### `delete_plan`
Remove a plan permanently.

**Parameters:**
- `plan_id` (number): The plan to delete

### Task Management

#### `add_task`
Add a new task to the current plan.

**Parameters:**
- `plan_id` (number): Target plan
- `description` (string): Task description
- `level_index` (number): Abstraction level (0-3)
- `notes` (optional string): Additional task details

**Abstraction Levels:**
- **0 (Planning)**: High-level goals and architecture
- **1 (Isolation)**: Independent components
- **2 (Ordering)**: Sequence and dependencies  
- **3 (Implementation)**: Concrete actions

#### `complete_task`
Mark a task as completed.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index (e.g., "0,1,2")
- `lease` (optional number): Completion lease for coordination
- `force` (optional boolean): Force completion without lease
- `summary` (optional string): Completion summary

#### `uncomplete_task`
Mark a completed task as incomplete.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index

#### `remove_task`
Delete a task from the plan.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index

### Navigation

#### `move_to`
Navigate to a specific task in the plan.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index to navigate to

#### `get_current`
Get details about the currently focused task.

**Parameters:**
- `plan_id` (number): Target plan

#### `get_distilled_context`
Get a focused summary of the current planning state.

**Parameters:**
- `plan_id` (number): Target plan

### Task Notes

#### `get_task_notes`
Retrieve notes for a specific task.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index

#### `set_task_notes`
Add or update notes for a task.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index
- `notes` (string): Note content

#### `delete_task_notes`
Remove notes from a task.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index

### Utility Tools

#### `change_level`
Change the abstraction level of a task.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index
- `level_index` (number): New abstraction level

#### `generate_lease`
Generate a coordination lease for task completion.

**Parameters:**
- `plan_id` (number): Target plan
- `index` (string): Task index

#### `get_guide`
Get comprehensive usage guide and help information.

## Workflow Examples

### Example 1: Creating a Software Project Plan

**Step 1: Create the plan**
```
AI: I'll help you create a plan for building a web application. Let me start by creating a plan.

[Uses create_plan with prompt: "Build a modern web application for personal task management"]
```

**Step 2: Add high-level tasks**
```
AI: Now I'll break this down into major components:

[Uses add_task with level 0: "Design system architecture"]
[Uses add_task with level 0: "Implement backend API"]
[Uses add_task with level 0: "Build frontend interface"]
[Uses add_task with level 0: "Deploy and test system"]
```

**Step 3: Refine into specific components**
```
AI: Let me focus on the backend API and break it down further:

[Uses move_to index "1"]
[Uses add_task with level 1: "Set up database schema"]
[Uses add_task with level 1: "Implement authentication"]
[Uses add_task with level 1: "Create task CRUD operations"]
```

### Example 2: Managing an Existing Plan

**Check current status**
```
AI: Let me see what we're currently working on:

[Uses get_current to see active task]
[Uses get_distilled_context for overview]
```

**Add implementation details**
```
AI: I see we're on the authentication component. Let me add specific implementation tasks:

[Uses add_task with level 3: "Install JWT library"]
[Uses add_task with level 3: "Create user registration endpoint"]
[Uses add_task with level 3: "Implement login validation"]
```

**Track progress**
```
AI: Great! You've completed the JWT setup. Let me mark that done:

[Uses complete_task with summary: "Successfully integrated JWT authentication library"]
```

## Best Practices

### Planning Strategy

1. **Start High-Level**: Begin with Level 0 tasks that capture major goals
2. **Progressive Refinement**: Break down tasks as you understand requirements better
3. **Use Notes Liberally**: Capture context, decisions, and requirements in task notes
4. **Regular Reviews**: Use `get_distilled_context` to maintain big-picture awareness

### Task Organization

- **Level 0**: Major project phases or architectural components
- **Level 1**: Independent modules or features that can be developed separately
- **Level 2**: Ordered steps within a feature, considering dependencies
- **Level 3**: Specific, actionable implementation tasks

### Collaboration with AI

- **Be Specific**: Provide clear context when asking AI to add or modify tasks
- **Review Suggestions**: AI can suggest task breakdowns, but review for your specific needs
- **Use Completion Summaries**: Document what was accomplished when completing tasks
- **Maintain Context**: Use notes to preserve important decisions and rationale

### Performance Tips

- **Use Leases**: For collaborative environments, use leases to coordinate task completion
- **Regular Navigation**: Use `move_to` to keep focus on current work
- **Plan Cleanup**: Remove or archive completed plans to maintain organization

## Troubleshooting

### Common Issues

#### MCP Server Won't Start

**Symptoms**: AI assistant can't connect to scatterbrain tools

**Solutions**:
1. Verify scatterbrain is installed: `scatterbrain --version`
2. Test MCP server manually: `scatterbrain mcp`
3. Check AI assistant MCP configuration
4. Restart AI assistant after configuration changes

#### Tools Not Appearing

**Symptoms**: AI assistant doesn't show scatterbrain tools

**Solutions**:
1. Confirm MCP configuration syntax is correct
2. Check file paths in configuration
3. Verify scatterbrain binary is in PATH
4. Try with `--example` flag for testing

#### Permission Errors

**Symptoms**: Cannot create or modify plans

**Solutions**:
1. Check file system permissions in working directory
2. Run with appropriate user permissions
3. Verify disk space availability

#### Performance Issues

**Symptoms**: Slow responses from MCP tools

**Solutions**:
1. Use `--expose` flag to enable web UI for large plans
2. Consider breaking very large plans into smaller ones
3. Regular cleanup of completed tasks

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
RUST_LOG=debug scatterbrain mcp
```

### Getting Help

- **Built-in Help**: Use the `get_guide` MCP tool
- **CLI Help**: Run `scatterbrain --help` or `scatterbrain mcp --help`
- **Issue Tracker**: [GitHub Issues](https://github.com/your-username/scatterbrain/issues)

## Advanced Usage

### Combined Mode with Web UI

Run MCP server with web interface for visual planning:

```bash
scatterbrain mcp --expose 8080
```

Access web UI at `http://localhost:8080` while maintaining MCP integration.

*[Screenshot placeholder: Web UI showing hierarchical task structure]*

### Environment Variables

Configure default behavior:

```bash
export SCATTERBRAIN_PLAN_ID=1  # Default plan for CLI operations
export RUST_LOG=info           # Logging level
```

### Integration with Development Workflows

Combine scatterbrain with other development tools:

```bash
# Start with development environment
scatterbrain mcp --expose 8080 &
code .  # Open editor
```

---

**Next Steps**: Explore the [CLI Reference](CLI-REFERENCE.md) for direct command-line usage or check out [Examples & Patterns](EXAMPLES.md) for more real-world scenarios. 