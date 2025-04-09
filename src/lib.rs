//! Scatterbrain library crate
//!
//! This library provides functionality for the scatterbrain tool.

/// Example function in the library
pub fn hello_library() -> &'static str {
    "Hello from the scatterbrain library!"
}

pub mod utils {
    /// A utility function
    pub fn calculate_something(input: i32) -> i32 {
        input * 2
    }
}
