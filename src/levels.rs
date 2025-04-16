//! Predefined abstraction levels for Scatterbrain
//!
//! This module defines the default abstraction levels used in Scatterbrain's planning process.

use serde::{Deserialize, Serialize};

/// Represents an abstraction level for the LLM to work through
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    description: String,
    questions: Vec<String>,
    abstraction_focus: String,
}

impl Level {
    /// Creates a new level
    pub fn new(description: String, questions: Vec<String>, abstraction_focus: String) -> Self {
        Self {
            description,
            questions,
            abstraction_focus,
        }
    }

    /// Returns a string that guides agents on how to effectively use this abstraction level
    pub fn get_guidance(&self) -> String {
        format!(
            "Abstraction level: {}\n\nFocus instruction: {}\n\nRelevant questions to consider:\n{}",
            self.description,
            self.abstraction_focus,
            self.questions
                .iter()
                .map(|q| format!("- {}", q))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Gets the description of this level
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Gets the questions for this level
    pub fn questions(&self) -> &[String] {
        &self.questions
    }

    /// Gets the abstraction focus for this level
    pub fn abstraction_focus(&self) -> &str {
        &self.abstraction_focus
    }
}

/// Returns the default planning level
pub fn plan_level() -> Level {
    Level {
        description: "high level planning; identifying architecture, scope, and approach"
            .to_string(),
        questions: vec![
            "Is this approach simple?".to_string(),
            "Is this approach extensible?".to_string(),
            "Does this approach provide good, minimally leaking abstractions?".to_string(),
        ],
        abstraction_focus: "Maintain altitude by focusing on system wholes. Avoid implementation details. Think about conceptual patterns rather than code structures. Consider how components will interact without specifying their internal workings.".to_string(),
    }
}

/// Returns the default isolation level
pub fn isolation_level() -> Level {
    Level {
        description: "Identifying discrete parts of the plan which can be completed independently"
            .to_string(),
        questions: vec![
            "If possible, can each part be completed and verified independently".to_string(),
            "Are the boundaries between pieces modular and extensible?".to_string(),
        ],
        abstraction_focus: "Focus on interfaces and boundaries between components. Define clear inputs and outputs for each part. Identify dependencies while preserving modularity. Look for natural divisions in the problem space.".to_string(),
    }
}

/// Returns the default ordering level
pub fn ordering_level() -> Level {
    Level {
        description: "Ordering the parts of the plan".to_string(),
        questions: vec![
            "Do we move from foundational building blocks to more complex concepts?".to_string(),
            "Do we follow idiomatic design patterns?".to_string(),
        ],
        abstraction_focus: "Think about sequence and progression. Identify dependencies and build order without diving into implementation details. Consider critical paths and bottlenecks. Focus on logical flow and execution constraints.".to_string(),
    }
}

/// Returns the default implementation level
pub fn implementation_level() -> Level {
    Level {
        description: "Turning each part into an ordered list of tasks".to_string(),
        questions: vec![
            "Can each task be completed independently?".to_string(),
            "Is each task complimentary to, or does it build upon, the previous tasks?".to_string(),
            "Does each task minimize the execution risk of the other tasks?".to_string(),
        ],
        abstraction_focus: "Focus on concrete, actionable steps. Define specific code changes or artifacts to produce. Reference higher abstractions when needed but maintain focus on precise implementation. Consider error cases and edge conditions.".to_string(),
    }
}

/// Returns the default set of levels for planning
pub fn default_levels() -> Vec<Level> {
    vec![
        plan_level(),
        isolation_level(),
        ordering_level(),
        implementation_level(),
    ]
}
