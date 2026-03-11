#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/scripts/git-hooks"

chmod +x "$HOOKS_DIR/pre-commit"
git config core.hooksPath "$HOOKS_DIR"
echo "Git hooks configured. Hooks path: $HOOKS_DIR"
