use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool,
    tool_handler,
    tool_router,
    transport::stdio,
    ErrorData as McpError,
    ServerHandler,
    ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::config_loader;
use crate::domain::{SandboxConfig, SandboxError};
use crate::sandbox::{DockerSandboxProvider, SandboxProvider};
use crate::scm::ThreadSafeScm;
use crate::compute::DockerCompute;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SandboxCreateArgs {
    pub name: String,
}

#[derive(Clone)]
pub struct SandboxServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SandboxServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "sandbox-create", description = "Create a new sandbox based on the current repository HEAD")]
    async fn sandbox_create(
        &self,
        Parameters(args): Parameters<SandboxCreateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let provider = build_provider().map_err(map_error)?;
        let loaded_config = config_loader::load_final()
            .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;
        let config = SandboxConfig {
            image: loaded_config.docker.image.unwrap(),
            setup_command: loaded_config.docker.setup_command,
        };
        let metadata = provider.create(&args.name, &config).await.map_err(map_error)?;
        let content = Content::json(metadata)
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![content]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SandboxServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Litterbox sandbox management".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let service = SandboxServer::new().serve(stdio()).await.inspect_err(|e| {
        eprintln!("Error starting MCP server: {e}");
    })?;
    service.waiting().await?;
    Ok(())
}

fn build_provider() -> Result<DockerSandboxProvider<ThreadSafeScm, DockerCompute>, SandboxError> {
    let scm = ThreadSafeScm::open(std::path::Path::new("."))?;
    let compute = DockerCompute::connect()?;
    Ok(DockerSandboxProvider::new(scm, compute))
}

fn map_error(error: SandboxError) -> McpError {
    match error {
        SandboxError::InvalidName { .. } => McpError::invalid_params(error.to_string(), None),
        SandboxError::SandboxExists { .. } => McpError::invalid_params(error.to_string(), None),
        SandboxError::SandboxNotFound { .. } => McpError::invalid_params(error.to_string(), None),
        _ => McpError::internal_error(error.to_string(), None),
    }
}
