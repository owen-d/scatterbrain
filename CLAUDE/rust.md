# Rust Project Guidelines

This document outlines conventions and best practices for developing Rust projects within this workspace.

## Project Structure & Organization

1.  **Workspaces:** Utilize Cargo workspaces for multi-crate projects. Place the `Cargo.toml` defining the workspace at the repository root.
2.  **Crate Granularity:** Start with logic grouped into single crates. Split crates only when necessary due to:
    *   **Complexity:** The crate becomes too large or difficult to manage.
    *   **Reuse:** A specific piece of functionality needs to be reused independently across multiple parts of the project or in other projects.
    *   **Compile Times:** Splitting can sometimes improve incremental compilation times.
    *   **Features/Dependencies:** To isolate optional features or heavy dependencies.
3.  **Minimal Binaries:** Keep executable logic in `main.rs` (or other files in `src/bin/`) minimal. Typically, it should parse arguments (often delegating to a dedicated CLI module/crate) and call a primary execution function from the library crate.
4.  **Separation of Concerns (Logic vs. I/O):**
    *   Strive to separate core business logic from I/O operations (networking, file system, etc.).
    *   Implement core logic in structs/modules that can be unit-tested without I/O dependencies.
    *   For concurrent access to shared state (common in servers), use thread-safe wrappers like `Arc<RwLock<CoreLogic>>`.
    *   Create separate layers or types to handle I/O integration (e.g., an Axum server layer that uses the `Arc<RwLock<CoreLogic>>`).
5.  **Visibility Minimization:** Keep functions, structs, fields, and modules private (`mod`, `pub(crate)`, or default) unless they *must* be part of the public API of the crate. Expose only what is necessary.
6.  **Client/Server Abstraction (for APIs/RPC):**
    *   When building components with client-server interactions (like web services or RPC), define a shared `Application` trait that encapsulates the core request-response functionality (`async fn handle(Input) -> Result<Output>`).
    *   Implement a `Server` type that fulfills this trait by calling internal logic (e.g., `app.method()`).
    *   Implement a `Client` type that fulfills the trait by making network calls (e.g., using `reqwest`).
    *   Organize code roughly as follows:
        ```
        src
        ├── api         # Defines the Application trait and client/server impls
        │   ├── client.rs
        │   ├── mod.rs
        │   └── server.rs
        ├── app.rs      # Core application logic, potentially behind Arc<RwLock>
        ├── cli.rs      # Optional CLI argument parsing/handling, uses api::client
        ├── lib.rs      # Exports public API, declares modules
        └── bin         # Executable(s)
            └── main.rs # Minimal entry point, calls cli or app directly
        ```

## Code Conventions
* Prefer smaller functions over larger ones. Readily break out logic into smaller functions when it makes sense.
* Prefer pure functions, e.g. combinators, over impure functions.

## Testing Strategy

1.  **Unit Tests:** Write unit tests for all non-trivial logic.
    *   **Private Functions:** Test private functions within the same file using a nested `#[cfg(test)] mod tests { ... }` module at the bottom.
    *   **Public Functions:** Test public API functions similarly within their modules.
2.  **Integration Tests:** Place tests that exercise the crate's public API as an external user would in a separate `tests/` directory at the *crate* root (not the workspace root, unless testing the entire workspace interaction).
3.  **Test Module Structure:** Mirror the `src/` module structure within the `tests/` directory for clarity (e.g., `tests/auth/login.rs` tests functions in `src/auth/login.rs`).
4.  **Documentation Tests:** Write code examples within `///` doc comments. These examples are compiled and run by `cargo test`, ensuring documentation stays accurate and functional. Prioritize these for demonstrating public API usage.

## Documentation & Comments

Maintain comprehensive documentation using Rust's built-in documentation system (`rustdoc`).

1.  **Doc Comments (`///` and `//!`):**
    *   **Modules:** Use `//!` at the beginning of `mod.rs` or crate root (`lib.rs`/`main.rs`) to document the module/crate itself.
    *   **Items:** Use `///` before every public function, struct, enum, trait, type alias, and important constants to explain their purpose and usage.
    *   **Examples:** Include runnable code examples within `///` comments using ```rust blocks. These are essential for demonstrating usage and are verified by `cargo test`.
    *   **Standard Sections:** Where applicable, include sections like `# Examples`, `# Panics` (conditions under which the function panics), `# Errors` (how errors are returned), `# Safety` (for `unsafe` code), and `# See also`.
