use std::path::Path;
use std::process::ExitCode;

use bollard::query_parameters::ListContainersOptionsBuilder;
use clap::{Parser, Subcommand};
use litterbox::compute::DockerCompute;
use litterbox::domain::{ComputeError, slugify_name, SandboxError, SandboxMetadata, SandboxStatus};
use litterbox::mcp;
use litterbox::sandbox::{
    branch_name_for_slug,
    container_name_for_slug,
    DockerSandboxProvider,
    SandboxProvider,
};
use litterbox::scm::{Scm, ThreadSafeScm};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Stdio,
    Pause {
        #[arg(required_unless_present_any = ["all_envs", "all_repos"])]
        name: Option<String>,
        #[arg(long, conflicts_with = "all_repos")]
        all_envs: bool,
        #[arg(long, conflicts_with = "all_envs")]
        all_repos: bool,
    },
    Resume { name: String },
    Delete {
        name: String,
        #[arg(short, long)]
        force: bool,
    },
    Shell {
        name: String,
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => handle_list().await,
        Commands::Stdio => handle_stdio().await,
        Commands::Pause {
            name,
            all_envs,
            all_repos,
        } => handle_pause(name, all_envs, all_repos).await,
        Commands::Resume { name } => handle_resume(name).await,
        Commands::Delete { name, force } => handle_delete(name, force).await,
        Commands::Shell { name, command } => handle_shell(name, command).await,
    }
}

async fn handle_stdio() -> ExitCode {
    if let Err(error) = mcp::run_stdio().await {
        return report_error("stdio", error);
    }
    ExitCode::from(0)
}

async fn handle_list() -> ExitCode {
    let scm = match ThreadSafeScm::open(Path::new(".")) {
        Ok(scm) => scm,
        Err(error) => return report_error("list", error),
    };
    let repo_prefix = match scm.repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("list", error),
    };
    let slugs = match scm.list_sandboxes() {
        Ok(slugs) => slugs,
        Err(error) => return report_error("list", error),
    };

    let compute = match DockerCompute::connect() {
        Ok(compute) => Some(compute),
        Err(_) => {
            eprintln!("list warning: docker unavailable; statuses shown as unknown");
            None
        }
    };

    let mut sandboxes = Vec::new();
    for slug in slugs {
        let status = match compute.as_ref() {
            Some(compute) => {
                let container = container_name_for_slug(&repo_prefix, &slug);
                match compute.client().inspect_container(&container, None).await {
                    Ok(info) => {
                        let state = info.state.as_ref();
                        let running = state.and_then(|state| state.running).unwrap_or(false);
                        let paused = state.and_then(|state| state.paused).unwrap_or(false);
                        if paused {
                            SandboxStatus::Paused
                        } else if running {
                            SandboxStatus::Active
                        } else {
                            SandboxStatus::Error("not running".to_string())
                        }
                    }
                    Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
                        SandboxStatus::Error("missing container".to_string())
                    }
                    Err(error) => return report_error("list", error),
                }
            }
            None => SandboxStatus::Error("docker unavailable".to_string()),
        };
        sandboxes.push(metadata_for_slug(&repo_prefix, &slug, status));
    }

    sandboxes.sort_by(|a, b| a.name.cmp(&b.name));
    for sandbox in sandboxes {
        println!("{} {}", sandbox.name, status_label(&sandbox.status));
    }

    ExitCode::from(0)
}

async fn handle_pause(name: Option<String>, all_envs: bool, all_repos: bool) -> ExitCode {
    if all_repos {
        return handle_pause_all_repos().await;
    }
    if all_envs {
        return handle_pause_all_envs().await;
    }

    let Some(name) = name else {
        return report_error("pause", "missing sandbox name");
    };
    let slug = match slugify_name(&name) {
        Ok(slug) => slug,
        Err(error) => return report_error("pause", error),
    };
    let repo_prefix = match repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("pause", error),
    };
    let container = container_name_for_slug(&repo_prefix, &slug);
    let provider = match build_provider() {
        Ok(provider) => provider,
        Err(error) => return report_error("pause", error),
    };
    if let Err(error) = provider.pause(&container).await {
        return report_error("pause", error);
    }
    let metadata = metadata_for_slug(&repo_prefix, &slug, SandboxStatus::Paused);
    println!("Paused {metadata}");
    ExitCode::from(0)
}

async fn handle_pause_all_envs() -> ExitCode {
    let scm = match ThreadSafeScm::open(Path::new(".")) {
        Ok(scm) => scm,
        Err(error) => return report_error("pause --all-envs", error),
    };
    let repo_prefix = match scm.repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("pause --all-envs", error),
    };
    let compute = match DockerCompute::connect() {
        Ok(compute) => compute,
        Err(error) => return report_error("pause --all-envs", error),
    };
    let slugs = match scm.list_sandboxes() {
        Ok(slugs) => slugs,
        Err(error) => return report_error("pause --all-envs", error),
    };

    let mut paused = 0usize;
    for slug in slugs {
        let container = container_name_for_slug(&repo_prefix, &slug);
        match compute.pause_container(&container).await {
            Ok(()) => paused += 1,
            Err(error) if is_container_missing(&error) => {}
            Err(error) => return report_error("pause --all-envs", error),
        }
    }

    println!("Paused {paused} sandbox(es)");
    ExitCode::from(0)
}

