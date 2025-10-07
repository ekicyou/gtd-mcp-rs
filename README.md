# gtd-mcp-rs
GTD MCP Server

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management.

## Features

- Task management (inbox, next actions, waiting for, someday/maybe, done)
- Project management
- Context management
- TOML-based storage (gtd.toml)
- Git-friendly data format

## Building

```bash
cargo build --release
```

## Usage

The server uses stdio transport and can be integrated with MCP clients:

```bash
cargo run
```

## MCP Tools

The server provides the following tools:

- `add_task`: Add a new task to the inbox
- `list_tasks`: List all tasks (with optional status filter)
- `add_project`: Add a new project
- `list_projects`: List all projects

## Data Storage

Tasks and projects are stored in `gtd.toml` in the current directory. This file can be version controlled with git for backup and synchronization.
