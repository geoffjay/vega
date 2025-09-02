# Agent Instructions Documentation

This document explains how to use agent instructions in the Ally AI system through `AGENTS.md` and `ALLY.md` files.

## Overview

Agent instructions are markdown files that provide context, guidelines, and behavioral instructions to AI agents. The Ally system supports two types of instruction files:

- **`AGENTS.md`**: General agent instructions for any AI agent
- **`ALLY.md`**: Ally-specific instructions that take priority when both files are present

## File Discovery and Priority

### Discovery Process

The Ally system automatically discovers instruction files using the following process:

1. **Start from current directory**: Begin searching in the current working directory
2. **Check for ALLY.md first**: Look for Ally-specific instructions (higher priority)
3. **Check for AGENTS.md**: Look for general agent instructions if ALLY.md not found
4. **Walk up directory tree**: Continue searching parent directories until a file is found or filesystem root is reached
5. **Return first match**: Use the first instruction file found in the search

### Priority Rules

- `ALLY.md` takes **absolute priority** over `AGENTS.md`
- If both files exist in the same directory, only `ALLY.md` is used
- Files in subdirectories take priority over files in parent directories
- The search stops at the first instruction file found

## File Structure and Content

### AGENTS.md Structure

The `AGENTS.md` file should contain general instructions that work with any AI agent. Here's the recommended structure:

```markdown
# AGENTS.md

## Project Overview

Brief description of the project, its purpose, and architecture.

## Setup Commands

Commands needed to build, run, and test the project.

## Code Style

Coding standards, formatting rules, and conventions.

## Testing Instructions

How to run tests, test structure, and testing guidelines.

## Tool Usage Guidelines

Instructions for using available tools effectively and safely.

## Security Considerations

Security practices and restrictions for the agent.

## Architecture Notes

Important architectural decisions and patterns.
```

### ALLY.md Structure

The `ALLY.md` file contains Ally-specific instructions and personality traits:

```markdown
# ALLY.md

## Ally-Specific Instructions

Instructions that are unique to the Ally agent.

## Ally's Personality

Personality traits and behavioral guidelines.

## Language Adaptation

Instructions for multilingual support and response adaptation.

## Ally-Specific Features

Features unique to Ally (context awareness, tool integration, etc.).

## Response Style Guidelines

How Ally should structure and format responses.

## Special Capabilities

Unique capabilities and use cases for Ally.
```

## Current Implementation

### AGENTS.md Content

The current `AGENTS.md` includes:

- **Project Overview**: Description of the Ally AI agent project built with Rust and Rig framework
- **Setup Commands**: Build (`cargo build`), run (`cargo run`), test (`cargo test`) commands
- **Code Style**: Rust 2024 edition, formatting with `cargo fmt`, functional programming patterns
- **Testing Instructions**: Various test execution commands and integration test location
- **Tool Usage Guidelines**: Best practices for each available tool (web search, file operations, etc.)
- **Security Considerations**: Safety guidelines and YOLO mode usage
- **Architecture Notes**: SQLite context storage, LLM provider support, Rig framework integration

### ALLY.md Content

The current `ALLY.md` includes:

- **Personality Traits**: Intelligent, thoughtful, kind, helpful, capable of deep reasoning
- **Language Adaptation**: Respond in the same language as the user
- **Enhanced Context Awareness**: Vector embeddings, conversation context maintenance
- **Tool Integration Philosophy**: Natural conversation flow, explanatory tool usage
- **Session Management**: Help with session switching and conversation export
- **Response Style Guidelines**: Conversational, reasoning-focused, proactive, encouraging
- **Special Capabilities**: Code analysis, architecture recommendations, debugging assistance

## Usage in Code

### AgentInstructionLoader

The `AgentInstructionLoader` class handles instruction file discovery and loading:

```rust
use crate::agent_instructions::{AgentInstructionLoader, InstructionFileType};

// Create loader from current directory
let loader = AgentInstructionLoader::new()?;

// Discover instructions automatically
let instructions = loader.discover_instructions()?;

// Load from specific path
let instructions = loader.load_from_path("path/to/ALLY.md")?;
```

### AgentInstructions Structure

```rust
pub struct AgentInstructions {
    pub content: String,           // Raw markdown content
    pub source_path: PathBuf,      // Path where instructions were found
    pub file_type: InstructionFileType, // AGENTS or ALLY
}

pub enum InstructionFileType {
    Agents,  // AGENTS.md file
    Ally,    // ALLY.md file
}
```

### Integration with System Prompts

Instructions are formatted for use in system prompts:

