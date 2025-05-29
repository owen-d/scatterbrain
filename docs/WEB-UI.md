# Scatterbrain Web UI Guide

The Scatterbrain Web UI provides a visual interface for managing hierarchical plans and tasks. It offers real-time updates, plan navigation, and comprehensive task visualization.

## Table of Contents

- [Getting Started](#getting-started)
- [Interface Overview](#interface-overview)
- [Plan Management](#plan-management)
- [Task Visualization](#task-visualization)
- [Real-time Updates](#real-time-updates)
- [Navigation](#navigation)
- [Features](#features)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Launching the Web UI

The Web UI can be accessed in several ways:

#### 1. Standalone HTTP Server
```bash
# Start HTTP server on default port 3000
scatterbrain serve

# Start on custom port with example data
scatterbrain serve --port 8080 --example
```

#### 2. Combined with MCP Server
```bash
# Start MCP server with Web UI exposed on port 3000
scatterbrain mcp --expose 3000

# With example data for testing
scatterbrain mcp --example --expose 8080
```

#### 3. Access the Interface
Once running, open your browser to:
- `http://localhost:3000` (default port)
- `http://localhost:8080` (custom port example)

### First Time Setup

1. **Create a Plan**: If no plans exist, you'll see a message to create one via CLI
2. **Example Data**: Use `--example` flag to populate with sample tasks for exploration
3. **Plan Selection**: The UI will redirect to `/ui` showing available plans

## Interface Overview

### Main Components

The Web UI consists of several key sections:

#### 1. Plan Navigation Tabs
- **Location**: Top of the interface
- **Function**: Switch between different plans
- **Format**: "Plan 1", "Plan 2", etc.
- **Active Plan**: Highlighted with different styling

#### 2. Plan Information Panel
- **Goal Display**: Shows the main objective of the current plan
- **Plan Notes**: Displays any additional context or notes
- **Visual Styling**: Light blue background with border accent

#### 3. Task Tree Visualization
- **Hierarchical Display**: Shows tasks in tree structure
- **Level Indicators**: Color-coded circles showing abstraction levels
- **Task Status**: Visual indicators for completion state
- **Current Task**: Highlighted with blue accent border

#### 4. Current Task Panel
- **Active Task**: Details of the currently selected task
- **Level Information**: Shows current abstraction level
- **Task Notes**: Displays any notes associated with the task
- **Subtasks**: Lists immediate child tasks

#### 5. History Panel
- **Recent Actions**: Chronological list of plan modifications
- **Timestamps**: UTC timestamps for each action
- **Action Types**: Move, complete, add, etc.
- **Details**: Specific information about each change

#### 6. Level Legend
- **Abstraction Levels**: Visual guide to the 4-level hierarchy
- **Color Coding**: 
  - **Level 0 (Blue)**: Planning - High-level strategy
  - **Level 1 (Purple)**: Isolation - Component boundaries  
  - **Level 2 (Green)**: Ordering - Sequence and dependencies
  - **Level 3 (Orange)**: Implementation - Concrete actions

#### 7. Connection Status
- **Real-time Indicator**: Shows connection to server
- **Status Types**:
  - **Green**: Connected and listening
  - **Orange**: Updating/syncing
  - **Gray**: Disconnected or waiting

## Plan Management

### Plan Selection

The UI automatically detects available plans and provides navigation:

```
Plan Navigation:
┌─────────────────────────────────┐
│ Plan 1  │ Plan 2  │ Plan 3     │ ← Tabs for each plan
└─────────────────────────────────┘
```

### Plan Information Display

Each plan shows:
- **Goal**: Primary objective or prompt
- **Notes**: Additional context or description
- **Creation Info**: Accessible via CLI commands

### Creating New Plans

Plans must be created via CLI:
```bash
# Create new plan
scatterbrain plan create "Build documentation system"

# With notes
scatterbrain plan create "API refactor" --notes "Focus on performance improvements"
```

## Task Visualization

### Hierarchical Structure

Tasks are displayed in a tree format showing:

```
📋 Plan Goal: Build comprehensive documentation
├── 🔵 0: Analyze existing documentation     [Planning]
├── 🟣 1: Design documentation architecture  [Isolation]  
│   ├── 🟣 1.0: Define README structure     [Isolation]
│   └── 🟣 1.1: Plan detailed files        [Isolation]
├── 🟢 2: Create comprehensive README       [Ordering]
└── 🟠 3: Validate and finalize            [Implementation]
```

### Visual Elements

#### Level Indicators
- **Circles**: Color-coded by abstraction level
- **Numbers**: Show level index (0-3)
- **Borders**: Indicate level boundaries

#### Task Status
- **Completed Tasks**: 
  - Strikethrough text
  - Green checkmark
  - Grayed out appearance
- **Current Task**:
  - Blue left border
  - Highlighted background
  - Bold task path

#### Task Information
- **Index Path**: Shows position in hierarchy (e.g., "1.2.0")
- **Description**: Task title and details
- **Notes**: Additional context when available
- **Completion Summary**: For completed tasks

### Task Notes Display

When tasks have notes, they appear:
- **Location**: Below task description
- **Styling**: Indented with gray background
- **Format**: Preserves line breaks and formatting

## Real-time Updates

### Server-Sent Events (SSE)

The UI uses EventSource for live updates:

```javascript
// Automatic connection to plan-specific events
EventSource: /ui/events/{plan_id}
```

### Update Behavior

When changes occur via CLI or MCP:
1. **Detection**: Server broadcasts change notification
2. **Status Update**: Connection indicator shows "Updating..."
3. **Refresh**: Page automatically reloads to show changes
4. **Reconnection**: Automatic reconnection on connection loss

### Connection States

- **🟢 Connected**: "Connected: Listening for changes"
- **🟠 Updating**: "Updating..." (during refresh)
- **⚪ Disconnected**: "Connection lost. Reconnecting..."

## Navigation

### Plan Switching

Click any plan tab to switch between plans:
- **URL Format**: `/ui/{plan_id}`
- **State Preservation**: Each plan maintains its own view
- **Instant Loading**: No page refresh required

### Task Navigation

Task navigation is handled via CLI:
```bash
# Move to specific task
scatterbrain move 1,2,0

# Move to parent level  
scatterbrain move 1,2

# Move to root
scatterbrain move 0
```

### URL Structure

- **Plan List**: `/ui` - Shows all available plans
- **Specific Plan**: `/ui/{id}` - Shows plan details
- **Events Stream**: `/ui/events/{id}` - SSE endpoint

## Features

### Responsive Design

The UI adapts to different screen sizes:
- **Desktop**: Multi-column layout with panels
- **Mobile**: Stacked layout for better readability
- **Flexible**: Panels resize based on content

### Accessibility

- **Semantic HTML**: Proper heading structure
- **Color Coding**: Consistent visual language
- **Keyboard Navigation**: Standard browser navigation
- **Screen Readers**: Descriptive text and labels

### Performance

- **Efficient Updates**: Only refreshes on actual changes
- **Minimal Data**: Lightweight JSON responses
- **Caching**: Browser caching for static assets
- **Connection Management**: Automatic reconnection handling

### Browser Compatibility

- **Modern Browsers**: Chrome, Firefox, Safari, Edge
- **EventSource Support**: Required for real-time updates
- **JavaScript**: Required for dynamic functionality

## Troubleshooting

### Common Issues

#### 1. "No plans found" Message
**Problem**: UI shows no plans available
**Solution**: 
```bash
# Create a plan via CLI
scatterbrain plan create "My first plan"
```

#### 2. Connection Issues
**Problem**: Status shows "Connection lost"
**Solutions**:
- Check if server is running
- Verify correct port (default 3000)
- Check firewall settings
- Restart server if needed

#### 3. Page Not Updating
**Problem**: Changes via CLI don't appear
**Solutions**:
- Check connection status indicator
- Manually refresh browser
- Verify plan ID matches
- Check server logs for errors

#### 4. Plan Not Found Error
**Problem**: URL shows plan not found
**Solutions**:
```bash
# List available plans
scatterbrain plan list

# Use correct plan ID in URL
```

### Server Configuration

#### Port Conflicts
```bash
# Check if port is in use
lsof -i :3000

# Use different port
scatterbrain serve --port 8080
```

#### CORS Issues
The server includes CORS headers for development:
- **Allow Origin**: Any
- **Allow Methods**: All HTTP methods
- **Allow Headers**: All headers

### Performance Tips

1. **Use Example Data**: Start with `--example` for testing
2. **Monitor Connection**: Watch status indicator
3. **Browser DevTools**: Check Network tab for issues
4. **Server Logs**: Monitor console output for errors

### Development Mode

For development and testing:
```bash
# Start with example data and custom port
scatterbrain serve --example --port 8080

# Combined MCP + HTTP for full testing
scatterbrain mcp --example --expose 8080
```

## Advanced Usage

### Multiple Plans Workflow

1. **Create Multiple Plans**: Use CLI to create several plans
2. **Switch Between Plans**: Use tab navigation
3. **Parallel Work**: Each plan maintains independent state
4. **Cross-Plan Reference**: Use CLI to check other plans

### Integration with MCP

When using `--expose` with MCP server:
- **Dual Access**: Both MCP tools and Web UI available
- **Shared State**: Changes via MCP appear in UI
- **Development**: Ideal for AI assistant integration testing

### Custom Styling

The UI uses embedded CSS that can be customized by:
1. **Browser Extensions**: User stylesheets
2. **Developer Tools**: Live CSS editing
3. **Proxy Servers**: CSS injection
4. **Fork and Modify**: Custom server implementation

---

For more information, see:
- [MCP Integration Guide](MCP-GUIDE.md)
- [CLI Reference](CLI-REFERENCE.md)
- [Examples and Workflows](EXAMPLES.md) 