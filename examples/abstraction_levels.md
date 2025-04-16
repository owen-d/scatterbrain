# Effective Abstraction Level Use Examples

This document provides examples of how agents can effectively utilize abstraction levels in planning with the enhanced `Level` structure in scatterbrain.

## Example: Building a Web Application

### Level 1: High-Level Planning

**Abstraction Focus:** *Maintain altitude by focusing on system wholes. Avoid implementation details. Think about conceptual patterns rather than code structures.*

**Good Example:**
```
Task: Design web application architecture
- Will use a client-server architecture with REST API
- Need authentication system, content management, and analytics
- Data should be persisted in a database with proper schemas
- Will follow MVC pattern for separation of concerns
```

**Poor Example (too detailed for Level 1):**
```
Task: Design web application architecture
- Will use React with Redux for state management
- Need to build login form with email validation
- Will use PostgreSQL with Sequelize ORM and define User table 
- Will add Jest for testing components
```

### Level 2: Isolation - Component Boundaries

**Abstraction Focus:** *Focus on interfaces and boundaries between components. Define clear inputs and outputs for each part.*

**Good Example:**
```
Tasks for Authentication System:
- API endpoints needed: register, login, verify, reset password
- Authentication will use JWT tokens
- User permissions system with roles
- Session management with appropriate timeouts
```

**Poor Example (inconsistent abstraction):**
```
Tasks for Authentication System:
- Need to create a users table in the database
- Write React component for login form with validation
- Use bcrypt for password hashing with 10 salt rounds
- Handle JWT token storage in localStorage or cookies
```

### Level 3: Ordering - Sequencing Components

**Abstraction Focus:** *Think about sequence and progression. Identify dependencies and build order without diving into implementation details.*

**Good Example:**
```
Authentication Implementation Order:
1. Set up user data model and database schema
2. Implement core authentication logic and JWT handling
3. Create API endpoints for authentication actions
4. Build frontend authentication components
5. Implement authorization middleware for protected routes
```

**Poor Example (mixed abstraction levels):**
```
Authentication Implementation Order:
1. Create PostgreSQL database and users table
2. Write login.jsx React component with Formik
3. Install jsonwebtoken package
4. Figure out how to validate emails
5. Create middleware/auth.js file
```

### Level 4: Implementation - Specific Tasks

**Abstraction Focus:** *Focus on concrete, actionable steps. Define specific code changes or artifacts to produce.*

**Good Example:**
```
Tasks for User Login Implementation:
1. Create LoginController.handleLogin method to validate credentials
2. Implement password comparison using bcrypt.compare()
3. Generate JWT token with appropriate claims and expiry
4. Return token and user data (excluding password) in response
```

**Poor Example (too vague for Level 4):**
```
Tasks for User Login Implementation:
1. Create login handler
2. Check password
3. Make token
4. Send response
```

## Using Abstraction Levels Effectively

### Tips for Navigating Between Levels

1. Start at Level 1 to establish direction and scope
2. Move to Level 2 to identify component boundaries 
3. Use Level 3 to sequence work for efficient implementation
4. Finally, drop to Level 4 for specific implementation tasks
5. Move back up levels when:
   - You encounter architectural problems
   - You need to reorganize priorities
   - You need to ensure components fit together

### Signs You're At The Wrong Abstraction Level

- **Too high:** Struggling to make progress, plans remain vague
- **Too low:** Getting bogged down in details, losing sight of overall goals
- **Inconsistent:** Mixing implementation details with architectural concerns 