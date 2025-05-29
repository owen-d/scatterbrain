# Scatterbrain

**A hierarchical planning and task management tool with powerful MCP server integration for AI assistants**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/your-username/scatterbrain)
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/your-username/scatterbrain)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Scatterbrain is designed to help you break down complex problems into manageable, hierarchical tasks. Its primary strength lies in its **Model Context Protocol (MCP) server**, which seamlessly integrates with AI assistants like Claude and Cursor, enabling intelligent task management and planning workflows.

## 🚀 Quick Start (MCP Server)

### Installation

```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build scatterbrain
git clone https://github.com/your-username/scatterbrain.git
cd scatterbrain
cargo build --release

# Install globally (optional)
cargo install --path .
```

### MCP Server Setup

#### For Cursor

Add to your Cursor MCP configuration:

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


#### Start MCP Server

```bash
# Basic MCP server
scatterbrain mcp

# With example data for testing
scatterbrain mcp --example

# With web UI exposed on port 8080
scatterbrain mcp --expose 8080
```

*[Screenshot placeholder: MCP server running in terminal]*

*[Screenshot placeholder: Cursor showing scatterbrain MCP tools]*

## ✨ Features

### 🎯 Hierarchical Task Management
- **"leveled" abstraction system**: Planning → Isolation → Ordering → Implementation
- Break complex projects into manageable pieces
- Natural progression from high-level goals to specific actions

### 🤖 MCP Server Integration
- **specialized tools** for AI assistants
- Plan creation, task management, navigation, and notes
- Seamless integration with Cursor, Claude, and other MCP-compatible tools

### 🌐 Web Interface
- Modern browser-based UI for visual task management
- Real-time updates and intuitive navigation
- Access via `--expose` flag when running MCP server

### 💻 Powerful CLI
- Complete command-line interface for all operations
- Shell completions for bash, zsh, fish
- Environment variable support for workflow integration

## 📋 Usage Modes

### MCP Server (Recommended)
Perfect for AI-assisted development and planning:
```bash
scatterbrain mcp --expose 8080
```
- Provides MCP tools to AI assistants
- Optional web UI access
- Ideal for collaborative AI workflows

### CLI Usage
Direct command-line interaction:
```bash
# Create a plan
scatterbrain plan create "Build documentation system"

# Add tasks and navigate
export SCATTERBRAIN_PLAN_ID=1
scatterbrain task add --level 0 "Design architecture" --notes "High-level planning"
scatterbrain move 0
```

### HTTP API Server
Standalone web server:
```bash
scatterbrain serve --port 3000
```
- Full REST API access
- Web UI at http://localhost:3000
- Programmatic integration

### Combined Mode
Best of all worlds:
```bash
scatterbrain mcp --expose 8080
```
- MCP server for AI assistants
- Web UI for visual management
- CLI access for scripting

## 📚 Documentation

- **[MCP Integration Guide](docs/MCP-GUIDE.md)** - Complete guide to using scatterbrain with AI assistants
- **[CLI Reference](docs/CLI-REFERENCE.md)** - Comprehensive command documentation
- **[Web UI Guide](docs/WEB-UI.md)** - Browser interface walkthrough
- **[Examples & Patterns](docs/EXAMPLES.md)** - Real-world usage scenarios
- **[Architecture Overview](docs/ARCHITECTURE.md)** - System design and concepts

## 🎯 Core Concepts

### Hierarchical Planning
Scatterbrain organizes work across four abstraction levels:

1. **Planning (Level 0)** - High-level goals and architecture
2. **Isolation (Level 1)** - Independent components and boundaries  
3. **Ordering (Level 2)** - Sequence and dependencies
4. **Implementation (Level 3)** - Concrete, actionable tasks

### MCP Tools
When running as an MCP server, scatterbrain provides these tools to AI assistants:

- `get_plan` - Retrieve plan details and structure
- `create_plan` - Create new plans from prompts
- `add_task` - Add tasks at appropriate abstraction levels
- `complete_task` - Mark tasks as completed with summaries
- `move_to` - Navigate between tasks
- `get_current` - Get current task context
- `get_distilled_context` - Get focused planning context
- And 10 more specialized tools for comprehensive task management

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/your-username/scatterbrain.git
cd scatterbrain
cargo build
cargo test
```

### Running Tests

```bash
make test
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- [GitHub Repository](https://github.com/your-username/scatterbrain)
- [Issue Tracker](https://github.com/your-username/scatterbrain/issues)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Cursor Editor](https://cursor.sh/)

---

**Want to stop babysitting your AI?** Start with the [MCP Integration Guide](docs/MCP-GUIDE.md) and see how scatterbrain transforms complex project management into an intelligent, collaborative process. 