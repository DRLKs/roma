#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
python_bin="${ROMA_BENCHMARK_PYTHON:-${repo_root}/.venv/bin/python}"

if [[ $# -lt 1 ]]; then
    echo "usage: ${0##*/} <benchmark_name> [args...]" >&2
    exit 2
fi

benchmark_name="$1"
shift
orchestrator_path="${script_dir}/${benchmark_name}/orchestrate.py"

if [[ ! -x "${python_bin}" ]]; then
    echo "python interpreter not found: ${python_bin}" >&2
    exit 1
fi

if [[ ! -f "${orchestrator_path}" ]]; then
    echo "orchestrator not found: ${orchestrator_path}" >&2
    exit 1
fi

exec "${python_bin}" "${orchestrator_path}" "$@"