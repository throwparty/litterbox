# Welcome, robots

We're building Litterbox, a tool for sandboxing agents doing software engineering. Our project has a few goals:

- Interoperate by use of MCP, the Model Control Protocol.
- Minimise external dependencies. We should need only a container runtime and an image to spawn in it.

# Stack

- Nix for dependency management
- Rust
  - `bollard` as Docker client
  - `clap` for CLI
  - `git2` for Git
  - `rmcp` for MCP

# Core concepts

- Sandboxes are pairs of SCM branches and some form of compute. SCM branches are the authoritative source of information about running sandboxes, and containers may be spun up from existing SCM branches in case of accidental deletion.
