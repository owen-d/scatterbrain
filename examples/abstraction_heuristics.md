# Heuristics for Identifying Appropriate Abstraction Levels

This document provides concrete guidelines for agents to identify when they should be operating at each abstraction level and when to transition between levels.

## Abstraction Level Identification Heuristics

### When to Use Level 1 (High-Level Planning)

**Key Indicators:**
- You're starting a new project or major component
- You need to make architectural decisions
- You're setting overall direction and approach
- You're evaluating alternative high-level approaches

**Questions that signal Level 1:**
- "What's the overall architecture?"
- "Which approach should we take?"
- "What are the main components?"
- "How will these parts interact conceptually?"

**Signs you should move down from Level 1:**
- You're confident in the overall approach
- You need to define specific components
- You keep wanting to talk about specific components in detail
- The plan has clear areas that can be separated

### When to Use Level 2 (Isolation)

**Key Indicators:**
- You need to define boundaries between components
- You're determining interfaces and contracts
- You're identifying independent work areas
- You're creating separation of concerns

**Questions that signal Level 2:**
- "What are the interfaces between components?"
- "How should we divide this into independent parts?"
- "What are the inputs and outputs of each component?"
- "How do we ensure loose coupling between parts?"

**Signs you should move down from Level 2:**
- Components and their interfaces are well-defined
- You need to determine implementation sequence
- You're thinking about dependencies between components
- You're ready to plan the order of implementation

### When to Use Level 3 (Ordering)

**Key Indicators:**
- You need to sequence implementation steps
- You're identifying dependencies between tasks
- You're planning the critical path
- You're organizing work for efficient execution

**Questions that signal Level 3:**
- "What order should we implement these components?"
- "What are the dependencies between tasks?"
- "Which parts should be built first?"
- "How do we sequence work to minimize integration issues?"

**Signs you should move down from Level 3:**
- The sequence of implementation is clear
- You need to define specific implementation tasks
- You're ready to write concrete, actionable tasks
- You need detailed implementation guidelines

### When to Use Level 4 (Implementation)

**Key Indicators:**
- You're defining specific, actionable tasks
- You're detailing exact implementation steps
- You're creating concrete task definitions
- You're ready for execution of tasks

**Questions that signal Level 4:**
- "What specific code changes are needed?"
- "What exact steps implement this component?"
- "What specific files need to be created or modified?"
- "What concrete tests should be written?"

**Signs you should move up from Level 4:**
- You're stuck on implementation details
- You need to revisit design decisions
- You've discovered integration issues
- You need to reorganize remaining work

## Transitioning Between Abstraction Levels

### Upward Transitions

**Level 4 → Level 3:**
- You've completed a set of implementation tasks
- You need to reorganize the sequence of remaining tasks
- You've discovered dependencies that weren't initially apparent
- You need to reprioritize your implementation order

**Level 3 → Level 2:**
- You've discovered issues with component interfaces
- You need to reconsider component boundaries
- Integration between ordered tasks is more complex than expected
- You need to redefine independence between components

**Level 2 → Level 1:**
- You've discovered fundamental flaws in the approach
- The separated components don't form a coherent system
- You need to reconsider the entire architecture
- You're facing major obstacles that require rethinking the approach

### Downward Transitions

**Level 1 → Level 2:**
- The high-level approach is clear and solid
- You're ready to define component boundaries
- You need to establish contracts between components
- You need to identify independent work areas

**Level 2 → Level 3:**
- Component boundaries are well-defined
- You're ready to determine implementation sequence
- You need to identify dependencies between components
- You need to create an efficient build order

**Level 3 → Level 4:**
- The implementation sequence is clear
- You're ready to define specific tasks
- You need to create concrete, actionable work items
- You're ready to execute on the implementation plan

## Example Abstraction Level Flow

For implementing a new login system:

1. **Level 1:** Decide on overall auth approach (e.g., JWT vs. sessions)
2. **Level 2:** Define auth component interfaces (API endpoints, data models)
3. **Level 3:** Sequence implementation (DB first, then backend, then frontend)
4. **Level 4:** Create specific tasks (e.g., "Create LoginController with method signatures")
5. **Back to Level 3:** After completing initial tasks, resequence remaining work
6. **Back to Level 4:** Continue with next set of implementation tasks

## Common Abstraction Level Mistakes

1. **Premature implementation detail:** Diving into code specifics at Level 1
2. **Inconsistent abstractions:** Mixing high-level and low-level concerns
3. **Abstraction resistance:** Staying too high-level when implementation details are needed
4. **Abstraction abandonment:** Getting lost in details and forgetting the big picture
5. **Level skipping:** Jumping from Level 1 to Level 4 without proper component definition and sequencing 