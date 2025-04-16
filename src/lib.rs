//! Scatterbrain library crate
//!
//! This library provides functionality for the scatterbrain tool.
//! It helps agents systematically work through complex projects
//! by breaking them down into manageable tasks in a hierarchical structure.

// Declare public modules
pub mod api;
pub mod cli;
pub mod levels;
pub mod models;

// Re-export the most commonly used types
pub use api::serve;
pub use cli::run;
pub use levels::{
    default_levels, implementation_level, isolation_level, ordering_level, plan_level, Level,
};
pub use models::{Context, Core, Plan, Task};
