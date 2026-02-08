use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

use bollard::container::LogOutput;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::{
    CreateContainerOptionsBuilder,
    CreateImageOptions,
    DownloadFromContainerOptionsBuilder,
    RemoveContainerOptions,
    UploadToContainerOptionsBuilder,
};
use bollard::body_full;
use bytes::Bytes;
use tar::{Archive, Builder};
use bollard::errors::Error as BollardError;
use bollard::{Docker, API_DEFAULT_VERSION};
use futures_util::future::BoxFuture;
use futures_util::StreamExt;

use crate::domain::{ComputeError, ExecutionResult, SandboxError};

pub trait Compute {
    fn ensure_image<'a>(&'a self, image: &'a str) -> BoxFuture<'a, Result<(), SandboxError>>;
    fn create_container<'a>(
        &'a self,
        spec: &'a ContainerSpec,
    ) -> BoxFuture<'a, Result<String, SandboxError>>;
    fn pause_container<'a>(&'a self, container_id: &'a str) -> BoxFuture<'a, Result<(), SandboxError>>;
    fn resume_container<'a>(&'a self, container_id: &'a str) -> BoxFuture<'a, Result<(), SandboxError>>;
    fn delete_container<'a>(&'a self, container_id: &'a str) -> BoxFuture<'a, Result<(), SandboxError>>;
    fn exec<'a>(
        &'a self,
        container_id: &'a str,
        command: &'a [String],
        working_dir: Option<&'a str>,
    ) -> BoxFuture<'a, Result<ExecutionResult, SandboxError>>;
    /// Copy a host path into the container at `dest_path`.
    fn upload_path<'a>(
        &'a self,
        container_id: &'a str,
        src_path: &'a Path,
        dest_path: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>>;
    /// Copy a container path into the host `dest_path` directory.
    fn download_path<'a>(
        &'a self,
        container_id: &'a str,
        src_path: &'a str,
        dest_path: &'a Path,
    ) -> BoxFuture<'a, Result<(), SandboxError>>;
}

#[derive(Clone, Debug)]
pub struct ContainerSpec {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

pub struct DockerCompute {
    client: Docker,
}

impl DockerCompute {
    pub fn new(client: Docker) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &Docker {
        &self.client
    }

    pub fn connect() -> Result<Self, SandboxError> {
        let client = connect_docker_client()?;
        Ok(Self { client })
    }

    fn connect_with_defaults() -> Result<Docker, SandboxError> {
        Docker::connect_with_local_defaults()
            .map_err(|source| SandboxError::Compute(ComputeError::Connection { source }))
    }

    pub async fn ensure_image(&self, image: &str) -> Result<(), SandboxError> {
        match self.client.inspect_image(image).await {
            Ok(_) => Ok(()),
            Err(error) if is_not_found(&error) => self.pull_image(image).await,
            Err(error) => Err(SandboxError::Compute(ComputeError::ImageInspect { source: error })),
        }
    }

    async fn pull_image(&self, image: &str) -> Result<(), SandboxError> {
        let options = Some(CreateImageOptions {
            from_image: Some(image.to_string()),
            ..Default::default()
        });
        let mut stream = self.client.create_image(options, None, None);

        while let Some(item) = stream.next().await {
            item.map_err(|source| SandboxError::Compute(ComputeError::ImagePull { source }))?;
        }

        Ok(())
    }

    pub async fn create_container(&self, spec: &ContainerSpec) -> Result<String, SandboxError> {
        let options = Some(
            CreateContainerOptionsBuilder::default()
                .name(&spec.name)
                .build(),
        );
        let config = ContainerCreateBody {
            image: Some(spec.image.clone()),
            cmd: if spec.command.is_empty() {
                None
            } else {
                Some(spec.command.clone())
            },
            working_dir: spec.working_dir.clone(),
            ..Default::default()
        };

        let response = self
            .client
            .create_container(options, config)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerProvision { source }))?;

