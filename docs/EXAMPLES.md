# Scatterbrain Examples and Workflows

This guide provides practical examples and real-world scenarios for using scatterbrain effectively. Whether you're using it as an MCP server with AI assistants, via CLI, or through the Web UI, these examples will help you understand the hierarchical planning methodology.

## Table of Contents

- [Quick Start Examples](#quick-start-examples)
- [MCP Integration Workflows](#mcp-integration-workflows)
- [Software Development Scenarios](#software-development-scenarios)
- [Project Management Examples](#project-management-examples)
- [Research and Analysis Workflows](#research-and-analysis-workflows)
- [Learning and Documentation](#learning-and-documentation)
- [Advanced Patterns](#advanced-patterns)
- [Best Practices](#best-practices)

## Quick Start Examples

### Example 1: Simple Task Planning

**Scenario**: Planning a weekend project

```bash
# Create a new plan
scatterbrain plan create "Organize home office"

# Add high-level planning tasks
scatterbrain task add "Assess current state and requirements" --level 0 --notes "Evaluate current setup and identify needs"
scatterbrain task add "Design organization system" --level 1 --notes "Plan storage solutions and workflow"
scatterbrain task add "Execute organization plan" --level 2 --notes "Implement the designed system"
scatterbrain task add "Maintain and optimize" --level 3 --notes "Regular maintenance and improvements"

# View the plan
scatterbrain plan show
```

**Result Structure**:
```
📋 Goal: Organize home office
├── 🔵 0: Assess current state and requirements    [Planning]
├── 🟣 1: Design organization system              [Isolation]
├── 🟢 2: Execute organization plan               [Ordering]
└── 🟠 3: Maintain and optimize                   [Implementation]
```

### Example 2: Breaking Down Complex Tasks

**Scenario**: The planning task needs subtasks

```bash
# Move to the first task
scatterbrain move 0

# Add subtasks for assessment
scatterbrain task add "Inventory current items and furniture" --level 1 --notes "Catalog all items and their current locations"
scatterbrain task add "Identify pain points and inefficiencies" --level 1 --notes "Document workflow problems and bottlenecks"
scatterbrain task add "Define success criteria" --level 1 --notes "Establish measurable goals for the organization project"

# View current state
scatterbrain current
```

**Result Structure**:
```
📋 Goal: Organize home office
├── 🔵 0: Assess current state and requirements    [Planning] ← Current
│   ├── 🟣 0.0: Inventory current items           [Isolation]
│   ├── 🟣 0.1: Identify pain points              [Isolation]
│   └── 🟣 0.2: Define success criteria           [Isolation]
├── 🟣 1: Design organization system              [Isolation]
├── 🟢 2: Execute organization plan               [Ordering]
└── 🟠 3: Maintain and optimize                   [Implementation]
```

## MCP Integration Workflows

### Example 3: AI-Assisted Software Development

**Scenario**: Using Claude/Cursor with scatterbrain MCP server

#### Setup
```bash
# Start MCP server with Web UI for monitoring
scatterbrain mcp --expose 3000
```

#### AI Assistant Interaction
```
Human: I need to build a REST API for a todo application. Can you help me plan this using scatterbrain? 