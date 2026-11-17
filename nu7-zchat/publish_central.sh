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
#   ORG_GRADLE_PROJECT_ZCASH_ASCII_GPG_KEY            (base64 of the ASCII-armored secret signing key)
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
case $VERSION in *-SNAPSHOT) echo "publish_central: no snapshots" >&2; exit 2 ;; esac
for abi in arm64-v8a armeabi-v7a x86 x86_64; do
  "$SDK/nu7-zchat/verify_so.sh" "$SO_DIR/$abi/libzcashwalletsdk.so"
done

GRADLE_ARGS="${GRADLE_ARGS:-} -PZCASH_PUBLISH_JAVADOC_JAR=true" \
  "$SDK/nu7-zchat/build_aar.sh" "$VERSION" "$SDK/nu7-zchat/out/central-$VERSION" "$SO_DIR" \
  :lightwallet-client-lib:publishToMavenCentral :backend-lib:publishToMavenCentral \
  :sdk-lib:publishToMavenCentral :sdk-incubator-lib:publishToMavenCentral
echo "publish_central: uploaded $VERSION as an unreleased deployment; check it on central.sonatype.com before Publish"
