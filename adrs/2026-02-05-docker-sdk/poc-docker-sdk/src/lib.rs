use async_trait::async_trait;
use std::error::Error;
use std::path::Path;

pub type DockerResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct ContainerSpec {
    pub image: String,
    pub name: Option<String>,
    pub cmd: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerSummary {
    pub id: String,
    pub names: Vec<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[async_trait]
pub trait DockerClient {
    async fn pull_image(&self, image: &str) -> DockerResult<()>;
    async fn create_container(&self, spec: ContainerSpec) -> DockerResult<String>;
    async fn start_container(&self, container_id: &str) -> DockerResult<()>;
    async fn stop_container(&self, container_id: &str) -> DockerResult<()>;
    async fn remove_container(&self, container_id: &str, force: bool) -> DockerResult<()>;
    async fn list_containers(&self, all: bool) -> DockerResult<Vec<ContainerSummary>>;
    async fn exec(&self, container_id: &str, cmd: Vec<String>) -> DockerResult<ExecOutput>;
    async fn copy_file_into(
        &self,
        container_id: &str,
        host_path: &Path,
        container_path: &Path,
    ) -> DockerResult<()>;
    async fn copy_file_out(
        &self,
        container_id: &str,
        container_path: &Path,
        host_path: &Path,
    ) -> DockerResult<()>;
}

#[path = "lib/bollard.rs"]
pub mod bollard;
pub use bollard::BollardClient;

#[path = "lib/rs_docker.rs"]
pub mod rs_docker;
pub use rs_docker::RsDockerClient;

#[path = "lib/docker_wrapper.rs"]
pub mod docker_wrapper;
pub use docker_wrapper::DockerWrapperClient;
