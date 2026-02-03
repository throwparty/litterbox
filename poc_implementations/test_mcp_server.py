#!/usr/bin/env python3
"""
Generic MCP test client for write_file PoC implementations
Sends JSON-RPC messages to the MCP server via stdio

Usage:
    python3 test_mcp_server.py <poc-directory>
    
Example:
    python3 test_mcp_server.py poc-rmcp
"""

import json
import subprocess
import sys
import tempfile
import os
from pathlib import Path

def send_request(proc, request, expect_response=True):
    """Send a JSON-RPC request and optionally read the response"""
    request_json = json.dumps(request) + "\n"
    print(f"→ Sending: {request_json.strip()}", file=sys.stderr)
    proc.stdin.write(request_json)
    proc.stdin.flush()
    
    if not expect_response:
        print("  (notification - no response expected)", file=sys.stderr)
        return None
    
    response_line = proc.stdout.readline()
    if response_line:
        print(f"← Received: {response_line.strip()}", file=sys.stderr)
        return json.loads(response_line)
    return None

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 test_mcp_server.py <poc-directory>", file=sys.stderr)
        print("Example: python3 test_mcp_server.py poc-rmcp", file=sys.stderr)
        sys.exit(1)
    
    poc_dir = sys.argv[1]
    poc_name = os.path.basename(poc_dir.rstrip('/'))
    
    print(f"Testing MCP PoC: {poc_name}", file=sys.stderr)
    print(f"Working directory: {poc_dir}", file=sys.stderr)
    
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
        })
        
        assert init_response.get("result"), "Initialize failed"
        print("✓ Initialize successful", file=sys.stderr)
        
        # Send initialized notification
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }, expect_response=False)
        
        # Test 2: List tools
        print("\n[TEST 2] List available tools", file=sys.stderr)
        tools_response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
        
        tools = tools_response.get("result", {}).get("tools", [])
        write_tool = next((t for t in tools if t["name"] == "write_file"), None)
        assert write_tool, "write_file tool not found"
        print(f"✓ Found write_file tool: {write_tool['description']}", file=sys.stderr)
        print(f"  Input schema: {write_tool.get('inputSchema', {})}", file=sys.stderr)
        
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
                    "name": "write_file",
                    "arguments": {
                        "path": test_file,
                        "content": test_content
                    }
                }
            })
            
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
                "name": "write_file",
                "arguments": {
                    "path": "relative/path.txt",
                    "content": "This should fail"
                }
            }
        })
        
        assert "error" in error_response, "Relative path should have been rejected"
        print(f"✓ Relative path correctly rejected: {error_response['error']}", file=sys.stderr)
        
        print("\n" + "="*50, file=sys.stderr)
        print("All tests passed! ✓", file=sys.stderr)
        print("="*50, file=sys.stderr)
        
    finally:
        proc.terminate()
        proc.wait(timeout=2)

if __name__ == "__main__":
    main()
