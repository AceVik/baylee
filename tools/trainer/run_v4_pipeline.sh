#!/usr/bin/env bash
set -euo pipefail
BAYLEE_TRAIN_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$BAYLEE_TRAIN_REPO"
exec tools/trainer/.venv/bin/python tools/trainer/pipeline4.py "$@"