        self.client
            .start_container(&response.id, None)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerProvision { source }))?;

        Ok(response.id)
    }

    pub async fn pause_container(&self, container_id: &str) -> Result<(), SandboxError> {
        match self.client.pause_container(container_id).await {
            Ok(()) => Ok(()),
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 409, .. }) => {
                Ok(())
            }
            Err(source) => Err(SandboxError::Compute(ComputeError::ContainerPause { source })),
        }
    }

    pub async fn resume_container(&self, container_id: &str) -> Result<(), SandboxError> {
        match self.client.unpause_container(container_id).await {
            Ok(()) => Ok(()),
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 409, .. }) => {
                Ok(())
            }
            Err(source) => Err(SandboxError::Compute(ComputeError::ContainerResume { source })),
        }
    }

    pub async fn delete_container(&self, container_id: &str) -> Result<(), SandboxError> {
        match self
            .client
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
                Ok(())
            }
            Err(source) => Err(SandboxError::Compute(ComputeError::ContainerDelete { source })),
        }
    }

    pub async fn exec(
        &self,
        container_id: &str,
        command: &[String],
        working_dir: Option<&str>,
    ) -> Result<ExecutionResult, SandboxError> {
        let command_args: Vec<&str> = command.iter().map(String::as_str).collect();
        let exec_options = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(command_args),
            working_dir,
            ..Default::default()
        };

        let exec = self
            .client
            .create_exec(container_id, exec_options)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerExec { source }))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let results = self
            .client
            .start_exec(&exec.id, None::<StartExecOptions>)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerExec { source }))?;

        if let StartExecResults::Attached { mut output, .. } = results {
            while let Some(item) = output.next().await {
                match item.map_err(|source| SandboxError::Compute(ComputeError::ContainerExec { source }))? {
                    LogOutput::StdOut { message } | LogOutput::Console { message } => {
                        stdout.extend_from_slice(&message)
                    }
                    LogOutput::StdErr { message } => stderr.extend_from_slice(&message),
                    LogOutput::StdIn { .. } => {}
                }
            }
        }

        let inspect = self
            .client
            .inspect_exec(&exec.id)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerExec { source }))?;
        let exit_code = inspect
            .exit_code
            .unwrap_or(1)
            .try_into()
            .unwrap_or(i32::MAX);

        Ok(ExecutionResult {
            exit_code,
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
        })
    }

    pub async fn upload_path(
        &self,
        container_id: &str,
        src_path: &Path,
        dest_path: &str,
    ) -> Result<(), SandboxError> {
        let tar = build_tar(src_path)?;
        self.upload_tar(container_id, dest_path, &tar).await
    }

    pub async fn download_path(
        &self,
        container_id: &str,
        src_path: &str,
        dest_path: &Path,
    ) -> Result<(), SandboxError> {
        let tar = self.download_tar(container_id, src_path).await?;
        extract_tar(dest_path, &tar)?;
        Ok(())
    }

    async fn upload_tar(
        &self,
        container_id: &str,
        dest_path: &str,
        tar: &[u8],
    ) -> Result<(), SandboxError> {
        let options = Some(
            UploadToContainerOptionsBuilder::default()
                .path(dest_path)
                .build(),
        );
        let body = body_full(Bytes::from(tar.to_vec()));
        self.client
            .upload_to_container(container_id, options, body)
            .await
            .map_err(|source| SandboxError::Compute(ComputeError::ContainerUpload { source }))
    }

    async fn download_tar(
        &self,
        container_id: &str,
        src_path: &str,
    ) -> Result<Vec<u8>, SandboxError> {
        let options = Some(
            DownloadFromContainerOptionsBuilder::default()
                .path(src_path)
                .build(),
        );
        let mut stream = self.client.download_from_container(container_id, options);
        let mut buffer = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|source| SandboxError::Compute(ComputeError::ContainerDownload { source }))?;
            buffer.extend_from_slice(&chunk);
        }
        Ok(buffer)
    }
}

fn connect_docker_client() -> Result<Docker, SandboxError> {
    if let Some(host) = docker_host_from_context() {
        return connect_with_host(&host);
    }
    DockerCompute::connect_with_defaults()
}

fn connect_with_host(host: &str) -> Result<Docker, SandboxError> {
    let (scheme, rest) = match host.split_once("://") {
        Some((scheme, rest)) => (scheme, rest),
        None => ("unix", host),
    };

    match scheme {
        "unix" => Docker::connect_with_unix(rest, 120, API_DEFAULT_VERSION)
            .map_err(|source| SandboxError::Compute(ComputeError::Connection { source })),
        "tcp" => {
            let endpoint = format!("http://{}", rest);
            Docker::connect_with_http(&endpoint, 120, API_DEFAULT_VERSION)
                .map_err(|source| SandboxError::Compute(ComputeError::Connection { source }))
        }
        _ => DockerCompute::connect_with_defaults(),
    }
}

