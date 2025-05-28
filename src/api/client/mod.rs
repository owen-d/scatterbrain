//! Client module
//!
//! This module provides HTTP client functionality to interact with the scatterbrain API server.

mod core;
mod http;
mod trait_def;

// Re-export the trait and types
pub use core::CoreClient;
pub use http::{ClientConfig, ClientError, HttpClientImpl};
pub use trait_def::Client;
