#!/usr/bin/env bash
# verify_so.sh <libzcashwalletsdk.so> — release-gate facts about one built native library (light, read-only).
# Prints: LOAD segment alignment (must all be 0x4000 = 16 KB), the network-upgrade names it carries (must include
# Nu7), the librustzcash pin seen in remapped source paths, NEEDED libraries and size. Exit 1 if a must fails.
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
pins=$("$TC/llvm-strings" -n 8 "$SO" | grep -oE 'librustzcash-[0-9a-f]+/[0-9a-f]{7}' | sort -u | tr '\n' ' ')
echo "librustzcash checkouts in paths: ${pins:-none}"
echo "$pins" | grep -q '/8dfc254' || { echo "FAIL: pin 8dfc254 not seen"; fail=1; }
home_paths=$("$TC/llvm-strings" -n 8 "$SO" | grep -c "$HOME" || true)
echo "unmapped \$HOME paths: $home_paths"
echo "NEEDED: $("$TC/llvm-readelf" -dW "$SO" | awk '/NEEDED/{print $NF}' | tr -d '[]' | tr '\n' ' ')"
echo "size: $(stat -c %s "$SO") bytes; sha256 $(sha256sum "$SO" | cut -c1-16)…"
exit $fail