2.  **Implementation Comments (`//` and `/* */`):**
    *   Use regular comments to explain *why* non-obvious code exists. Clarify complex algorithms, tricky logic, safety invariants for `unsafe` blocks, or performance trade-offs.
    *   Use `TODO:` or `FIXME:` markers to indicate areas needing future work or known issues.
    *   Use `NB:` (Nota Bene) or similar markers for important implementation notes or rationales.
    *   Use `SAFETY:` comments immediately preceding or within `unsafe` blocks to justify their safety.
    * Do NOT use comments for notes related to your temporary process/aren't otherwise relevant. Our code is not your scratch pad. Things like `// REFACTOR START .. REFACTOR END` are not helpful to later users. 

## Example Documentation Structure (from `std`)

```rust
//! Module-level documentation explaining the overall purpose.

/// Item-level documentation for a public struct/function/etc.
/// Explains what it does.
///
/// # Examples
///
/// ```rust
/// // Runnable code example demonstrating usage.
/// let example = demonstrates_api();
/// assert!(example.is_ok());
/// ```
///
/// # Errors
/// Describes conditions under which an error (`Result::Err`) is returned.
///
/// # Panics
/// Describes conditions under which this function might panic.
///
/// # Safety
/// ONLY for unsafe functions/traits. Explains the contract the caller must uphold.
pub fn demonstrates_api() -> Result<(), &'static str> {
    // SAFETY: Justification for why this unsafe block is safe.
    unsafe {
        // ... unsafe code ...
    }

    // NB: Implementation note about a design choice or tricky part.
    // We chose approach X because Y...

    // TODO: Refactor this later.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from the outer module

    #[test]
    fn test_demonstrates_api() {
        assert!(demonstrates_api().is_ok());
    }

    // Test private functions if necessary
    // fn test_internal_helper() { ... }
}
```

## Implementation vs Thread-Safe Wrapper Pattern

When designing Rust libraries and applications that need both clean implementation logic and thread-safety, follow this pattern to separate concerns.

### Core Implementation Struct

Create a base implementation struct that focuses purely on business logic:

```rust
struct MyLogicImpl {
    data: Vec<String>,
    config: Config,
    // other internal state...
}

impl MyLogicImpl {
    fn new(config: Config) -> Self {
        Self {
            data: Vec::new(),
            config,
        }
    }
    
    // Methods focus on core logic without concurrency concerns
    fn add_item(&mut self, item: String) {
        self.data.push(item);
    }
    
    fn process_data(&self) -> Result<(), Error> {
        // Business logic implementation
        Ok(())
    }
}
```

### Thread-Safe Wrapper

Create a wrapper type that handles concurrency concerns:

```rust
struct MyLogic {
    inner: Arc<RwLock<MyLogicImpl>>,
}

impl MyLogic {
    fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MyLogicImpl::new(config))),
        }
    }
    
    // Public API methods handle locking
    fn add_item(&self, item: String) -> Result<(), Error> {
        let mut guard = self.inner.write().map_err(|_| Error::LockPoisoned)?;
        guard.add_item(item);
        Ok(())
    }
    
    fn process_data(&self) -> Result<(), Error> {
        let guard = self.inner.read().map_err(|_| Error::LockPoisoned)?;
        guard.process_data()
    }
}
```

### Benefits

- **Separation of concerns**: Implementation logic is distinct from concurrency management
- **Simplified testing**: Core logic can be tested without threading complexities
- **API consistency**: Public interface remains clean while safely managing state
- **Flexibility**: Easy to adapt with different concurrency primitives (Mutex, parking_lot, etc.)
- **Maintainability**: Changes to locking strategy don't require modifying business logic

### Common Variations

- Using `Mutex<T>` when write access is more common than read access
- Using `parking_lot` primitives for better performance characteristics
- Adding an additional service interface for dependency injection patterns
- Implementing interior mutability patterns within the implementation struct