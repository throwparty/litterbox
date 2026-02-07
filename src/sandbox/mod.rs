use std::io::Cursor;
use std::path::Path;

use futures_util::future::BoxFuture;
use tar::Archive;
use tempfile::TempDir;

use crate::compute::{Compute, ContainerSpec};
use crate::domain::{
    slugify_name,
    ComputeError,
    ExecutionResult,
    SandboxConfig,
    SandboxError,
    SandboxMetadata,
    SandboxStatus,
};
use crate::scm::Scm;

const DEFAULT_IMAGE: &str = "busybox:latest";
const DEFAULT_WORKDIR: &str = "/src";

pub trait SandboxProvider {
    fn create<'a>(
        &'a self,
        name: &'a str,
        config: &'a SandboxConfig,
    ) -> BoxFuture<'a, Result<SandboxMetadata, SandboxError>>;
    fn pause<'a>(&'a self, container_id: &'a str)
        -> BoxFuture<'a, Result<(), SandboxError>>;
    fn resume<'a>(&'a self, container_id: &'a str)
        -> BoxFuture<'a, Result<(), SandboxError>>;
    fn delete<'a>(&'a self, metadata: &'a SandboxMetadata)
        -> BoxFuture<'a, Result<(), SandboxError>>;
    fn shell<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        command: &'a [String],
    ) -> BoxFuture<'a, Result<ExecutionResult, SandboxError>>;
    fn upload_path<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        src_path: &'a Path,
        dest_path: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>>;
    fn download_path<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        src_path: &'a str,
        dest_path: &'a Path,
    ) -> BoxFuture<'a, Result<(), SandboxError>>;
}

pub struct DockerSandboxProvider<S, C> {
    scm: S,
    compute: C,
}

impl<S, C> DockerSandboxProvider<S, C> {
    pub fn new(scm: S, compute: C) -> Self {
        Self { scm, compute }
    }
}

pub fn container_name_for_slug(repo_prefix: &str, slug: &str) -> String {
    format!("litterbox-{}-{}", repo_prefix, slug)
}

pub fn branch_name_for_slug(slug: &str) -> String {
    format!("litterbox/{}", slug)
}

impl<S, C> SandboxProvider for DockerSandboxProvider<S, C>
where
    S: Scm + Send + Sync,
    C: Compute + Send + Sync,
{
    fn create<'a>(
        &'a self,
        name: &'a str,
        _config: &'a SandboxConfig,
    ) -> BoxFuture<'a, Result<SandboxMetadata, SandboxError>> {
        Box::pin(async move {
            let slug = slugify_name(name)?;
            let branch_name = self.scm.create_branch(&slug)?;
            let repo_prefix = self.scm.repo_prefix()?;
            let archive = match self.scm.make_archive("HEAD") {
                Ok(archive) => archive,
                Err(error) => {
                    let _ = self.scm.delete_branch(&slug);
                    return Err(error);
                }
            };
            let staged = match stage_archive(&archive) {
                Ok(staged) => staged,
                Err(error) => {
                    let _ = self.scm.delete_branch(&slug);
                    return Err(error);
                }
            };

            if let Err(error) = self.compute.ensure_image(DEFAULT_IMAGE).await {
                let _ = self.scm.delete_branch(&slug);
                return Err(error);
            }

            let spec = ContainerSpec {
                name: container_name_for_slug(&repo_prefix, &slug),
                image: DEFAULT_IMAGE.to_string(),
                command: vec!["sh".to_string(), "-c".to_string(), "tail -f /dev/null".to_string()],
                working_dir: Some(DEFAULT_WORKDIR.to_string()),
            };

            let container_id = match self.compute.create_container(&spec).await {
                Ok(id) => id,
                Err(error) => {
                    let _ = self.scm.delete_branch(&slug);
                    if is_container_name_conflict(&error) {
                        return Err(SandboxError::SandboxExists { name: slug.clone() });
                    }
                    return Err(error);
                }
            };

            if let Err(error) = self
                .compute
                .upload_path(&container_id, staged.path(), DEFAULT_WORKDIR)
                .await
            {
                let _ = self.compute.delete_container(&container_id).await;
                let _ = self.scm.delete_branch(&slug);
                return Err(error);
            }

            let startup_command = vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo hello world > /proc/1/fd/1".to_string(),
            ];
            let result = match self
                .compute
                .exec(&container_id, &startup_command, Some(DEFAULT_WORKDIR))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    let _ = self.compute.delete_container(&container_id).await;
                    let _ = self.scm.delete_branch(&slug);
                    return Err(error);
                }
            };

            if result.exit_code != 0 {
                let _ = self.compute.delete_container(&container_id).await;
                let _ = self.scm.delete_branch(&slug);
                let stderr = if result.stderr.is_empty() {
                    result.stdout
                } else {
                    result.stderr
                };
                return Err(SandboxError::SetupCommandFailed {
                    exit_code: result.exit_code,
                    stderr,
                });
            }

            Ok(SandboxMetadata {
                name: slug,
                branch_name,
                container_id,
                status: SandboxStatus::Active,
            })
        })
    }

    fn pause<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { self.compute.pause_container(container_id).await })
    }

    fn resume<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { self.compute.resume_container(container_id).await })
    }

    fn delete<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move {
            self.compute.delete_container(&metadata.container_id).await?;
            self.scm.delete_branch(&metadata.name)?;
            Ok(())
        })
    }

    fn shell<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        command: &'a [String],
    ) -> BoxFuture<'a, Result<ExecutionResult, SandboxError>> {
        Box::pin(async move {
            self.compute
                .exec(&metadata.container_id, command, Some(DEFAULT_WORKDIR))
                .await
        })
    }

    fn upload_path<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        src_path: &'a Path,
        dest_path: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move {
            self.compute
                .upload_path(&metadata.container_id, src_path, dest_path)
                .await
        })
    }

    fn download_path<'a>(
        &'a self,
        metadata: &'a SandboxMetadata,
        src_path: &'a str,
        dest_path: &'a Path,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move {
            self.compute
                .download_path(&metadata.container_id, src_path, dest_path)
                .await
        })
    }
}

