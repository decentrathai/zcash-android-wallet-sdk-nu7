#!/usr/bin/env bash
# publish_central.sh <version> <so-dir> — upload the four SDK artifacts of this commit, signed, to the Maven Central
# Portal as ONE deployment that is NOT released: the deployment waits in central.sonatype.com until someone presses
# "Publish" there (or drops it). Nothing else is pushed.
#
# Same build as build_aar.sh (it calls it, with the verified build_so.sh libraries), plus the javadoc jars that
# Maven Central asks for. The javadoc jars, and the .module files that record their hashes, are not reproducible
# (Dokka orders deprecated.html differently per run); every other file is the build_aar.sh file of the same commit.
#
# Credentials come only from the environment, never from a file in this repository:
#   ORG_GRADLE_PROJECT_mavenCentralUsername, ORG_GRADLE_PROJECT_mavenCentralPassword   (Portal user token)
#   ORG_GRADLE_PROJECT_ZCASH_ASCII_GPG_KEY            (base64 of the ASCII-armored secret signing key, on ONE line:
#                                                      base64 -w0; wrapped base64 fails at configuration)
#   ORG_GRADLE_PROJECT_ZCASH_ASCII_GPG_KEY_PASSWORD   (its passphrase; empty for none)
set -euo pipefail
USAGE='usage: publish_central.sh <version> <so-dir>'
VERSION=${1:?$USAGE}
SO_DIR=$(cd "${2:?$USAGE}" && pwd)
SDK=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

for v in ORG_GRADLE_PROJECT_mavenCentralUsername ORG_GRADLE_PROJECT_mavenCentralPassword ORG_GRADLE_PROJECT_ZCASH_ASCII_GPG_KEY; do
  [ -n "${!v:-}" ] || { echo "publish_central: $v is not set" >&2; exit 2; }
done
[ -z "$(git -C "$SDK" status --porcelain)" ] || { echo "publish_central: the checkout is not clean" >&2; exit 2; }
[[ $VERSION =~ ^3\.0\.0-nu7-zchat\.[0-9]+$ ]] || { echo "publish_central: version must be 3.0.0-nu7-zchat.N" >&2; exit 2; }
HEAD_SHA=$(git -C "$SDK" rev-parse HEAD)
# The POM's SCM points at the public repository: upload only a commit that a tag of this version names.
[ "$(git -C "$SDK" rev-parse -q --verify "refs/tags/v$VERSION^{commit}" || true)" = "$HEAD_SHA" ] ||
  { echo "publish_central: HEAD is not the commit of tag v$VERSION (clone the pushed tag)" >&2; exit 2; }
OUT=$SDK/nu7-zchat/out/central-$VERSION
STAGING=$SDK/build/publish/staging
[ ! -e "$OUT" ] || { echo "publish_central: $OUT exists; use a fresh clone" >&2; exit 2; }
[ ! -e "$STAGING" ] || { echo "publish_central: $STAGING exists; use a fresh clone" >&2; exit 2; }
for abi in arm64-v8a armeabi-v7a x86 x86_64; do
  "$SDK/nu7-zchat/verify_so.sh" "$SO_DIR/$abi/libzcashwalletsdk.so"
done

GRADLE_ARGS="${GRADLE_ARGS:-} -PZCASH_PUBLISH_JAVADOC_JAR=true" \
  "$SDK/nu7-zchat/build_aar.sh" "$VERSION" "$OUT" "$SO_DIR" \
  :lightwallet-client-lib:publishToMavenCentral :backend-lib:publishToMavenCentral \
  :sdk-lib:publishToMavenCentral :sdk-incubator-lib:publishToMavenCentral

# What was uploaded: the plugin leaves its staging directory (one per run) on disk. Record its files, check that
# each artifact has its five files, each signed, and compare the reproducible ones with EXPECTED-SHA256SUMS.
GROUP_PATH=$(sed -n 's/^ZCASH_PUBLISH_GROUP=//p' "$SDK/gradle.properties" | tr . /)
mapfile -t RUNS < <(find "$STAGING" -mindepth 1 -maxdepth 1 -type d)
[ ${#RUNS[@]} = 1 ] || { echo "publish_central: expected one staging directory, found ${#RUNS[@]}" >&2; exit 1; }
cd "${RUNS[0]}"
problems=0
for artifact in lightwallet-client zcash-android-backend zcash-android-sdk zcash-android-sdk-incubator; do
  for suffix in .aar .pom .module -sources.jar -javadoc.jar; do
    f=$GROUP_PATH/$artifact/$VERSION/$artifact-$VERSION$suffix
    [ -f "$f" ] || { echo "missing in the upload: $f"; problems=$((problems + 1)); continue; }
    [ -f "$f.asc" ] || { echo "not signed: $f"; problems=$((problems + 1)); }
  done
done
find "$GROUP_PATH" -path "*/$VERSION/*" -type f ! -name '*.md5' ! -name '*.sha1' ! -name '*.sha256' ! -name '*.sha512' \
  | sort | xargs sha256sum > "$OUT/SHA256SUMS-central-$VERSION"
{ echo "version=$VERSION"; echo "sdk_head=$HEAD_SHA"; echo "staging=${RUNS[0]}"; } > "$OUT/BUILD-INFO-central-$VERSION.txt"
# .aar, .pom and -sources.jar must equal the two-build sums (the javadoc jars and .module files cannot, ZC-676).
while read -r sum path; do
  case $path in *"/$VERSION/"*.aar | *"/$VERSION/"*.pom | *"/$VERSION/"*-sources.jar) ;; *) continue ;; esac
  got=$(sha256sum "$path" 2>/dev/null | cut -d' ' -f1 || true)
  [ "$got" = "$sum" ] || { echo "differs from EXPECTED-SHA256SUMS: $path"; problems=$((problems + 1)); }
done < <(grep -v '^#' "$SDK/nu7-zchat/EXPECTED-SHA256SUMS" | grep -F "/$VERSION/")
[ "$(grep -v '^#' "$SDK/nu7-zchat/EXPECTED-SHA256SUMS" | grep -cF "/$VERSION/")" -ge 12 ] ||
  { echo "EXPECTED-SHA256SUMS has no complete $VERSION lines"; problems=$((problems + 1)); }
echo "publish_central: upload record in $OUT (SHA256SUMS-central-$VERSION, BUILD-INFO-central-$VERSION.txt)"
if [ "$problems" != 0 ]; then
  echo "publish_central: $problems problem(s): DROP this deployment on central.sonatype.com, do not Publish it" >&2
  exit 1
fi
echo "publish_central: uploaded $VERSION as an unreleased deployment; check it on central.sonatype.com before Publish"
