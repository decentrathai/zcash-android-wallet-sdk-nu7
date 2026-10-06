#!/usr/bin/env bash
# build_aar.sh <version> <maven-repo-dir> <so-dir> [gradle task ...] — build the four SDK artifacts from source and
# publish them into a local Maven repository directory (never to a remote; the artifacts are not signed).
#
# Without tasks, one Gradle run publishes lightwallet-client, zcash-android-backend, zcash-android-sdk and
# zcash-android-sdk-incubator (.aar, .pom, .module and the sources jar of each) and runs the unit tests of
# backend-lib and sdk-lib, the two modules this branch changes. With tasks, only those run (for example one
# module's publishToMavenLocal per call on a machine with little memory). SHA256SUMS-<version> and
# BUILD-INFO-<version>.txt are written once all four artifacts are in the repository.
#
# The Kotlin code is compiled by the SDK's own Gradle build. The native libraries are NOT rebuilt by Gradle: the
# reproducible build_so.sh outputs in <so-dir>/<abi>/ are placed where the Rust Gradle plugin would put them
# (backend-lib/build/rustJniLibs/android/<abi>/) and its cargo tasks are excluded, so the Android Gradle Plugin
# packages and strips exactly those files.
#
# What this script sets, so that the files do not depend on the machine:
#   * Gradle JVM (GRADLE_JVMARGS): -Xmx3g -XX:MaxMetaspaceSize=1g, the values of this repository's
#     gradle.properties, on the command line, because a user-level ~/.gradle/gradle.properties wins over the project
#     file. On the build machine that file caps Metaspace at 512 MiB for another project, and this build, which runs
#     the Kotlin compiler inside the Gradle JVM so that no Kotlin daemon stays behind, stopped there with
#     "OutOfMemoryError: Metaspace" (README, "Memory"). With at most GRADLE_WORKERS (2) worker processes it asks
#     for 3 GB of Gradle heap plus 512 MB for the unit-test JVM.
#   * No javadoc jars (-PZCASH_PUBLISH_JAVADOC_JAR=false): Dokka writes the sections of deprecated.html in an order
#     that changes from run to run, so those jars, and the .module files that record their hashes, would differ
#     between two builds of the same commit. A normal dependency never asks for them.
#   * The release-keystore properties are passed empty: demo-app reads them, and a user-level gradle.properties
#     that sets them for an app with the same property names must not reach this build.
#   * No build cache: every file is produced by this run. GRADLE_ARGS adds Gradle options (e.g. --rerun-tasks).
#
# Needs JDK 17, the Android SDK at $ANDROID_HOME with NDK ANDROID_NDK_VERSION (platform 36 is fetched by the
# Android Gradle Plugin if missing and the SDK licences are accepted) and network access for Gradle dependencies.
set -euo pipefail
USAGE='usage: build_aar.sh <version> <maven-repo-dir> <so-dir> [gradle task ...]'
VERSION=${1:?$USAGE}
REPO=$(mkdir -p "${2:?$USAGE}" && cd "$2" && pwd)
SO_DIR=$(cd "${3:?$USAGE}" && pwd)
shift 3
TASKS=("$@")
[ ${#TASKS[@]} -gt 0 ] || TASKS=(
  :lightwallet-client-lib:publishToMavenLocal :backend-lib:publishToMavenLocal
  :sdk-lib:publishToMavenLocal :sdk-incubator-lib:publishToMavenLocal
  :backend-lib:testReleaseUnitTest :sdk-lib:testReleaseUnitTest
)
SDK=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
: "${ANDROID_HOME:?set ANDROID_HOME}"
JVMARGS=${GRADLE_JVMARGS:--Xmx3g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}
WORKERS=${GRADLE_WORKERS:-2}
read -r -a EXTRA_ARGS <<<"${GRADLE_ARGS:-}"
ABIS=(arm64-v8a armeabi-v7a x86 x86_64)

NDK_VERSION=$(sed -n 's/^ANDROID_NDK_VERSION=//p' "$SDK/gradle.properties")
[ -d "$ANDROID_HOME/ndk/$NDK_VERSION" ] ||
  { echo "no NDK $NDK_VERSION in $ANDROID_HOME/ndk: the libraries would be packaged unstripped" >&2; exit 2; }
JAVA_VERSION=$("${JAVA_HOME:+$JAVA_HOME/bin/}java" -version 2>&1 | sed -n 1p)
case $JAVA_VERSION in *'"17.'*) ;; *) echo "JDK 17 is needed, found: $JAVA_VERSION" >&2; exit 2 ;; esac

