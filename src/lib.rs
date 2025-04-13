//! Scatterbrain library crate
//!
//! This library provides functionality for the scatterbrain tool.

use std::path::PathBuf;

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

// todo: server, client, cli, log
#[derive(Debug, Clone)]
pub struct Level {
    pub description: &'static str,
    pub questions: &'static [&'static str],
}

pub const PLAN: Level = Level {
    description: "high level planning; identifying architecture, scope, and approach",
    questions: &[
        "Is this approach simple?",
        "Is this approach extensible?",
        "Does tihs approach provide good, minimally leaking abstractions?",
    ],
};

pub const ISOLATION: Level = Level {
    description: "Identifying discrete parts of the plan which can be completed independently",
    questions: &[
        "If possible, can each part be completed and verified independently",
        "Are the boundaries between pieces modular and extensible?",
    ],
};

pub const ORDERING: Level = Level {
    description: "Ordering the parts of the plan",
    questions: &[
        "Do we move from foundational building blocks to more complex concepts?",
        "Do we follow idiomatic design patterns?",
    ],
};

pub const IMPLEMENTATION: Level = Level {
    description: "Turning each part into an ordered list of tasks",
    questions: &[
        "Can each task be completed independently?",
        "Is each task complimentary to, or does it build upon, the previous tasks?",
        "Does each task minimize the execution risk of the other tasks?",
    ],
};

pub const DEFAULT_LEVELS: &[Level] = &[PLAN, ISOLATION, ORDERING, IMPLEMENTATION];

#[derive(Debug, Clone)]
pub struct Map {
    levels: Vec<Level>,
    cursor: u32,
}

enum Answer {
    Pass(String),
    Fail(String),
    Navigate(u32),
}

pub struct Config {
    dir: PathBuf,
}

pub struct App {
    config: Config,
    map: Map,
}

impl App {
    fn current(&self) -> Level {
        self.map.levels[self.map.cursor as usize].clone()
    }

    fn levels(&self) -> Vec<Level> {
        self.map.levels.clone()
    }

    fn increment_cursor(&mut self) {
        self.map.cursor += 1;
    }

    fn decrement_cursor(&mut self) {
        self.map.cursor -= 1;
    }

    fn move_cursor(&mut self, index: u32) {
        self.map.cursor = index;
    }
}
