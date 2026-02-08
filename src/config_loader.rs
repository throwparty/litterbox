use std::fs;
use std::path::Path;

use crate::config::{Config, ConfigError};

/// Loads and parses a single TOML configuration file into a Config struct.
pub fn load_file(path: &Path) -> Result<Config, ConfigError> {
    let contents = fs::read_to_string(path)
        .map_err(|_| ConfigError::FileNotFound(path.to_path_buf()))?;

    toml::from_str(&contents)
        .map_err(|e| ConfigError::ParseError(e.to_string()))
}

/// Merges two Config structs, with values from `local` overriding `base`.
pub fn merge(base: Config, local: Config) -> Config {
    Config {
        project: crate::config::ProjectConfig {
            slug: local.project.slug.or(base.project.slug),
        },
        docker: crate::config::DockerConfig {
            image: local.docker.image.or(base.docker.image),
            setup_command: local.docker.setup_command.or(base.docker.setup_command),
        },
    }
}

/// Creates a default configuration based on the current directory.
fn default_config() -> Config {
    let current_dir = std::env::current_dir().ok();
    let project_slug = current_dir
        .as_ref()
        .and_then(|dir| dir.file_name())
        .and_then(|name| name.to_str())
        .map(|name| crate::domain::slugify(name))
        .filter(|slug| !slug.is_empty());

    Config {
        project: crate::config::ProjectConfig {
            slug: project_slug,
        },
        docker: crate::config::DockerConfig {
            image: None,
            setup_command: None,
        },
    }
}

/// Loads the final merged configuration from defaults, .litterbox.toml, and .litterbox.local.toml.
pub fn load_final() -> Result<Config, ConfigError> {
    // Start with defaults
    let defaults = default_config();

    // Load project config
    let base_path = Path::new(".litterbox.toml");
    let base_config = load_file(base_path)?;

    // Load local config if it exists
    let local_path = Path::new(".litterbox.local.toml");
    let local_config = if local_path.exists() {
        load_file(local_path)?
    } else {
        // Empty config for merging
        Config {
            project: crate::config::ProjectConfig { slug: None },
            docker: crate::config::DockerConfig {
                image: None,
                setup_command: None,
            },
        }
    };

    // Merge: defaults <- project <- local
    let merged = merge(merge(defaults, base_config), local_config);

    // Validate required keys
    if merged.docker.image.as_deref().unwrap_or("").is_empty() {
        return Err(ConfigError::MissingRequiredKey("docker.image".to_string()));
    }
    if merged.docker.setup_command.as_deref().unwrap_or("").is_empty() {
        return Err(ConfigError::MissingRequiredKey("docker.setup-command".to_string()));
    }

    Ok(merged)
}
