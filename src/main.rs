//! GTD MCP Server - Main Entry Point
//!
//! This is the main entry point for the GTD MCP server application.
//! The actual implementation is in the `gtd_mcp` library.

use anyhow::Result;
use clap::{CommandFactory, Parser};
use gtd_mcp::GtdServerHandler;
use mcp_attr::server::serve_stdio;

/// GTD MCP Server - Getting Things Done task management via Model Context Protocol
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the GTD data file
    file: String,

    /// Enable git synchronization on save
    #[arg(long)]
    sync_git: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check if no arguments were provided (except the program name)
    if std::env::args().len() == 1 {
        // No arguments provided, show help and exit with error code
        let mut cmd = Args::command();
        cmd.print_help().ok();
        println!(); // Add a newline after help
        std::process::exit(2);
    }

    let args = Args::parse();
    let handler = GtdServerHandler::new(&args.file, args.sync_git)?;
    serve_stdio(handler).await?;
    Ok(())
}