fn stage_archive(archive: &[u8]) -> Result<TempDir, SandboxError> {
    let tempdir = TempDir::new()?;
    let mut archive = Archive::new(Cursor::new(archive));
    archive.unpack(tempdir.path())?;
    Ok(tempdir)
}

fn is_container_name_conflict(error: &SandboxError) -> bool {
    matches!(
        error,
        SandboxError::Compute(ComputeError::ContainerProvision {
            source: bollard::errors::Error::DockerResponseServerError { status_code: 409, .. }
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use bollard::query_parameters::RemoveContainerOptions;
    use git2::{IndexAddOption, Repository, Signature};
    use tempfile::TempDir;

    use crate::compute::DockerCompute;
    use crate::scm::ThreadSafeScm;

    static UNIQUE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn unique_suffix() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{nanos}-{counter}")
    }

    fn init_repo() -> (TempDir, Repository) {
        let tempdir = TempDir::new().expect("tempdir");
        let repo = Repository::init(tempdir.path()).expect("repo init");

        let file_path = tempdir.path().join("README.md");
        fs::write(&file_path, "hello").expect("write file");

        let mut index = repo.index().expect("index");
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .expect("add all");
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write tree");

        let signature = Signature::now("Litterbox", "noreply@example.com")
            .expect("signature");
        {
            let tree = repo.find_tree(tree_id).expect("find tree");
            repo.commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
                .expect("commit");
        }

        (tempdir, repo)
    }

    #[tokio::test]
    async fn create_provisions_container() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("LITTERBOX_DOCKER_TESTS").is_err() {
            return Ok(());
        }

        let (tempdir, _repo) = init_repo();
        let scm = ThreadSafeScm::open(tempdir.path())?;
        let compute = DockerCompute::connect()?;
        let provider = DockerSandboxProvider::new(scm, compute);

        let name = format!("sandbox-{}", unique_suffix());
        let metadata = provider.create(&name, &SandboxConfig { setup_command: None }).await?;

        let client = provider.compute.client();
        let container = client.inspect_container(&metadata.container_id, None).await?;
        let running = container
            .state
            .and_then(|state| state.running)
            .unwrap_or(false);
        assert!(running);

        let _ = client
            .remove_container(
                &metadata.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
        let _ = provider.scm.delete_branch(&metadata.name);

        Ok(())
    }

    #[tokio::test]
    async fn pause_resume_delete_container() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("LITTERBOX_DOCKER_TESTS").is_err() {
            return Ok(());
        }

        let (tempdir, _repo) = init_repo();
        let scm = ThreadSafeScm::open(tempdir.path())?;
        let compute = DockerCompute::connect()?;
        let provider = DockerSandboxProvider::new(scm, compute);

        let name = format!("sandbox-{}", unique_suffix());
        let metadata = provider.create(&name, &SandboxConfig { setup_command: None }).await?;

        provider.pause(&metadata.container_id).await?;
        let client = provider.compute.client();
        let container = client.inspect_container(&metadata.container_id, None).await?;
        let paused = container
            .state
            .and_then(|state| state.paused)
            .unwrap_or(false);
        assert!(paused);

        provider.resume(&metadata.container_id).await?;
        let container = client.inspect_container(&metadata.container_id, None).await?;
        let running = container
            .state
            .and_then(|state| state.running)
            .unwrap_or(false);
        assert!(running);

        provider.delete(&metadata).await?;
        assert!(client.inspect_container(&metadata.container_id, None).await.is_err());

        let repo = Repository::open(tempdir.path())?;
        assert!(repo
            .find_branch(&metadata.branch_name, git2::BranchType::Local)
            .is_err());

        Ok(())
    }

    #[tokio::test]
    async fn shell_executes_commands() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("LITTERBOX_DOCKER_TESTS").is_err() {
            return Ok(());
        }

        let (tempdir, _repo) = init_repo();
        let scm = ThreadSafeScm::open(tempdir.path())?;
        let compute = DockerCompute::connect()?;
        let provider = DockerSandboxProvider::new(scm, compute);

        let name = format!("sandbox-{}", unique_suffix());
        let metadata = provider.create(&name, &SandboxConfig { setup_command: None }).await?;

        let result = provider
            .shell(
                &metadata,
                &[
                    "sh".to_string(),
                    "-c".to_string(),
                    "echo hello".to_string(),
                ],
            )
            .await?;
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));

        let failure = provider
            .shell(
                &metadata,
                &[
                    "sh".to_string(),
                    "-c".to_string(),
                    "ls /does-not-exist".to_string(),
                ],
            )
            .await?;
        assert_ne!(failure.exit_code, 0);
        assert!(!failure.stderr.is_empty());

        provider.delete(&metadata).await?;
        Ok(())
    }

}
