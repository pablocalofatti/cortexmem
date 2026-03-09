#!/usr/bin/env bash
# cortexmem session-start hook
# Fires on SessionStart — detects project, starts a memory session,
# and outputs the Memory Protocol instructions + recent context.

set -euo pipefail

PROJECT=$(basename "$PWD")
CORTEXMEM=$(command -v cortexmem 2>/dev/null || echo "")

if [ -z "$CORTEXMEM" ]; then
    echo "cortexmem binary not found. Memory features disabled."
    exit 0
fi

# Output Memory Protocol instructions
cat "$(dirname "$0")/../skills/memory-protocol/SKILL.md"

echo ""
echo "---"
echo "Project: $PROJECT"
echo "Directory: $PWD"
echo ""
echo "Use mem_session_start to initialize your session, then mem_context for recent observations."
