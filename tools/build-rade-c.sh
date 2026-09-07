#!/usr/bin/env bash
set -euo pipefail

# Keep the native source in one reproducible third-party-owned cache.  The
# Rust adapter links this checkout; QSONaut does not carry a second RADE copy.
RADE_C_REPOSITORY="${RADE_C_REPOSITORY:-https://github.com/freedv/rade_c.git}"
RADE_C_COMMIT="0d5c5f7c27e650e3ca8d9e0f7d4781f2e73ee2b0"
RADE_CACHE_ROOT="${RADE_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/qsonaut-third-party/rade_c}"
RADE_C_DIR="${RADE_C_DIR:-$RADE_CACHE_ROOT/$RADE_C_COMMIT/source}"
RADE_C_BUILD_DIR="${RADE_C_BUILD_DIR:-$RADE_CACHE_ROOT/$RADE_C_COMMIT/build}"

mkdir -p "$(dirname "$RADE_C_DIR")" "$(dirname "$RADE_C_BUILD_DIR")"

if [[ ! -d "$RADE_C_DIR/.git" ]]; then
    if [[ -e "$RADE_C_DIR" ]]; then
        echo "RADE_C_DIR exists but is not a Git checkout: $RADE_C_DIR" >&2
        exit 1
    fi
    git clone --filter=blob:none "$RADE_C_REPOSITORY" "$RADE_C_DIR"
fi

if ! git -C "$RADE_C_DIR" cat-file -e "$RADE_C_COMMIT^{commit}" 2>/dev/null; then
    git -C "$RADE_C_DIR" fetch --quiet origin "$RADE_C_COMMIT"
fi
git -C "$RADE_C_DIR" checkout --quiet --detach "$RADE_C_COMMIT"

# RADE's CMake file links libm unconditionally for Unix builds. A Windows
# cross-build has no libm, so remove that Unix-only dependency from the pinned
# checkout before configuring.
if [[ "${RADE_CROSS_TARGET:-}" == windows ]]; then
    sed -i 's/ m)/)/g' "$RADE_C_DIR/src/CMakeLists.txt"
fi

cmake_args=(-DCMAKE_BUILD_TYPE=Release)
if [[ "${RADE_CROSS_TARGET:-}" == windows ]]; then
    cmake_args+=(
        -DCMAKE_SYSTEM_NAME=Windows
        "-DCMAKE_SYSTEM_PROCESSOR=${RADE_CMAKE_SYSTEM_PROCESSOR:?RADE_CMAKE_SYSTEM_PROCESSOR is required for Windows cross-builds}"
        "-DCMAKE_C_COMPILER_TARGET=${RADE_CMAKE_COMPILER_TARGET:?RADE_CMAKE_COMPILER_TARGET is required for Windows cross-builds}"
    )
fi

cmake -S "$RADE_C_DIR" -B "$RADE_C_BUILD_DIR" "${cmake_args[@]}"
# The default target also builds upstream test programs. Some tests are not
# wired to every pinned source configuration and are not needed by the adapter.
cmake --build "$RADE_C_BUILD_DIR" --target rade --parallel

cat <<EOF
RADE_C_DIR=$RADE_C_DIR
RADE_C_BUILD_DIR=$RADE_C_BUILD_DIR
RADE_C_LIB_DIR=$RADE_C_BUILD_DIR/src

Run the consumer with:
  RADE_C_DIR=$RADE_C_DIR RADE_C_BUILD_DIR=$RADE_C_BUILD_DIR RADE_C_LIB_DIR=$RADE_C_BUILD_DIR/src cargo test --features rade-speech
EOF
