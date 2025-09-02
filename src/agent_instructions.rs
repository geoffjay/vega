//! # Agent Instructions System
//!
//! This module provides functionality for discovering, loading, and managing
//! agent instruction files (`AGENTS.md` and `ALLY.md`). These files contain
//! behavioral guidelines, project context, and configuration for AI agents.
//!
//! ## File Types
//!
//! - **`AGENTS.md`**: General agent instructions that work with any AI agent
//! - **`ALLY.md`**: Vega-specific instructions that take priority when present
//!
//! ## Discovery Process
//!
//! The system automatically searches for instruction files by:
//! 1. Starting from the current working directory
//! 2. Looking for `ALLY.md` first (higher priority)
//! 3. Looking for `AGENTS.md` if `ALLY.md` not found
//! 4. Walking up the directory tree until a file is found or root is reached
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use vega::agent_instructions::{AgentInstructionLoader, format_instructions_for_prompt};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let loader = AgentInstructionLoader::new()?;
//!     
//!     if let Some(instructions) = loader.discover_instructions()? {
//!         let formatted = format_instructions_for_prompt(&instructions);
//!         println!("Found instructions: {}", formatted);
//!     }
//!     
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Represents agent instruction content loaded from AGENTS.md or ALLY.md files.
///
/// This structure contains the raw markdown content along with metadata about
/// where the instructions were found and what type of file they came from.
#[derive(Debug, Clone)]
pub struct AgentInstructions {
    /// The raw markdown content of the instruction file
    pub content: String,
    /// The path where the instructions were found
    pub source_path: PathBuf,
    /// Whether this came from AGENTS.md or ALLY.md
    pub file_type: InstructionFileType,
}

/// The type of instruction file found
#[derive(Debug, Clone, PartialEq)]
pub enum InstructionFileType {
    /// Standard AGENTS.md file
    Agents,
    /// Vega-specific VEGA.md file
    Vega,
}

impl InstructionFileType {
    /// Get the filename for this instruction type
    pub fn filename(&self) -> &'static str {
        match self {
            InstructionFileType::Agents => "AGENTS.md",
            InstructionFileType::Vega => "VEGA.md",
        }
    }
}

/// Discovers and loads agent instruction files from the current working directory and parent directories
pub struct AgentInstructionLoader {
    /// The starting directory for the search
    start_dir: PathBuf,
}

impl AgentInstructionLoader {
    /// Create a new loader starting from the current working directory
    pub fn new() -> Result<Self> {
        let start_dir = env::current_dir().context("Failed to get current working directory")?;
        Ok(Self { start_dir })
    }

    /// Create a new loader starting from a specific directory
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            start_dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Discover and load agent instructions from the directory tree
    ///
    /// This method searches for AGENTS.md and ALLY.md files starting from the current
    /// directory and walking up the directory tree. It prioritizes ALLY.md over AGENTS.md
    /// and returns the first instruction file found.
    ///
    /// The search follows these rules:
    /// 1. Start from the current working directory
    /// 2. Look for ALLY.md first, then AGENTS.md in each directory
    /// 3. Walk up the directory tree until a file is found or root is reached
    /// 4. Return the first instruction file found
    pub fn discover_instructions(&self) -> Result<Option<AgentInstructions>> {
        let mut current_dir = self.start_dir.clone();

        loop {
            debug!(
                "Searching for instruction files in: {}",
                current_dir.display()
            );

            // Check for VEGA.md first (Vega-specific takes priority)
            let vega_path = current_dir.join("VEGA.md");
            if vega_path.exists() && vega_path.is_file() {
                info!("Found VEGA.md at: {}", vega_path.display());
                return self
                    .load_instruction_file(&vega_path, InstructionFileType::Vega)
                    .map(Some);
            }

            // Check for AGENTS.md
            let agents_path = current_dir.join("AGENTS.md");
            if agents_path.exists() && agents_path.is_file() {
                info!("Found AGENTS.md at: {}", agents_path.display());
                return self
                    .load_instruction_file(&agents_path, InstructionFileType::Agents)
                    .map(Some);
            }

            // Move to parent directory
            match current_dir.parent() {
                Some(parent) => {
                    current_dir = parent.to_path_buf();
                }
                None => {
                    debug!("Reached filesystem root, no instruction files found");
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Load a specific instruction file
    fn load_instruction_file(
        &self,
        path: &Path,
        file_type: InstructionFileType,
    ) -> Result<AgentInstructions> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read instruction file: {}", path.display()))?;

        if content.trim().is_empty() {
            warn!("Instruction file is empty: {}", path.display());
        }

        debug!(
            "Loaded {} bytes from {} file: {}",
            content.len(),
            file_type.filename(),
            path.display()
        );

        Ok(AgentInstructions {
            content,
            source_path: path.to_path_buf(),
            file_type,
        })
    }

    /// Load instructions from a specific file path
    pub fn load_from_path<P: AsRef<Path>>(&self, path: P) -> Result<AgentInstructions> {
        let path = path.as_ref();
        let file_type = match path.file_name().and_then(|n| n.to_str()) {
            Some("VEGA.md") => InstructionFileType::Vega,
            Some("AGENTS.md") => InstructionFileType::Agents,
            _ => {
                // Default to AGENTS type for unknown files
                InstructionFileType::Agents
            }
        };

        self.load_instruction_file(path, file_type)
    }
}

impl Default for AgentInstructionLoader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            start_dir: PathBuf::from("."),
        })
    }
}