fn docker_host_from_context() -> Option<String> {
    let output = Command::new("docker")
        .args([
            "context",
            "inspect",
            "-f",
            "{{.Endpoints.docker.Host}}",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let host = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if host.is_empty() {
        return None;
    }
    Some(host)
}

impl Compute for DockerCompute {
    fn ensure_image<'a>(&'a self, image: &'a str) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { DockerCompute::ensure_image(self, image).await })
    }

    fn create_container<'a>(
        &'a self,
        spec: &'a ContainerSpec,
    ) -> BoxFuture<'a, Result<String, SandboxError>> {
        Box::pin(async move { DockerCompute::create_container(self, spec).await })
    }

    fn pause_container<'a>(&'a self, container_id: &'a str) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { DockerCompute::pause_container(self, container_id).await })
    }

    fn resume_container<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { DockerCompute::resume_container(self, container_id).await })
    }

    fn delete_container<'a>(&'a self, container_id: &'a str) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move { DockerCompute::delete_container(self, container_id).await })
    }

    fn exec<'a>(
        &'a self,
        container_id: &'a str,
        command: &'a [String],
        working_dir: Option<&'a str>,
    ) -> BoxFuture<'a, Result<ExecutionResult, SandboxError>> {
        Box::pin(async move { DockerCompute::exec(self, container_id, command, working_dir).await })
    }

    fn upload_path<'a>(
        &'a self,
        container_id: &'a str,
        src_path: &'a Path,
        dest_path: &'a str,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move {
            DockerCompute::upload_path(self, container_id, src_path, dest_path).await
        })
    }

    fn download_path<'a>(
        &'a self,
        container_id: &'a str,
        src_path: &'a str,
        dest_path: &'a Path,
    ) -> BoxFuture<'a, Result<(), SandboxError>> {
        Box::pin(async move {
            DockerCompute::download_path(self, container_id, src_path, dest_path).await
        })
    }
}

fn build_tar(src_path: &Path) -> Result<Vec<u8>, SandboxError> {
    let mut builder = Builder::new(Vec::new());
    if src_path.is_dir() {
        append_dir(&mut builder, src_path, src_path)?;
    } else {
        let name = src_path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
        builder.append_path_with_name(src_path, name)?;
    }
    builder.finish()?;
    Ok(builder.into_inner()?)
}

fn append_dir(builder: &mut Builder<Vec<u8>>, root: &Path, dir: &Path) -> Result<(), SandboxError> {
    let mut entries = fs::read_dir(dir)?;
    let mut has_entries = false;

    while let Some(entry) = entries.next() {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
        has_entries = true;

        if path.is_dir() {
            builder.append_dir(relative, &path)?;
            append_dir(builder, root, &path)?;
        } else if path.is_file() {
            builder.append_path_with_name(&path, relative)?;
        }
    }

    if !has_entries {
        let relative = dir
            .strip_prefix(root)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
        if !relative.as_os_str().is_empty() {
            builder.append_dir(relative, dir)?;
        }
    }

    Ok(())
}

fn extract_tar(dest_path: &Path, tar: &[u8]) -> Result<(), SandboxError> {
    fs::create_dir_all(dest_path)?;
    let mut archive = Archive::new(Cursor::new(tar));
    archive.unpack(dest_path)?;
    Ok(())
}

fn is_not_found(error: &BollardError) -> bool {
    matches!(error, BollardError::DockerResponseServerError { status_code: 404, .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn docker_connects_and_ensures_image() -> Result<(), Box<dyn std::error::Error>> {
        // Requires a running Docker daemon; opt in with LITTERBOX_DOCKER_TESTS.
        if std::env::var("LITTERBOX_DOCKER_TESTS").is_err() {
            return Ok(());
        }

        let compute = DockerCompute::connect()?;
        compute.ensure_image("busybox:latest").await?;
        Ok(())
    }
}
