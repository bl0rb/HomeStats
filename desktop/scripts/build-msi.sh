#!/bin/bash
# Builds dist/HomeStats-<version>.msi (installs to Program Files). Runs in Git Bash on Windows.
# Same steps as KeyZapper's build-msi.sh.
#   VERSION  app/package version (default: version in src-tauri/tauri.conf.json), e.g. 1.2.0 or 1.2.0-beta.3
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${VERSION:-$(node -p "require('./src-tauri/tauri.conf.json').version")}"

mkdir -p target dist
CORE="${VERSION%%-*}"
if [ "$CORE" = "$VERSION" ]; then
  printf '{"version":"%s"}' "$VERSION" > target/version.conf.json
else
  # Windows Installer versions are numeric only: 1.2.0-beta.3 becomes 1.2.0.3.
  BUILD="${VERSION##*.}"
  case "$BUILD" in ''|*[!0-9]*) BUILD=0 ;; esac
  printf '{"version":"%s","bundle":{"windows":{"wix":{"version":"%s.%s"}}}}' "$VERSION" "$CORE" "$BUILD" > target/version.conf.json
fi
rm -rf target/release/bundle/msi
npx tauri build --bundles msi --config target/version.conf.json
MSI="dist/HomeStats-$VERSION.msi"
cp target/release/bundle/msi/*.msi "$MSI"
echo "Built $MSI"
