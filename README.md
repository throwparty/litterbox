# Litterbox

Review *outputs*, not *actions*: give your AI agents litter trays to poop into.

> [!WARNING]
> Litterbox is in the proof of concept stage. Because it works by directly updating your Git index, bugs may cause data loss. Please use at your own risk.

<p align="center">
  <img src="logo.jpg" />
</p>

---

## Brief

Litterbox is a tool for sandboxing coding agents, allowing them to safely mutate code and execute commands without affecting the host system, or each other. As changes are made to the sandboxed working tree, Litterbox automatically stages the changes in your Git index for your review. Ports on the container can be forwarded to your host machine, allowing you to interact with APIs or frontends running in the sandbox.

## Installation

Download the appropriate binary from [the latest release](https://github.com/throwparty/litterbox/releases/latest), make it executable (`chmod +x litterbox`), and put it in a directory in your `PATH`.

## Configuration

Litterbox uses a three-layer TOML configuration system that merges three layers:

1. **Defaults** - Automatically derived from your environment
2. **Project configuration** (`.litterbox.toml`) - Committed to your repository
3. **Local overrides** (`.litterbox.local.toml`) - User-specific settings

### Layer 1: defaults

Litterbox automatically provides these defaults:

- `project.slug`: Derived from your current directory name (slugified)

### Layer 2: project configuration (`.litterbox.toml`)

Create a `.litterbox.toml` file in your project root:

```toml
[project]
slug = "my-project"  # Optional: override the default directory-based slug

[docker]
image = "ubuntu:latest"
setup-command = "echo 'Setup complete'"
```

### Layer 3: local overrides (`.litterbox.local.toml`)

For local development, create `.litterbox.local.toml`:

```toml
[docker]
image = "my-custom-image:v1.0"
```

Local settings override project settings, which override defaults.

### Required Keys

- `docker.image`: Docker image to use for sandboxes
- `docker.setup-command`: Command to run during sandbox setup

### Optional Keys

- `project.slug`: Unique identifier for the project (defaults to directory name)
