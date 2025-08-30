# AGENTS Guidelines for this Project

This repository contains a Rust application located in the root of this repository. When
working on the project interactively with an agent (e.g. the Codex CLI) please follow
the guidelines below so that the development experience continues to work smoothly.

## 1. Use the Development Server, **not** `cargo build`

- **Always use `cargo run`** while iterating on the application

## 2. Keep Dependencies in Sync

If you add or update dependencies remember to:

1. Update the appropriate lockfile (`Cargo.lock`).
2. Re-build the project so that ally picks up the changes.

## 3. Coding Conventions

- Prefer Rust (`.rs`) for new components and utilities.
- Prefer modules for new functionality.

## 4. Useful Commands Recap

| Command     | Purpose                    |
| ----------- | -------------------------- |
| `cargo run` | Start the ally chat agent. |

---

Following these practices ensures that the agent-assisted development workflow stays
fast and dependable.
