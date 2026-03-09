#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: update-homebrew.sh <version>}"
echo "Triggering homebrew-tap update for v${VERSION}..."
gh api repos/pablocalofatti/homebrew-tap/dispatches \
  -f event_type=cortexmem-release \
  -f "client_payload[version]=${VERSION}"
echo "Done. Check https://github.com/pablocalofatti/homebrew-tap/actions"
