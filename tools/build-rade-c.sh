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

# RADE's CMake file links libm unconditionally for Unix builds. MSVC does not
# provide m.lib, so remove that Unix-only dependency from the pinned checkout
# before configuring the Windows build. Keep the source checkout pinned; this
# is a deterministic platform adaptation, not an upstream revision change.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        for tool in sh autoreconf make patch; do
            if ! command -v "$tool" >/dev/null 2>&1; then
                echo "Windows RADE builds require '$tool' from MSYS2 (install the MSYS2 base and autotools packages)." >&2
                exit 1
            fi
        done

        # RADE's Opus ExternalProject contains Unix autotools commands.  The
        # Visual Studio generator otherwise sends ./configure and make to
        # cmd.exe, where neither command can run.  Route those commands
        # through the MSYS2 shell while retaining MSVC for the native RADE
        # targets.
        opus_cmake="$RADE_C_DIR/cmake/BuildOpus.cmake"
        sed -i 's#^set(CONFIGURE_COMMAND ./autogen.sh#set(CONFIGURE_COMMAND sh -c \"./autogen.sh#' "$opus_cmake"
        sed -i '/^set(CONFIGURE_COMMAND sh -c/ s#)$#\")#' "$opus_cmake"
        sed -i 's#BUILD_COMMAND.*MAKE.*#BUILD_COMMAND sh -c \"make\"#' "$opus_cmake"
        sed -i 's/ m)/)/g' "$RADE_C_DIR/src/CMakeLists.txt"
        ;;
esac

cmake -S "$RADE_C_DIR" -B "$RADE_C_BUILD_DIR" -DCMAKE_BUILD_TYPE=Release
cmake --build "$RADE_C_BUILD_DIR" --parallel

cat <<EOF
RADE_C_DIR=$RADE_C_DIR
RADE_C_BUILD_DIR=$RADE_C_BUILD_DIR
RADE_C_LIB_DIR=$RADE_C_BUILD_DIR/src

Run the consumer with:
  RADE_C_DIR=$RADE_C_DIR RADE_C_BUILD_DIR=$RADE_C_BUILD_DIR RADE_C_LIB_DIR=$RADE_C_BUILD_DIR/src cargo test --features rade-speech
EOF
