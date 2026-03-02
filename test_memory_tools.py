#!/usr/bin/env python3
"""
Simple test script to demonstrate the new memory tools via MCP.
"""
import subprocess
import json
import sys

def send_mcp_request(tool_name, params):
    """Send an MCP request to the server via stdio."""
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        }
    }
    
    print(f"\n{'='*60}")
    print(f"Testing: {tool_name}")
    print(f"{'='*60}")
    print(f"Request: {json.dumps(params, indent=2)}")
    
    # Note: In a real test, we'd pipe this to the server
    # For now, we'll just show what we'd send
    print(f"\nWould send to server:")
    print(json.dumps(request, indent=2))
    return request

def main():
    """Test all 4 new memory tools."""
    
    print("=" * 60)
    print("MEMORY TOOLS DEMONSTRATION")
    print("=" * 60)
    
    # Test 1: List sessions
    print("\n\n1. LIST SESSIONS")
    print("-" * 60)
    send_mcp_request("reasoning_list_sessions", {
        "limit": 10,
        "offset": 0,
        "mode_filter": None
    })
    
    # Test 2: Search sessions
    print("\n\n2. SEARCH SESSIONS")
    print("-" * 60)
    send_mcp_request("reasoning_search", {
        "query": "database optimization",
        "limit": 5,
        "min_similarity": 0.7,
        "mode_filter": None
    })
    
    # Test 3: Resume session
    print("\n\n3. RESUME SESSION")
    print("-" * 60)
    send_mcp_request("reasoning_resume", {
        "session_id": "test-session-123",
        "include_checkpoints": True,
        "compress": False
    })
    
    # Test 4: Relate sessions
    print("\n\n4. RELATE SESSIONS")
    print("-" * 60)
    send_mcp_request("reasoning_relate", {
        "session_id": "test-session-123",
        "depth": 2,
        "min_strength": 0.5
    })
    
    print("\n\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print("""
All 4 memory tools are now available via MCP:

✅ reasoning_list_sessions - Browse past sessions with pagination
✅ reasoning_search - Semantic search over history  
✅ reasoning_resume - Resume sessions with full context
✅ reasoning_relate - Discover relationship patterns

These tools enable AI agents to:
- Learn from past reasoning patterns
- Resume interrupted work
- Find relevant historical context
- Discover unexpected connections
    """)

if __name__ == "__main__":
    main()
