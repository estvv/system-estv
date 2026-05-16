#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_HASH=$(git -C "$SCRIPT_DIR" rev-parse --short HEAD)

echo "{\"gitHash\": \"$GIT_HASH\"}" > "$SCRIPT_DIR/system-estv/static/version.json"

echo "Updated version.json with git hash: $GIT_HASH"