async fn handle_pause_all_repos() -> ExitCode {
    let compute = match DockerCompute::connect() {
        Ok(compute) => compute,
        Err(error) => return report_error("pause --all-repos", error),
    };
    let options = Some(ListContainersOptionsBuilder::default().all(true).build());
    let containers = match compute.client().list_containers(options).await {
        Ok(containers) => containers,
        Err(error) => return report_error("pause --all-repos", error),
    };

    let mut paused = 0usize;
    for container in containers {
        let Some(names) = container.names.as_ref() else {
            continue;
        };
        if !names.iter().any(|name| name.starts_with("/litterbox-")) {
            continue;
        }
        let Some(id) = container.id.as_ref() else {
            continue;
        };
        let running = matches!(container.state, Some(bollard::models::ContainerSummaryStateEnum::RUNNING));
        if !running {
            continue;
        }
        match compute.pause_container(id).await {
            Ok(()) => paused += 1,
            Err(error) if is_container_missing(&error) => {}
            Err(error) => return report_error("pause --all-repos", error),
        }
    }

    println!("Paused {paused} sandbox(es)");
    ExitCode::from(0)
}

async fn handle_resume(name: String) -> ExitCode {
    let slug = match slugify_name(&name) {
        Ok(slug) => slug,
        Err(error) => return report_error("resume", error),
    };
    let repo_prefix = match repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("resume", error),
    };
    let container = container_name_for_slug(&repo_prefix, &slug);
    let provider = match build_provider() {
        Ok(provider) => provider,
        Err(error) => return report_error("resume", error),
    };
    if let Err(error) = provider.resume(&container).await {
        return report_error("resume", error);
    }
    let metadata = metadata_for_slug(&repo_prefix, &slug, SandboxStatus::Active);
    println!("Resumed {metadata}");
    ExitCode::from(0)
}

async fn handle_delete(name: String, force: bool) -> ExitCode {
    let slug = match slugify_name(&name) {
        Ok(slug) => slug,
        Err(error) => return report_error("delete", error),
    };
    let repo_prefix = match repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("delete", error),
    };
    let container = container_name_for_slug(&repo_prefix, &slug);
    let compute = match DockerCompute::connect() {
        Ok(compute) => compute,
        Err(error) => return report_error("delete", error),
    };

    match compute.client().inspect_container(&container, None).await {
        Ok(info) => {
            let running = info
                .state
                .as_ref()
                .and_then(|state| state.running)
                .unwrap_or(false);
            let paused = info
                .state
                .as_ref()
                .and_then(|state| state.paused)
                .unwrap_or(false);
            if running && !paused && !force {
                return report_error("delete", "sandbox is active; use --force to delete");
            }
        }
        Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {}
        Err(error) => return report_error("delete", error),
    }

    let provider = match build_provider() {
        Ok(provider) => provider,
        Err(error) => return report_error("delete", error),
    };
    let metadata = metadata_for_slug(&repo_prefix, &slug, SandboxStatus::Active);
    if let Err(error) = provider.delete(&metadata).await {
        return report_error("delete", error);
    }

    println!("Deleted {name}");
    ExitCode::from(0)
}

async fn handle_shell(name: String, command: Vec<String>) -> ExitCode {
    let slug = match slugify_name(&name) {
        Ok(slug) => slug,
        Err(error) => return report_error("shell", error),
    };
    let provider = match build_provider() {
        Ok(provider) => provider,
        Err(error) => return report_error("shell", error),
    };
    let repo_prefix = match repo_prefix() {
        Ok(prefix) => prefix,
        Err(error) => return report_error("shell", error),
    };
    let metadata = metadata_for_slug(&repo_prefix, &slug, SandboxStatus::Active);

    let result = match provider.shell(&metadata, &command).await {
        Ok(result) => result,
        Err(error) => return report_error("shell", error),
    };

    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprint!("{}", result.stderr);
    }
    if result.exit_code != 0 {
        eprintln!("shell failed: {result}");
    }

    if result.exit_code == 0 {
        ExitCode::from(0)
    } else if let Ok(code) = u8::try_from(result.exit_code) {
        ExitCode::from(code)
    } else {
        ExitCode::from(1)
    }
}

fn build_provider() -> Result<DockerSandboxProvider<ThreadSafeScm, DockerCompute>, SandboxError> {
    let scm = ThreadSafeScm::open(Path::new("."))?;
    let compute = DockerCompute::connect()?;
    Ok(DockerSandboxProvider::new(scm, compute))
}

fn metadata_for_slug(repo_prefix: &str, slug: &str, status: SandboxStatus) -> SandboxMetadata {
    SandboxMetadata {
        name: slug.to_string(),
        branch_name: branch_name_for_slug(slug),
        container_id: container_name_for_slug(repo_prefix, slug),
        status,
    }
}

fn report_error(action: &str, error: impl std::fmt::Display) -> ExitCode {
    eprintln!("{action} failed: {error}");
    ExitCode::from(1)
}

fn repo_prefix() -> Result<String, SandboxError> {
    ThreadSafeScm::open(Path::new("."))?.repo_prefix()
}

fn status_label(status: &SandboxStatus) -> String {
    match status {
        SandboxStatus::Active => "active".to_string(),
        SandboxStatus::Paused => "paused".to_string(),
        SandboxStatus::Error(message) if message == "missing container" => "missing".to_string(),
        SandboxStatus::Error(message) if message == "docker unavailable" => "unknown".to_string(),
        SandboxStatus::Error(message) => format!("error: {message}"),
    }
}

fn is_container_missing(error: &SandboxError) -> bool {
    matches!(
        error,
        SandboxError::Compute(ComputeError::ContainerPause {
            source: bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }
        })
    )
}