head_line() {
  echo "sdk_head=$(git -C "$SDK" rev-parse HEAD) dirty=$([ -n "$(git -C "$SDK" status --porcelain)" ] && echo yes || echo no)"
}
HEAD_LINE=$(head_line)   # before Gradle runs: what the artifacts are built from

JNI=$SDK/backend-lib/build/rustJniLibs/android
for abi in "${ABIS[@]}"; do
  mkdir -p "$JNI/$abi"
  cmp -s "$SO_DIR/$abi/libzcashwalletsdk.so" "$JNI/$abi/libzcashwalletsdk.so" ||
    cp "$SO_DIR/$abi/libzcashwalletsdk.so" "$JNI/$abi/libzcashwalletsdk.so"
done

cd "$SDK"
echo "build_aar: version=$VERSION $HEAD_LINE java=$JAVA_VERSION jvmargs=$JVMARGS workers=$WORKERS tasks=${TASKS[*]}"
./gradlew --no-daemon --no-build-cache --no-parallel --console=plain \
  "-Dorg.gradle.jvmargs=$JVMARGS" \
  "-Dorg.gradle.workers.max=$WORKERS" \
  "-Dmaven.repo.local=$REPO" \
  -Pkotlin.compiler.execution.strategy=in-process \
  "-PLIBRARY_VERSION=$VERSION" -PIS_SNAPSHOT=false -PZCASH_PUBLISH_JAVADOC_JAR=false \
  -PZCASH_RELEASE_KEYSTORE_PATH= -PZCASH_RELEASE_KEYSTORE_PASSWORD= \
  -PZCASH_RELEASE_KEY_ALIAS= -PZCASH_RELEASE_KEY_ALIAS_PASSWORD= \
  -x cargoBuild -x cargoBuildArm -x cargoBuildArm64 -x cargoBuildX86 -x cargoBuildX86_64 \
  ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} "${TASKS[@]}"

# The repository is complete when each artifact has exactly its four files; the listing covers those sixteen.
# (publishToMavenLocal also writes maven-metadata-local.xml beside the version folders. It carries the time of the
# run, is not needed to resolve a fixed version and is not part of the listing.)
cd "$REPO"
problems=0
for artifact in lightwallet-client zcash-android-backend zcash-android-sdk zcash-android-sdk-incubator; do
  dir=cash/z/ecc/android/$artifact/$VERSION
  for suffix in .aar .pom .module -sources.jar; do
    [ -f "$dir/$artifact-$VERSION$suffix" ] || { echo "not built yet: $dir/$artifact-$VERSION$suffix"; problems=$((problems + 1)); }
  done
  [ ! -d "$dir" ] || [ "$(find "$dir" -type f | wc -l)" -le 4 ] ||
    { echo "unexpected files in $dir (expected the four of this build)"; problems=$((problems + 1)); }
done
if [ "$problems" != 0 ]; then
  echo "build_aar: the repository is not complete; SHA256SUMS-$VERSION is written when all four artifacts are built"
  exit 0
fi
{
  echo "version=$VERSION"
  echo "$HEAD_LINE"
  for abi in "${ABIS[@]}"; do
    echo "so $abi sha256=$(sha256sum "$SO_DIR/$abi/libzcashwalletsdk.so" | cut -d' ' -f1) (build_so.sh output, before stripping)"
  done
  echo "java=$JAVA_VERSION"
  echo "gradle=$(sed -n 's/^distributionUrl=.*\/gradle-\(.*\)-bin\.zip$/\1/p' "$SDK/gradle/wrapper/gradle-wrapper.properties") ndk=$NDK_VERSION"
  echo "gradle_jvmargs=$JVMARGS workers=$WORKERS"
} > "BUILD-INFO-$VERSION.txt"
cat "BUILD-INFO-$VERSION.txt"
find cash/z/ecc/android -path "*/$VERSION/*" -type f | sort | xargs sha256sum | tee "SHA256SUMS-$VERSION"
