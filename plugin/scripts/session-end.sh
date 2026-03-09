#!/usr/bin/env bash
# cortexmem session-end hook
# Fires on Stop — reminds the agent to save important observations
# before the session closes.

set -euo pipefail

CORTEXMEM=$(command -v cortexmem 2>/dev/null || echo "")

if [ -z "$CORTEXMEM" ]; then
    exit 0
fi

echo "Session ending. Before closing:"
echo "1. Save any important decisions, patterns, or discoveries with mem_save"
echo "2. Call mem_session_summary with a brief recap of this session"
echo "3. Call mem_session_end to close the session"
