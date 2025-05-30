//! API module
//!
//! This module provides the API functionality for the scatterbrain tool,
//! including the server, client, and data models.

pub mod client;
pub mod mcp;
pub mod server;

// Re-export commonly used types
pub use client::{Client, ClientConfig, ClientError, HttpClientImpl};
pub use mcp::ScatterbrainMcpServer;
pub use server::{serve, ServerConfig};
