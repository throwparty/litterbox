# Tasks: MCP Server SDK Evaluation

This task list tracks the evaluation of various Rust MCP SDKs to determine the best fit for the Litterbox project.

## 1. Preparation
- [x] Refine specification (`spec.md`)
- [x] Create technical plan and selection workflow (`plan.md`)
- [x] Identify candidate SDKs (`rmcp`, `hyper-mcp`, `pmcp`, `ultrafast-mcp`, `mcp-protocol-sdk`)

## 2. Proof-of-Concept Implementation

### 2.1 PoC with `rmcp`
- [x] **Task 2.1.1:** Create a new Rust project for `rmcp` PoC at `poc_implementations/poc-rmcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-rmcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-rmcp/`.
    - **Status:** ✅ Completed
- [x] **Task 2.1.2:** Add `rmcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `rmcp` and other necessary crates (e.g., `tokio`, `serde`) are added to `poc_implementations/poc-rmcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Dependencies added:**
      - `rmcp = { version = "0.14.0", features = ["server", "transport-io", "macros", "schemars"] }`
      - `tokio = { version = "1.49.0", features = ["full"] }`
      - `serde = { version = "1.0.228", features = ["derive"] }`
      - `schemars = { version = "1.2.1", features = ["derive"] }`
      - `anyhow = "1.0.100"`
      - `serde_json = "1.0.149"`
    - **Critical Learning:** The `schemars` feature flag is REQUIRED on `rmcp` for the `#[tool]` macro to work correctly.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-rmcp/`.
    - **Status:** ✅ Completed
- [x] **Task 2.1.3:** Implement `write_file` tool using `rmcp` in `poc_implementations/poc-rmcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-rmcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Implementation Details:**
      - Server struct: `WriteFileServer` with `ToolRouter<Self>`
      - Tool method: `write_file(Parameters<WriteFileArgs>) -> Result<CallToolResult, McpError>`
      - Input validation: Checks for absolute paths
      - File operations: Creates parent directories, writes content
      - Macros used: `#[tool_router]`, `#[tool]`, `#[tool_handler]`
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
    - **Status:** ✅ Completed
- [x] **Task 2.1.4:** Run test harness against `rmcp` PoC.
    - **Acceptance Criteria:** The generic test harness successfully validates the `rmcp` PoC implementation.
    - **Test Command (Python):** 
      ```bash
      cd poc_implementations && python3 test_mcp_server.py poc-rmcp
      ```
    - **Test Command (Bash):**
      ```bash
      cd poc_implementations && ./test_mcp_server.sh poc-rmcp
      ```
    - **Test Requirements:** 
      - All 4 tests pass (initialize, list tools, write with absolute path, reject relative path)
      - File is created at the specified location with correct content
      - Relative path is properly rejected with an error
    - **Test Results:**
      - ✅ Initialize: Server responds with correct capabilities (rmcp v0.14.0)
      - ✅ List tools: `write_file` discovered with proper JSON Schema
      - ✅ Write absolute path: File created with 29 bytes, parent dirs created
      - ✅ Reject relative path: Error -32602 "path must be absolute"
    - **Status:** ✅ Completed

### 2.2 PoC with `hyper-mcp` [✅ COMPLETE - REJECTED DUE TO WASM SANDBOX]
- [x] **Task 2.2.1:** Create a new Rust project for `hyper-mcp` PoC at `poc_implementations/poc-hyper-mcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-hyper-mcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-hyper-mcp/`.
    - **Status:** ✅ Completed
