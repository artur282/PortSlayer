#!/usr/bin/env bash
# Desinstalador de PortSlayer.
# Delega en install.sh --uninstall para mantener una sola fuente de verdad.

set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$ROOT/scripts/install.sh" --uninstall "$@"
