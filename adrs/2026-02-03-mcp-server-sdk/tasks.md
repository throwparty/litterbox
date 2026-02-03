# Tasks: MCP Server SDK Evaluation

This task list tracks the evaluation of various Rust MCP SDKs to determine the best fit for the Litterbox project.

## 1. Preparation
- [x] Refine specification (`spec.md`)
- [x] Create technical plan and selection workflow (`plan.md`)
- [x] Identify candidate SDKs (`rmcp`, `hyper-mcp`, `pmcp`, `ultrafast-mcp`, `mcp-protocol-sdk`)

## 2. Proof-of-Concept Implementation

### 2.1 PoC with `rmcp`
- [ ] **Task:** Create a new Rust project for `rmcp` PoC at `poc_implementations/poc-rmcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-rmcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-rmcp/`.
- [ ] **Task:** Add `rmcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `rmcp` and other necessary crates (e.g., `tokio`, `serde`) are added to `poc_implementations/poc-rmcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-rmcp/`.
- [ ] **Task:** Implement `write_file` tool using `rmcp` in `poc_implementations/poc-rmcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-rmcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task:** Write basic unit/integration test for `rmcp` PoC in `poc_implementations/poc-rmcp/`.
    - **Acceptance Criteria:** A test suite exists that verifies the `write_file` tool's functionality.
    - **Test Requirements:** `cargo test` runs successfully and passes in `poc_implementations/poc-rmcp/`.

### 2.2 PoC with `hyper-mcp`
- [ ] **Task:** Create a new Rust project for `hyper-mcp` PoC at `poc_implementations/poc-hyper-mcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-hyper-mcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-hyper-mcp/`.
- [ ] **Task:** Add `hyper-mcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `hyper-mcp` and other necessary crates are added to `poc_implementations/poc-hyper-mcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-hyper-mcp/`.
- [ ] **Task:** Implement `write_file` tool using `hyper-mcp` in `poc_implementations/poc-hyper-mcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-hyper-mcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task:** Write basic unit/integration test for `hyper-mcp` PoC in `poc_implementations/poc-hyper-mcp/`.
    - **Acceptance Criteria:** A test suite exists that verifies the `write_file` tool's functionality.
    - **Test Requirements:** `cargo test` runs successfully and passes in `poc_implementations/poc-hyper-mcp/`.

### 2.3 PoC with `pmcp`
- [ ] **Task:** Create a new Rust project for `pmcp` PoC at `poc_implementations/poc-pmcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-pmcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-pmcp/`.
- [ ] **Task:** Add `pmcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `pmcp` and other necessary crates are added to `poc_implementations/poc-pmcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-pmcp/`.
- [ ] **Task:** Implement `write_file` tool using `pmcp` in `poc_implementations/poc-pmcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-pmcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task:** Write basic unit/integration test for `pmcp` PoC in `poc_implementations/poc-pmcp/`.
    - **Acceptance Criteria:** A test suite exists that verifies the `write_file` tool's functionality.
    - **Test Requirements:** `cargo test` runs successfully and passes in `poc_implementations/poc-pmcp/`.

### 2.4 PoC with `ultrafast-mcp`
- [ ] **Task:** Create a new Rust project for `ultrafast-mcp` PoC at `poc_implementations/poc-ultrafast-mcp/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-ultrafast-mcp/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-ultrafast-mcp/`.
- [ ] **Task:** Add `ultrafast-mcp` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `ultrafast-mcp` and other necessary crates are added to `poc_implementations/poc-ultrafast-mcp/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-ultrafast-mcp/`.
- [ ] **Task:** Implement `write_file` tool using `ultrafast-mcp` in `poc_implementations/poc-ultrafast-mcp/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-ultrafast-mcp/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task:** Write basic unit/integration test for `ultrafast-mcp` PoC in `poc_implementations/poc-ultrafast-mcp/`.
    - **Acceptance Criteria:** A test suite exists that verifies the `write_file` tool's functionality.
    - **Test Requirements:** `cargo test` runs successfully and passes in `poc_implementations/poc-ultrafast-mcp/`.

### 2.5 PoC with `mcp-protocol-sdk`
- [ ] **Task:** Create a new Rust project for `mcp-protocol-sdk` PoC at `poc_implementations/poc-mcp-protocol-sdk/`.
    - **Acceptance Criteria:** A new Rust project directory `poc_implementations/poc-mcp-protocol-sdk/` exists with a valid `Cargo.toml`.
    - **Test Requirements:** `cargo check` runs successfully in `poc_implementations/poc-mcp-protocol-sdk/`.
- [ ] **Task:** Add `mcp-protocol-sdk` and required dependencies using `cargo add`.
    - **Acceptance Criteria:** `mcp-protocol-sdk` and other necessary crates are added to `poc_implementations/poc-mcp-protocol-sdk/Cargo.toml` using their latest versions via `cargo add`.
    - **Test Requirements:** `cargo build` runs successfully in `poc_implementations/poc-mcp-protocol-sdk/`.
- [ ] **Task:** Implement `write_file` tool using `mcp-protocol-sdk` in `poc_implementations/poc-mcp-protocol-sdk/`.
    - **Acceptance Criteria:** The `poc_implementations/poc-mcp-protocol-sdk/` project contains an MCP server implementation that exposes a `write_file` tool as defined in `plan.md` (Section 6.1).
    - **Test Requirements:** The server can be started and responds to a `write_file` MCP call, successfully writing content to a specified path.
- [ ] **Task:** Write basic unit/integration test for `mcp-protocol-sdk` PoC in `poc_implementations/poc-mcp-protocol-sdk/`.
    - **Acceptance Criteria:** A test suite exists that verifies the `write_file` tool's functionality.
    - **Test Requirements:** `cargo test` runs successfully and passes in `poc_implementations/poc-mcp-protocol-sdk`.

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
