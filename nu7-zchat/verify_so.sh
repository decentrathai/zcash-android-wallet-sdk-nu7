#!/usr/bin/env bash
# verify_so.sh <libzcashwalletsdk.so> — release-gate facts about one built native library (light, read-only).
# Prints: LOAD segment alignment (must all be 0x4000 = 16 KB), the network-upgrade names it carries (must include
# Nu7), the wallet-crate versions seen in remapped source paths, NEEDED libraries and size. Exit 1 if a must fails.
set -u
SO=${1:?usage: verify_so.sh <so>}
NDK=${ANDROID_NDK_HOME:-${ANDROID_HOME:?set ANDROID_NDK_HOME or ANDROID_HOME}/ndk/27.0.12077973}
TC=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin
fail=0
aligns=$("$TC/llvm-readelf" -lW "$SO" | awk '$1=="LOAD"{print $NF}' | sort -u | tr '\n' ' ')
echo "LOAD alignments: $aligns"
[ "$aligns" = "0x4000 " ] || { echo "FAIL: not 16 KB aligned"; fail=1; }
# Rust stores enum variant names back to back ("...CanopyNu5Nu6Nu6_1Nu6_2Nu6_3..."), so a stray "Nu7" in binary
# data proves nothing (SDK 3.0.0 has one); count the network-upgrade name tables that continue with Nu7.
read -r tables with_nu7 < <(python3 -c '
import sys; b=open(sys.argv[1],"rb").read()
print(b.count(b"Nu6_2Nu6_3"), b.count(b"Nu6_2Nu6_3Nu7"))' "$SO")
echo "upgrade-name tables: $tables, of which end ...Nu6_3Nu7: $with_nu7"
[ "$with_nu7" -ge 1 ] || { echo "FAIL: no network-upgrade table lists Nu7"; fail=1; }
# The wallet crates must be the published NU7 pre-releases (their versions appear in the remapped source paths).
crates=$("$TC/llvm-strings" -n 8 "$SO" | grep -oE '(zcash_client_backend|zcash_client_sqlite|zcash_keys|zcash_protocol|zcash_primitives|pczt|zcash_pool_migration)-[0-9][0-9a-z.-]*' | sed -E 's/[.-]$//' | sort -u | tr '\n' ' ')
echo "librustzcash crates in paths: ${crates:-none}"
for want in zcash_client_backend-0.25.0-pre.0 zcash_client_sqlite-0.23.0-pre.0 zcash_keys-0.17.0-pre.0 zcash_protocol-0.11.0-pre.0; do
  echo "$crates" | grep -q "$want" || { echo "FAIL: $want not seen"; fail=1; }
done
git_src=$("$TC/llvm-strings" -n 8 "$SO" | grep -c 'git/checkouts/librustzcash' || true)
[ "$git_src" = 0 ] || { echo "FAIL: librustzcash sources from a git checkout ($git_src paths)"; fail=1; }
home_paths=$("$TC/llvm-strings" -n 8 "$SO" | grep -c "$HOME" || true)
echo "unmapped \$HOME paths: $home_paths"
echo "NEEDED: $("$TC/llvm-readelf" -dW "$SO" | awk '/NEEDED/{print $NF}' | tr -d '[]' | tr '\n' ' ')"
echo "size: $(stat -c %s "$SO") bytes; sha256 $(sha256sum "$SO" | cut -c1-16)…"
exit $fail