```rust
use crate::agent_instructions::format_instructions_for_prompt;

let formatted = format_instructions_for_prompt(&instructions);
// Returns formatted string with header and content ready for system prompt
```

## Best Practices

### Writing Effective Instructions

1. **Be Specific**: Provide clear, actionable guidelines rather than vague suggestions
2. **Use Examples**: Include concrete examples of desired behavior
3. **Organize Logically**: Structure content with clear headings and sections
4. **Keep Updated**: Regularly review and update instructions as the project evolves
5. **Test Instructions**: Verify that instructions produce the desired agent behavior

### AGENTS.md Best Practices

- Focus on **project-specific** information that any agent would need
- Include **technical details** about the codebase and architecture
- Provide **clear commands** for common tasks
- Document **security requirements** and restrictions
- Explain **tool usage patterns** and best practices

### ALLY.md Best Practices

- Define **personality traits** and communication style
- Specify **unique capabilities** that differentiate Ally
- Include **response formatting** preferences
- Document **special features** like context awareness
- Provide **interaction guidelines** for different scenarios

### File Organization

1. **Place at project root**: Instructions are most discoverable at the project root
2. **Use consistent naming**: Always use `AGENTS.md` and `ALLY.md` (case-sensitive)
3. **Maintain both files**: Keep both general and Ally-specific instructions
4. **Version control**: Include instruction files in version control
5. **Document changes**: Use commit messages to track instruction updates

## Advanced Usage

### Multiple Instruction Files

While the system uses only one instruction file per session, you can organize instructions hierarchically:

- **Root level**: General project instructions
- **Subdirectories**: Component-specific or feature-specific instructions
- **Development branches**: Branch-specific behavioral modifications

### Dynamic Instructions

Instructions can reference:

- **Environment variables**: For configuration-dependent behavior
- **File paths**: For project-specific tool usage
- **External resources**: Links to documentation or examples

### Instruction Templates

Create templates for common instruction patterns:

```markdown
# Template: Project Instructions

## Project: [PROJECT_NAME]

## Language: [PRIMARY_LANGUAGE]

## Framework: [FRAMEWORK_NAME]

## Setup Commands

- Build: [BUILD_COMMAND]
- Test: [TEST_COMMAND]
- Run: [RUN_COMMAND]

## Code Style

[STYLE_GUIDELINES]

## Architecture

[ARCHITECTURE_DESCRIPTION]
```

## Troubleshooting

### Common Issues

1. **Instructions not found**:

   - Check file naming (case-sensitive)
   - Verify file location in directory tree
   - Ensure file has proper markdown extension

2. **Wrong instructions loaded**:

   - Check file priority (ALLY.md vs AGENTS.md)
   - Verify current working directory
   - Review directory search path

3. **Instructions not applied**:
   - Verify file content is valid markdown
   - Check for parsing errors in logs
   - Ensure instructions are properly formatted

### Debugging

Enable verbose logging to see instruction discovery:

```bash
cargo run -- --verbose
```

Check logs for instruction loading messages:

- "Found ALLY.md at: [path]"
- "Found AGENTS.md at: [path]"
- "Loaded X bytes from [file] file: [path]"

### Validation

Test instruction loading programmatically:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_discovery() {
        let loader = AgentInstructionLoader::new().unwrap();
        let instructions = loader.discover_instructions().unwrap();

        assert!(instructions.is_some());
        let instructions = instructions.unwrap();
        assert!(!instructions.content.is_empty());
    }
}
```

## Integration Examples

### System Prompt Integration

```rust
async fn create_agent_with_instructions() -> Result<Agent> {
    let loader = AgentInstructionLoader::new()?;

    if let Some(instructions) = loader.discover_instructions()? {
        let formatted_instructions = format_instructions_for_prompt(&instructions);

        let agent = client
            .agent(model)
            .preamble(&format!("{}\n{}", base_preamble, formatted_instructions))
            .build();

        Ok(agent)
    } else {
        // Use default agent without custom instructions
        Ok(client.agent(model).preamble(base_preamble).build())
    }
}
```

### Context-Aware Loading

```rust
fn load_context_specific_instructions(context: &str) -> Result<Option<AgentInstructions>> {
    let loader = AgentInstructionLoader::new()?;

    // Try context-specific directory first
    let context_loader = AgentInstructionLoader::from_dir(format!("contexts/{}", context));
    if let Some(instructions) = context_loader.discover_instructions()? {
        return Ok(Some(instructions));
    }

    // Fall back to general instructions
    loader.discover_instructions()
}
```

This instruction system provides a flexible, hierarchical way to configure agent behavior while maintaining simplicity and discoverability.
