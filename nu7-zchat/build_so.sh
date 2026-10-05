#!/usr/bin/env bash
# build_so.sh <abi> — release build of libzcashwalletsdk.so for one Android ABI from the NU7 SDK checkout.
#
# Mirrors the SDK's own Gradle `cargo {}` block (backend-lib/build.gradle.kts): profile release, no extra features,
# API level ANDROID_MIN_SDK_VERSION (27), NDK 27.0.12077973 clang as linker and C compiler, and the 16 KB page-size
# link argument (`-Wl,-z,max-page-size=16384`, which the Gradle plugin passes as RUST_ANDROID_GRADLE_CC_LINK_ARG).
# Adds, for reproducible output: `--locked`, path remapping for Rust and C sources, and SOURCE_DATE_EPOCH = the
# time of the last commit touching backend-lib. Needs rustup with toolchain 1.92.0 and the four Android targets, and NDK 27.0.12077973 at
# $ANDROID_NDK_HOME (or $ANDROID_HOME/ndk/27.0.12077973).
#
# Output: <out>/<abi>/libzcashwalletsdk.so plus build-info.txt (default <out> = nu7-zchat/out).
set -euo pipefail
ABI=${1:?usage: build_so.sh <arm64-v8a|armeabi-v7a|x86|x86_64> [out-dir]}
SDK=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
NDK=${ANDROID_NDK_HOME:-${ANDROID_HOME:?set ANDROID_NDK_HOME or ANDROID_HOME}/ndk/27.0.12077973}
TC=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin
API=$(sed -n 's/^ANDROID_MIN_SDK_VERSION=//p' "$SDK/gradle.properties")
OUT=${2:-$SDK/nu7-zchat/out}/$ABI
TOOLCHAIN=1.92.0

case "$ABI" in
  arm64-v8a)   TRIPLE=aarch64-linux-android;   CLANG=aarch64-linux-android ;;
  armeabi-v7a) TRIPLE=armv7-linux-androideabi; CLANG=armv7a-linux-androideabi ;;
  x86)         TRIPLE=i686-linux-android;      CLANG=i686-linux-android ;;
  x86_64)      TRIPLE=x86_64-linux-android;    CLANG=x86_64-linux-android ;;
  *) echo "unknown ABI $ABI" >&2; exit 2 ;;
esac
T_ENV=${TRIPLE//-/_}                                  # cc-rs style: CC_aarch64_linux_android
T_CARGO=$(echo "$TRIPLE" | tr 'a-z-' 'A-Z_')          # cargo style: CARGO_TARGET_AARCH64_LINUX_ANDROID_*
CC_BIN=$TC/${CLANG}${API}-clang
[ -x "$CC_BIN" ] || { echo "missing $CC_BIN" >&2; exit 2; }

REMAP="--remap-path-prefix=$HOME/.cargo/registry/src=/cargo/registry/src --remap-path-prefix=$HOME/.cargo/git/checkouts=/cargo/git/checkouts --remap-path-prefix=$HOME/.rustup=/rustup --remap-path-prefix=$SDK=/sdk"
CMAP="-ffile-prefix-map=$HOME/.cargo=/cargo -ffile-prefix-map=$SDK=/sdk -ffile-prefix-map=$NDK=/ndk"

export ANDROID_NDK_HOME=$NDK
export "CC_$T_ENV=$CC_BIN"
export "CXX_$T_ENV=${CC_BIN}++"
export "AR_$T_ENV=$TC/llvm-ar"
export "CFLAGS_$T_ENV=$CMAP"
export "CXXFLAGS_$T_ENV=$CMAP"
export "CARGO_TARGET_${T_CARGO}_LINKER=$CC_BIN"
export "CARGO_TARGET_${T_CARGO}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384 $REMAP"
export SOURCE_DATE_EPOCH=$(git -C "$SDK" log -1 --format=%ct -- backend-lib)   # tooling commits do not move it
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}

cd "$SDK/backend-lib"
cargo +$TOOLCHAIN build --locked --release --lib --target "$TRIPLE"

mkdir -p "$OUT"
SO=target/$TRIPLE/release/libzcashwalletsdk.so
cp "$SO" "$OUT/libzcashwalletsdk.so"
{
  echo "abi=$ABI triple=$TRIPLE api=$API"
  echo "sdk_head=$(git -C "$SDK" rev-parse HEAD) dirty=$([ -n "$(git -C "$SDK" status --porcelain)" ] && echo yes || echo no)"
  echo "cargo_lock_sha256=$(sha256sum Cargo.lock | cut -d' ' -f1)"
  echo "rustc=$(rustc +$TOOLCHAIN -V)"
  echo "cargo=$(cargo +$TOOLCHAIN -V)"
  echo "ndk=$(sed -n 's/^Pkg.Revision = //p' "$NDK/source.properties") clang=$("$CC_BIN" --version | head -1)"
  echo "source_date_epoch=$SOURCE_DATE_EPOCH"
  echo "built=$(date -Iseconds)"
  echo "so_sha256=$(sha256sum "$OUT/libzcashwalletsdk.so" | cut -d' ' -f1) bytes=$(stat -c %s "$OUT/libzcashwalletsdk.so")"
} > "$OUT/build-info.txt"
cat "$OUT/build-info.txt"
