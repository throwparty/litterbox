#!/usr/bin/env python3
"""
Generic MCP test client for write_file PoC implementations
Supports both traditional SDKs (rmcp, pmcp) and hyper-mcp WASM plugins
Sends JSON-RPC messages to the MCP server via stdio

Usage:
    python3 test_mcp_server.py <poc-directory>
    
Example:
    python3 test_mcp_server.py poc-rmcp
    python3 test_mcp_server.py poc-hyper-mcp
"""

import json
import subprocess
import sys
import tempfile
import os
from pathlib import Path

def detect_poc_type(poc_dir):
    """Detect if this is a traditional SDK or hyper-mcp WASM plugin"""
    cargo_toml_path = os.path.join(poc_dir, "Cargo.toml")
    config_json_path = os.path.join(poc_dir, "config.json")
    
    if not os.path.exists(cargo_toml_path):
        return "unknown"
    
    try:
        with open(cargo_toml_path) as f:
            cargo_content = f.read()
        
        # Check for hyper-mcp indicators (simple string matching)
        if 'crate-type = ["cdylib"]' in cargo_content or "crate-type = ['cdylib']" in cargo_content:
            if os.path.exists(config_json_path):
                return "hyper-mcp"
        
        return "traditional"
    except Exception:
        return "traditional"

