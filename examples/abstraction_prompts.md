# Prompts for Guiding Agents in Abstraction Level Navigation

This document provides specific prompts that can be used to guide agents in effectively navigating between abstraction levels during planning.

## Initialization Prompts

### Setting Expectations for Abstraction Levels

```
As we work through this project, we will be operating at different levels of abstraction. Each level has a specific purpose:

1. Level 1 (Planning): Focus on overall architecture and approach
2. Level 2 (Isolation): Focus on component boundaries and interfaces
3. Level 3 (Ordering): Focus on implementation sequence and dependencies 
4. Level 4 (Implementation): Focus on specific, actionable tasks

I'll help you maintain the appropriate level of abstraction as we go. Please let me know if you feel we need to move up or down in abstraction level.
```

### Initial Assessment for Project Complexity

```
Before we begin planning, let's assess the complexity of this project:
- How many distinct components or systems will be involved?
- Are there existing constraints or requirements that shape our approach?
- What are the major technical or organizational challenges we anticipate?

This will help us determine the appropriate level of abstraction to start with.
```

## Level-Specific Guidance Prompts

### Level 1 (Planning) Guidance

```
We're currently at the Planning level of abstraction. At this level:

- Focus on the big picture and overall architecture
- Avoid implementation details or specific technologies when possible
- Consider alternative approaches and evaluate tradeoffs
- Think about how the system will function as a whole

Questions to consider:
- What are the key components or subsystems needed?
- How will these components interact at a high level?
- What are the key requirements or constraints driving our approach?
```

### Level 2 (Isolation) Guidance

```
We're now at the Component Isolation level of abstraction. At this level:

- Focus on defining clear boundaries between components
- Identify interfaces, inputs, and outputs for each component
- Ensure components can be developed and tested independently
- Consider how to maintain loose coupling between parts

Questions to consider:
- What are the key interfaces between components?
- How should responsibilities be divided between components?
- What data or events flow between components?
- How can we ensure changes in one component won't break others?
```

### Level 3 (Ordering) Guidance

```
We're at the Ordering level of abstraction. At this level:

- Focus on the sequence of implementation
- Identify dependencies between components and tasks
- Plan the critical path for development
- Consider how to minimize integration risks

Questions to consider:
- Which components must be built first?
- What are the dependencies between different parts?
- How can we validate each part as early as possible?
- What is the most efficient sequence for implementation?
```

### Level 4 (Implementation) Guidance

```
We're at the Implementation level of abstraction. At this level:

- Focus on specific, actionable tasks
- Define concrete implementation steps
- Be specific about files, functions, and changes needed
- Consider testing, edge cases, and error handling

Questions to consider:
- What specific code changes are needed?
- What files need to be created or modified?
- How will we test this implementation?
- What edge cases or error conditions should we handle?
```

## Transition Guidance Prompts

### Moving Up in Abstraction (When Too Detailed)

```
I notice we're getting into very specific implementation details, but we might benefit from stepping back to a higher level of abstraction. Let's temporarily set aside these details and focus on [higher level concern].

Would you like to move up to Level [N] to reconsider [broader aspect]?
```

### Moving Down in Abstraction (When Too General)

```
We have a good handle on the [current level] considerations. I think we're ready to move down to a more concrete level of abstraction.

Let's move to Level [N-1] to [specific next step appropriate for that level].
```

### Recognizing Abstraction Level Mismatch

```
I notice we're mixing different levels of abstraction in our discussion. For example, we're talking about [high-level concern] but also diving into [low-level detail].

Let's focus on [appropriate level concern] first, and we can address the implementation details once we've established the broader structure.
```

## Problem-Solving Prompts

### When Stuck at Implementation Level

```
We seem to be stuck on these implementation details. It might help to move up to Level [3 or 2] and reconsider [component boundaries or sequence].

Sometimes implementation difficulties indicate that we need to revisit higher-level decisions.
```

### When High-Level Planning is Too Vague

```
Our high-level plan feels a bit abstract. Let's try to make it more concrete by moving down to Level 2 and identifying specific components and their interfaces.

This will help us validate that our high-level approach is feasible.
```

### When Reaching Decision Points

```
We've reached a point where we need to make a decision about [issue]. 

First, let's identify which level of abstraction this decision belongs to:
- Is this about overall architecture? (Level 1)
- Is this about component boundaries? (Level 2)
- Is this about implementation sequence? (Level 3)
- Is this about specific implementation? (Level 4)

This will help us approach the decision with the right considerations in mind.
```

## Progress Evaluation Prompts

### Checking Abstraction Level Effectiveness

```
Let's pause to evaluate how effectively we're using abstraction levels:

- Are we making good progress at our current level?
- Are we getting stuck in details or losing sight of the big picture?
- Would moving up or down a level help us make better progress?

This reflection can help us adjust our approach if needed.
```

### Completion Assessment for Current Level

```
We've made significant progress at Level [N]. Before moving to the next level, let's check:

- Have we addressed all the key concerns at this level?
- Are there any gaps or uncertainties we should resolve first?
- Are we confident in our decisions at this level?

Once we're satisfied, we can move to Level [N+1 or N-1] to [appropriate next step].
``` 