- [x] **Task 2.2.2:** Add `hyper-mcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `hyper-mcp` and other necessary crates are added to `poc_implementations/poc-hyper-mcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Dependencies added:**
      - `extism-pdk = "1.4.1"` (WASM plugin development kit)
      - `serde = { version = "1.0.228", features = ["derive"] }`
      - `serde_json = "1.0.149"`
      - `anyhow = "1.0.100"`
      - `base64 = "0.22.1"`
      - `base64-serde = "0.8.0"`
      - `chrono = { version = "0.4.43", features = ["serde"] }`
    - **Build target:** `wasm32-wasip1` (WebAssembly plugin)
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-hyper-mcp/`.
    - **Status:** ✅ Completed
- [x] **Task 2.2.3:** Implement `write_file` tool using `hyper-mcp` in `poc_implementations/poc-hyper-mcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-hyper-mcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Implementation Details:**
      - WASM plugin architecture (not standalone binary)
      - Uses Extism PDK types instead of standard MCP types
      - Plugin exports: `mcp_list_tools`, `mcp_call_tool`
      - Compiled to `plugin.wasm` (391KB)
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
    - **Status:** ✅ Code complete and builds successfully
- [x] **Task 2.2.4:** Run test harness against `hyper-mcp` PoC.
    - **Acceptance Criteria:** The generic test harness successfully validates the `hyper-mcp` PoC implementation.
    - **Test Command:** Custom test harness (standard test doesn't work for WASM plugins)
    - **Test Requirements:** All 4 tests pass (initialize, list tools, write with absolute path, reject relative path).
    - **Status:** ⚠️ PARTIAL - 2 of 4 tests pass, rejected due to WASM sandbox
    - **Test Results:**
      - ✅ Initialize: Server responds (hyper-mcp v0.2.3)
      - ✅ List tools: `write_file_plugin-write_file` discovered
      - ❌ Write absolute path: **BLOCKED by WASM sandbox** (no filesystem access by design)
      - ⚠️ Reject relative path: Cannot test (sandbox blocks before validation)
    - **Rejection Reason:** WASM sandbox prevents filesystem operations - architectural mismatch for our use case

### 2.3 PoC with `pmcp` [⚠️ TESTS PASS - BUT REQUIRES UNRELEASED GIT DEPENDENCY]
- [x] **Task 2.3.1:** Create a new Rust project for `pmcp` PoC at `poc_implementations/poc-pmcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-pmcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-pmcp/`.
    - **Status:** ✅ Completed
- [x] **Task 2.3.2:** Add `pmcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `pmcp` and other necessary crates are added to `poc_implementations/poc-pmcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Dependencies added:**
      - ⚠️ **`pmcp = { git = "https://github.com/paiml/rust-mcp-sdk", branch = "main" }`** (git dependency required - crates.io v1.9.4 has broken stdio)
      - `tokio = { version = "1.49", features = ["full"] }`
      - `serde = { version = "1.0.228", features = ["derive"] }`
      - `schemars = { version = "1.2.1", features = ["derive"] }`
      - `anyhow = "1.0.100"`
      - `serde_json = "1.0.149"`
      - `async-trait = "0.1.89"`
      - `tracing-subscriber = { version = "0.3.22", features = ["env-filter"] }`
    - **Critical Issue Discovered:** pmcp v1.9.4 on crates.io has non-functional stdio transport (used HTTP Content-Length framing instead of newline-delimited JSON-RPC). Fixed in PR #157 (merged Jan 18, 2026) but not yet released.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-pmcp/`.
    - **Status:** ✅ Completed (using git main branch, commit e1bcebaf)
- [x] **Task 2.3.3:** Implement `write_file` tool using `pmcp` in `poc_implementations/poc-pmcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-pmcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Implementation Details:**
      - Server builder pattern: `Server::builder().name().version().capabilities().tool().build()`
      - Tool handler: Implements `ToolHandler` trait with async `handle()` method
      - Entry point: `#[tokio::main]` async main calling `server.run_stdio().await`
      - Pattern matched official pmcp example: `/tmp/rust-mcp-sdk/examples/02_server_basic.rs`
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
    - **Status:** ✅ Code complete (compiles with `cargo check`)
- [x] **Task 2.3.4:** Run test harness against `pmcp` PoC.
    - **Acceptance Criteria:** The generic test harness successfully validates the `pmcp` PoC implementation.
    - **Test Command:** `nix develop ./nix --command bash -c 'cd poc_implementations && python3 test_mcp_server.py poc-pmcp'`
    - **Test Requirements:** All 4 tests pass (initialize, list tools, write with absolute path, reject relative path).
    - **Status:** ✅ ALL TESTS PASS (when using pmcp from git main branch)
    - **Test Results:**
      - ✅ Initialize: Server responds with correct capabilities (poc-pmcp-write-file v1.0.0)
      - ✅ List tools: `write_file` discovered (but no schema - pmcp limitation)
      - ✅ Write absolute path: File created with 29 bytes, parent dirs created
      - ✅ Reject relative path: Error -32603 "Validation error: path must be absolute"
    - **⚠️ CRITICAL PRODUCTION CONCERN:**
      - pmcp v1.9.4 (latest stable on crates.io) has **broken stdio support**
      - Must use unreleased code from git main branch (commit e1bcebaf)
      - PR #157 fixed stdio on Jan 18, 2026 but no new release published (17 days later as of Feb 4, 2026)
      - **Cannot recommend for production** due to dependency on unreleased code
      - Using git dependencies bypasses semantic versioning and creates maintenance burden

### 2.4 PoC with `ultrafast-mcp`
- [ ] **Task 2.4.1:** Create a new Rust project for `ultrafast-mcp` PoC at `poc_implementations/poc-ultrafast-mcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-ultrafast-mcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-ultrafast-mcp/`.
- [ ] **Task 2.4.2:** Add `ultrafast-mcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `ultrafast-mcp` and other necessary crates are added to `poc_implementations/poc-ultrafast-mcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-ultrafast-mcp/`.
- [ ] **Task 2.4.3:** Implement `write_file` tool using `ultrafast-mcp` in `poc_implementations/poc-ultrafast-mcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-ultrafast-mcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task 2.4.4:** Run test harness against `ultrafast-mcp` PoC.
    - **Acceptance Criteria:** The generic test harness successfully validates the `ultrafast-mcp` PoC implementation.
    - **Test Command:** `cd poc_implementations && python3 test_mcp_server.py poc-ultrafast-mcp`
    - **Test Requirements:** All 4 tests pass (initialize, list tools, write with absolute path, reject relative path).

### 2.5 PoC with `mcp-protocol-sdk`
- [ ] **Task 2.5.1:** Create a new Rust project for `mcp-protocol-sdk` PoC at `poc_implementations/poc-mcp-protocol-sdk/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-mcp-protocol-sdk/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-mcp-protocol-sdk/`.
- [ ] **Task 2.5.2:** Add `mcp-protocol-sdk` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `mcp-protocol-sdk` and other necessary crates are added to `poc_implementations/poc-mcp-protocol-sdk/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-mcp-protocol-sdk/`.
- [ ] **Task 2.5.3:** Implement `write_file` tool using `mcp-protocol-sdk` in `poc_implementations/poc-mcp-protocol-sdk/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-mcp-protocol-sdk/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task 2.5.4:** Run test harness against `mcp-protocol-sdk` PoC.
    - **Acceptance Criteria:** The generic test harness successfully validates the `mcp-protocol-sdk` PoC implementation.
    - **Test Command:** `cd poc_implementations && python3 test_mcp_server.py poc-mcp-protocol-sdk`
    - **Test Requirements:** All 4 tests pass (initialize, list tools, write with absolute path, reject relative path).

## 3. Evaluation and Decision
- [ ] **Task:** Compare PoC implementations.
    - **Acceptance Criteria:** A summary document (e.g., markdown table) comparing ergonomics, boilerplate code, and build times for each PoC.
    - **Test Requirements:** N/A
- [ ] **Task:** Verify protocol compliance with MCP Inspector.
    - **Acceptance Criteria:** Each PoC server is tested with an MCP Inspector tool, and any compliance issues are documented.
    - **Test Requirements:** N/A
- [ ] **Task:** Finalize the decision in `plan.md`.
    - **Acceptance Criteria:** The `plan.md` is updated with the final primary and fallback SDK choices, along with detailed justification.
    - **Test Requirements:** N/A
- [ ] **Task:** Archive/Delete unsuccessful PoC code.
    - **Acceptance Criteria:** PoC directories for non-selected SDKs are removed or moved to an archive.
    - **Test Requirements:** N/A
