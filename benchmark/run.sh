#!/usr/bin/env bash

set -euo pipefail

BENCH_DIR="$(cd "$(dirname "$0")" && pwd)"

cd "$BENCH_DIR"
trap 'rm -rf .build .tmp' EXIT
go run ./run.go "$@"