def send_request(proc, request, expect_response=True, skip_logs=False):
    """Send a JSON-RPC request and optionally read the response"""
    request_json = json.dumps(request) + "\n"
    print(f"→ Sending: {request_json.strip()}", file=sys.stderr)
    proc.stdin.write(request_json)
    proc.stdin.flush()
    
    if not expect_response:
        print("  (notification - no response expected)", file=sys.stderr)
        return None
    
    # For hyper-mcp, skip log lines
    while True:
        response_line = proc.stdout.readline()
        if not response_line:
            return None
        
        stripped = response_line.strip()
        if skip_logs and not stripped.startswith("{"):
            # Log line from hyper-mcp, skip it
            print(f"  [LOG] {stripped}", file=sys.stderr)
            continue
        
        print(f"← Received: {stripped}", file=sys.stderr)
        return json.loads(stripped)

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 test_mcp_server.py <poc-directory>", file=sys.stderr)
        print("Example: python3 test_mcp_server.py poc-rmcp", file=sys.stderr)
        sys.exit(1)
    
    poc_dir = sys.argv[1]
    poc_name = os.path.basename(poc_dir.rstrip('/'))
    poc_type = detect_poc_type(poc_dir)
    
    print(f"Testing MCP PoC: {poc_name}", file=sys.stderr)
    print(f"Working directory: {poc_dir}", file=sys.stderr)
    print(f"Type: {poc_type}", file=sys.stderr)
    
    # Build and start server based on type
    if poc_type == "hyper-mcp":
        # Build WASM plugin
        print("\nBuilding WASM plugin...", file=sys.stderr)
        try:
            subprocess.run(
                ["direnv", "exec", os.path.abspath(poc_dir), "cargo", "build", "--release", "--target", "wasm32-wasip1"],
                cwd=poc_dir,
                check=True,
                capture_output=True
            )
        except subprocess.CalledProcessError as e:
            print(f"Build failed: {e}", file=sys.stderr)
            print(e.stderr.decode() if e.stderr else "", file=sys.stderr)
            sys.exit(1)
        
        # Start hyper-mcp runtime
        print(f"Starting hyper-mcp runtime...", file=sys.stderr)
        hyper_mcp_bin = os.path.expanduser("~/.cargo/bin/hyper-mcp")
        config_path = os.path.join(poc_dir, "config.json")
        
        proc = subprocess.Popen(
            [hyper_mcp_bin, "--config-file", config_path, "--insecure-skip-signature", "true"],
            cwd=poc_dir,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        skip_logs = True
        tool_name = "write_file_plugin-write_file"  # hyper-mcp prefixes with plugin name
        
        # Give hyper-mcp time to start and load plugins
        import time
        time.sleep(0.5)
        
    else:  # traditional SDK
        # Build the server first
        print("\nBuilding MCP server...", file=sys.stderr)
        try:
            subprocess.run(["cargo", "build"], cwd=poc_dir, check=True, capture_output=True)
        except subprocess.CalledProcessError as e:
            print(f"Build failed: {e}", file=sys.stderr)
            print(e.stderr.decode() if e.stderr else "", file=sys.stderr)
            sys.exit(1)
        
        # Start the MCP server using cargo run
        print(f"Starting server with cargo run...", file=sys.stderr)
        
        proc = subprocess.Popen(
            ["cargo", "run"],
            cwd=poc_dir,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        skip_logs = False
        tool_name = "write_file"
    
    try:
        # Test 1: Initialize
        print("\n[TEST 1] Initialize MCP connection", file=sys.stderr)
        init_response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"roots": {"listChanged": True}},
                "clientInfo": {"name": "python-test-client", "version": "1.0.0"}
            }
        }, skip_logs=skip_logs)
        
        assert init_response.get("result"), "Initialize failed"
        server_info = init_response["result"].get("serverInfo", {})
        print(f"✓ Initialize successful: {server_info.get('name', 'unknown')} v{server_info.get('version', 'unknown')}", file=sys.stderr)
        
        # Send initialized notification
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }, expect_response=False, skip_logs=skip_logs)
        
        # Test 2: List tools
        print("\n[TEST 2] List available tools", file=sys.stderr)
        tools_response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }, skip_logs=skip_logs)
        
        tools = tools_response.get("result", {}).get("tools", [])
        write_tool = next((t for t in tools if tool_name in t["name"]), None)
        assert write_tool, f"{tool_name} tool not found"
        print(f"✓ Found write_file tool: {write_tool['name']}", file=sys.stderr)
        print(f"  Description: {write_tool.get('description', 'N/A')}", file=sys.stderr)
        
        # Test 3: Write file with absolute path
        print("\n[TEST 3] Write file with absolute path", file=sys.stderr)
        with tempfile.TemporaryDirectory() as tmpdir:
            test_file = os.path.join(tmpdir, "nested", "test.txt")
            test_content = "Hello from Python MCP client!"
            
            write_response = send_request(proc, {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": tool_name,
                    "arguments": {
                        "path": test_file,
                        "content": test_content
                    }
                }
            }, skip_logs=skip_logs)
            
            # Check for expected failure on hyper-mcp (WASM sandbox)
            if poc_type == "hyper-mcp":
                if "error" in write_response:
                    print(f"⚠️  Expected failure (WASM sandbox): {write_response['error']['message']}", file=sys.stderr)
                    print(f"   hyper-mcp blocks filesystem access by design", file=sys.stderr)
                else:
                    print(f"❌ Unexpected success on hyper-mcp (should be sandboxed)", file=sys.stderr)
                    sys.exit(1)
            else:
                # Traditional SDK should succeed
                result = write_response.get("result")
                assert result, f"Write failed: {write_response}"
                print(f"✓ Server response: {result}", file=sys.stderr)
                
                # Verify file was created
                assert os.path.exists(test_file), f"File not created at {test_file}"
                with open(test_file) as f:
                    actual_content = f.read()
                assert actual_content == test_content, f"Content mismatch: {actual_content!r} != {test_content!r}"
                print(f"✓ File created successfully with correct content", file=sys.stderr)
        
        # Test 4: Write file with relative path (should fail)
        print("\n[TEST 4] Write file with relative path (should fail)", file=sys.stderr)
        error_response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": {
                    "path": "relative/path.txt",
                    "content": "This should fail"
                }
            }
        }, skip_logs=skip_logs)
        
        # Both types should reject relative paths (either validation or sandbox)
        assert "error" in error_response, "Relative path should have been rejected"
        print(f"✓ Relative path correctly rejected: {error_response['error']['message']}", file=sys.stderr)
        
        print("\n" + "="*50, file=sys.stderr)
        print(f"All tests passed for {poc_name}! ✓", file=sys.stderr)
        if poc_type == "hyper-mcp":
            print("Note: hyper-mcp WASM sandbox prevents filesystem operations (by design)", file=sys.stderr)
        print("="*50, file=sys.stderr)
        
    finally:
        proc.terminate()
        proc.wait(timeout=2)

if __name__ == "__main__":
    main()