/// Extract and format agent instructions for use in system prompts
pub fn format_instructions_for_prompt(instructions: &AgentInstructions) -> String {
    let mut formatted = String::new();

    // Add header indicating the source
    formatted.push_str(&format!(
        "\n# Agent Instructions (from {})\n\n",
        instructions.source_path.display()
    ));

    // Add the raw content
    formatted.push_str(&instructions.content);

    // Ensure there's a newline at the end
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }

    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_instruction_file_type_filename() {
        assert_eq!(InstructionFileType::Agents.filename(), "AGENTS.md");
        assert_eq!(InstructionFileType::Vega.filename(), "VEGA.md");
    }

    #[test]
    fn test_agent_instruction_loader_creation() {
        let loader = AgentInstructionLoader::new();
        assert!(loader.is_ok());
    }

    #[test]
    fn test_agent_instruction_loader_from_dir() {
        let temp_dir = tempdir().unwrap();
        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        assert_eq!(loader.start_dir, temp_dir.path());
    }

    #[test]
    fn test_discover_instructions_no_files() {
        let temp_dir = tempdir().unwrap();
        let loader = AgentInstructionLoader::from_dir(temp_dir.path());

        let result = loader.discover_instructions().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_instructions_vega_md() {
        let temp_dir = tempdir().unwrap();
        let vega_path = temp_dir.path().join("VEGA.md");
        fs::write(&vega_path, "# Vega Instructions\n\nThis is a test.").unwrap();

        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        let result = loader.discover_instructions().unwrap();

        assert!(result.is_some());
        let instructions = result.unwrap();
        assert_eq!(instructions.file_type, InstructionFileType::Vega);
        assert_eq!(instructions.source_path, vega_path);
        assert!(instructions.content.contains("Vega Instructions"));
    }

    #[test]
    fn test_discover_instructions_agents_md() {
        let temp_dir = tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Agent Instructions\n\nThis is a test.").unwrap();

        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        let result = loader.discover_instructions().unwrap();

        assert!(result.is_some());
        let instructions = result.unwrap();
        assert_eq!(instructions.file_type, InstructionFileType::Agents);
        assert_eq!(instructions.source_path, agents_path);
        assert!(instructions.content.contains("Agent Instructions"));
    }

    #[test]
    fn test_discover_instructions_priority() {
        let temp_dir = tempdir().unwrap();

        // Create both files
        let vega_path = temp_dir.path().join("VEGA.md");
        let agents_path = temp_dir.path().join("AGENTS.md");
        fs::write(&vega_path, "# Vega Instructions").unwrap();
        fs::write(&agents_path, "# Agent Instructions").unwrap();

        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        let result = loader.discover_instructions().unwrap();

        assert!(result.is_some());
        let instructions = result.unwrap();
        // ALLY.md should take priority
        assert_eq!(instructions.file_type, InstructionFileType::Vega);
        assert_eq!(instructions.source_path, vega_path);
    }

    #[test]
    fn test_discover_instructions_parent_directory() {
        let temp_dir = tempdir().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        // Create AGENTS.md in parent directory
        let agents_path = temp_dir.path().join("AGENTS.md");
        fs::write(&agents_path, "# Parent Agent Instructions").unwrap();

        // Search from subdirectory
        let loader = AgentInstructionLoader::from_dir(&sub_dir);
        let result = loader.discover_instructions().unwrap();

        assert!(result.is_some());
        let instructions = result.unwrap();
        assert_eq!(instructions.file_type, InstructionFileType::Agents);
        assert_eq!(instructions.source_path, agents_path);
        assert!(instructions.content.contains("Parent Agent Instructions"));
    }

    #[test]
    fn test_load_from_path() {
        let temp_dir = tempdir().unwrap();
        let vega_path = temp_dir.path().join("VEGA.md");
        fs::write(&vega_path, "# Custom Vega Instructions").unwrap();

        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        let result = loader.load_from_path(&vega_path).unwrap();

        assert_eq!(result.file_type, InstructionFileType::Vega);
        assert_eq!(result.source_path, vega_path);
        assert!(result.content.contains("Custom Vega Instructions"));
    }

    #[test]
    fn test_format_instructions_for_prompt() {
        let temp_dir = tempdir().unwrap();
        let vega_path = temp_dir.path().join("VEGA.md");

        let instructions = AgentInstructions {
            content: "# Test Instructions\n\nThis is a test.".to_string(),
            source_path: vega_path.clone(),
            file_type: InstructionFileType::Vega,
        };

        let formatted = format_instructions_for_prompt(&instructions);

        assert!(formatted.contains("Agent Instructions"));
        assert!(formatted.contains(&vega_path.display().to_string()));
        assert!(formatted.contains("Test Instructions"));
        assert!(formatted.contains("This is a test."));
        assert!(formatted.ends_with('\n'));
    }

    #[test]
    fn test_load_empty_file() {
        let temp_dir = tempdir().unwrap();
        let empty_path = temp_dir.path().join("AGENTS.md");
        fs::write(&empty_path, "").unwrap();

        let loader = AgentInstructionLoader::from_dir(temp_dir.path());
        let result = loader.load_from_path(&empty_path).unwrap();

        assert_eq!(result.content, "");
        assert_eq!(result.file_type, InstructionFileType::Agents);
    }
}
