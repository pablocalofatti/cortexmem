#!/usr/bin/env bash
# cortexmem compaction-recovery hook
# Re-injects Memory Protocol instructions after context compaction
# so the agent remembers how to use cortexmem tools.

set -euo pipefail

cat "$(dirname "$0")/../skills/memory-protocol/SKILL.md"

echo ""
echo "---"
echo "Context was compacted. Call mem_context to recover recent observations."
echo "Call mem_session_summary before this point to preserve session state."
