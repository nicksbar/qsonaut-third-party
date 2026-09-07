#!/usr/bin/env bash
set -euo pipefail

# Package only the native outputs consumed by qsonaut-third-party's build.rs.
# The source checkout and build system remain in the producer job; downstream
# QSONaut jobs receive this small, target-specific component artifact.
RADE_C_BUILD_DIR="${RADE_C_BUILD_DIR:?RADE_C_BUILD_DIR must point to the completed RADE build}"
RADE_ARTIFACT_DIR="${RADE_ARTIFACT_DIR:-$PWD/rade-artifact}"
OPUS_ROOT="$RADE_C_BUILD_DIR/build_opus-prefix/src/build_opus"

rm -rf "$RADE_ARTIFACT_DIR"
mkdir -p "$RADE_ARTIFACT_DIR/build/src" "$RADE_ARTIFACT_DIR/build/build_opus-prefix/src/build_opus"

cp -a "$RADE_C_BUILD_DIR/src/." "$RADE_ARTIFACT_DIR/build/src/"
for directory in dnn include celt; do
    cp -a "$OPUS_ROOT/$directory" "$RADE_ARTIFACT_DIR/build/build_opus-prefix/src/build_opus/"
done
mkdir -p "$RADE_ARTIFACT_DIR/build/build_opus-prefix/src/build_opus/.libs"
cp -a "$OPUS_ROOT/.libs/libopus"* \
    "$RADE_ARTIFACT_DIR/build/build_opus-prefix/src/build_opus/.libs/"

cat > "$RADE_ARTIFACT_DIR/RADE_ARTIFACT.txt" <<EOF
RADE_C_BUILD_DIR=build
RADE_C_LIB_DIR=build/src
RADE_C_COMMIT=0d5c5f7c27e650e3ca8d9e0f7d4781f2e73ee2b0
EOF

echo "RADE_ARTIFACT_DIR=$RADE_ARTIFACT_DIR"
