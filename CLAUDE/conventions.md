# General Project Conventions

## Documentation

Prioritize comprehensive Rust documentation comments (`///`) for all public items (modules, structs, enums, functions, traits, etc.).

*   **Purpose:** Clearly explain the *what* and *why* of each item. Describe its purpose, usage, and any important considerations or constraints.
*   **Examples:** Include illustrative code examples (` ```rust ... ``` `) within doc comments where appropriate to demonstrate usage.
*   **`cargo doc`:** Regularly build and review the documentation using `cargo doc --no-deps --open`. Ensure documentation is accurate, complete, and free of warnings (like broken links).
*   **Intra-Doc Links:** Use intra-doc links (`[`name`]` or `[`name`](path::to::Item)`) to connect related concepts within the documentation.
*   **Living Document:** Treat documentation comments as living documents, updating them alongside code changes.

## Workspace README

Maintain an informative `README.md` file at the **workspace root**.

*   **Content:** Provide a high-level overview of the project, its purpose, setup instructions, build/test commands, and contribution guidelines.
*   **Module READMEs:** Avoid creating separate `README.md` files within individual modules or sub-directories. Module-level explanations should primarily reside in the module's documentation comment (`//!` at the top of `mod.rs` or the module file).

## Simplicity

Focus on simple implementations that can be refactored. Needs change and it's more effective to rebuild when we have more context.