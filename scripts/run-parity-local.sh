#!/usr/bin/env bash
# Builds native binaries and tests only disposable local infrastructure.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -z "${JAVA_HOME:-}" && -d /opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home ]]; then
  export JAVA_HOME=/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home
fi
if [[ -n "${JAVA_HOME:-}" ]]; then export JAVA_BIN="$JAVA_HOME/bin/java"; fi
python3 scripts/check-route-parity.py
cargo build --locked --bin ciel
mvn -q -f spring/pom.xml package
dotnet test dotnet -m:1 /nodeReuse:false /p:UseSharedCompilation=false
python3 scripts/parity-integration.py "$@"
