#!/usr/bin/env python3
"""package_aar.py — build the local Maven repository for the NU7 SDK (version e.g. 3.0.0-nu7-zchat.1).

Inputs:
  * the official SDK 3.0.0 artifacts from Maven Central (zcash-android-backend, zcash-android-sdk,
    zcash-android-sdk-incubator, lightwallet-client: .aar, .pom, .module), downloaded and checked against the
    SHA-256 values pinned below (they are the published MIT builds of tag v3.0.0);
  * our four libzcashwalletsdk.so builds (build_so.sh output, default nu7-zchat/out/<abi>/), stripped here with the NDK's
    llvm-strip --strip-unneeded, as the Android Gradle Plugin strips native libraries of a library AAR.

Output: <repo>/cash/z/ecc/android/<artifact>/<version>/ with .aar/.pom/.module and .md5/.sha1/.sha256/.sha512.
  * zcash-android-backend: the official AAR with jni/<abi>/libzcashwalletsdk.so replaced; every other entry is the
    official one, byte for byte. The Kotlin classes therefore stay exactly 3.0.0's.
  * the other three: the official AAR bytes unchanged; only their .pom/.module are rewritten to the new version.

Deterministic: entries keep the official order, timestamps and attributes and are written uncompressed (ZIP_STORED),
so the output depends only on the inputs, not on the zlib version. Run it twice and compare (see --check-against).

Usage: package_aar.py --version 3.0.0-nu7-zchat.1 --repo <dir> [--so-dir <dir>] [--check-against <other repo>]
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

GROUP = "cash.z.ecc.android"
BASE = "3.0.0"
CENTRAL = "https://repo1.maven.org/maven2"
ABIS = ["arm64-v8a", "armeabi-v7a", "x86", "x86_64"]
ARTIFACTS = ["zcash-android-backend", "zcash-android-sdk", "zcash-android-sdk-incubator", "lightwallet-client"]
HERE = pathlib.Path(__file__).resolve().parent
NDK = pathlib.Path(os.environ.get("ANDROID_NDK_HOME") or pathlib.Path(os.environ["ANDROID_HOME"]) / "ndk/27.0.12077973")
NDK_STRIP = NDK / "toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"
CACHE = HERE / "official-3.0.0"
# SHA-256 of the official 3.0.0 files from Maven Central; checked on every run.
PINS_FILE = HERE / "official-3.0.0.sha256"


def sha(path, algo):
    h = hashlib.new(algo)
    h.update(pathlib.Path(path).read_bytes())
    return h.hexdigest()


def fetch_official(pin):
    """Download the official 3.0.0 files once and check them against the pinned SHA-256 values."""
    CACHE.mkdir(parents=True, exist_ok=True)
    pins = {}
    if PINS_FILE.exists():
        for line in PINS_FILE.read_text().splitlines():
            digest, name = line.split()
            pins[name] = digest
    elif not pin:
        sys.exit(f"no pins at {PINS_FILE}; run once with --pin after checking the downloads")
    for art in ARTIFACTS:
        for ext in ("aar", "pom", "module"):
            name = f"{art}-{BASE}.{ext}"
            dest = CACHE / name
            if not dest.exists():
                url = f"{CENTRAL}/{GROUP.replace('.', '/')}/{art}/{BASE}/{name}"
                with urllib.request.urlopen(url, timeout=120) as r:
                    dest.write_bytes(r.read())
            digest = sha(dest, "sha256")
            if name in pins and pins[name] != digest:
                sys.exit(f"SHA-256 mismatch for official {name}")
            pins[name] = digest
    if pin and not PINS_FILE.exists():
        PINS_FILE.write_text("".join(f"{d}  {n}\n" for n, d in sorted(pins.items())))


def strip_so(src, dest):
    subprocess.run([str(NDK_STRIP), "--strip-unneeded", "-o", str(dest), str(src)], check=True)


def repack_backend(official_aar, so_dir, out_aar):
    with tempfile.TemporaryDirectory(dir=HERE) as tmp:
        stripped = {}
        for abi in ABIS:
            dest = pathlib.Path(tmp) / f"{abi}.so"
            strip_so(pathlib.Path(so_dir) / abi / "libzcashwalletsdk.so", dest)
            stripped[f"jni/{abi}/libzcashwalletsdk.so"] = dest.read_bytes()
        replaced = set()
        with zipfile.ZipFile(official_aar) as zin, zipfile.ZipFile(out_aar, "w") as zout:
            for info in zin.infolist():
                data = stripped.get(info.filename)
                if data is None:
                    data = zin.read(info)
                else:
                    replaced.add(info.filename)
                out = zipfile.ZipInfo(info.filename, date_time=info.date_time)
                out.compress_type = zipfile.ZIP_STORED
                out.external_attr = info.external_attr
                out.create_system = info.create_system
                zout.writestr(out, data)
        missing = set(stripped) - replaced
        if missing:
            sys.exit(f"official AAR has no entry for {sorted(missing)}")


def rewrite_pom(text, version):
    # The artifact's own <version> and every cash.z.ecc.android dependency on BASE move to the new version.
    text = re.sub(rf"(<artifactId>[^<]+</artifactId>\s*<version>){re.escape(BASE)}(</version>)", rf"\g<1>{version}\2",
                  text)
    return text


def rewrite_module(text, art, version, aar_path):
    doc = json.loads(text)
    doc["component"]["version"] = version
    for variant in doc.get("variants", []):
        for dep in variant.get("dependencies", []):
            if dep.get("group") == GROUP and dep.get("version", {}).get("requires") == BASE:
                dep["version"]["requires"] = version
        for f in variant.get("files", []):
            if f["name"] == f"{art}-{BASE}.aar":
                f["name"] = f["url"] = f"{art}-{version}.aar"
                f["size"] = aar_path.stat().st_size
                for algo in ("sha512", "sha256", "sha1", "md5"):
                    f[algo] = sha(aar_path, algo)
    return json.dumps(doc, indent=2) + "\n"


def write_checksums(path):
    for algo in ("md5", "sha1", "sha256", "sha512"):
        pathlib.Path(f"{path}.{algo}").write_text(sha(path, algo))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--version", required=True)
    ap.add_argument("--repo", required=True)
    ap.add_argument("--so-dir", default=str(HERE / "out"))
    ap.add_argument("--pin", action="store_true", help="record the official SHA-256 values on first download")
    ap.add_argument("--check-against", help="another repo built from the same inputs; all files must be equal")
    a = ap.parse_args()

    fetch_official(a.pin)
    repo = pathlib.Path(a.repo)
    for art in ARTIFACTS:
        d = repo / GROUP.replace(".", "/") / art / a.version
        if d.exists():
            shutil.rmtree(d)
        d.mkdir(parents=True)
        aar = d / f"{art}-{a.version}.aar"
        if art == "zcash-android-backend":
            repack_backend(CACHE / f"{art}-{BASE}.aar", a.so_dir, aar)
        else:
            shutil.copyfile(CACHE / f"{art}-{BASE}.aar", aar)
        pom = d / f"{art}-{a.version}.pom"
        pom.write_text(rewrite_pom((CACHE / f"{art}-{BASE}.pom").read_text(), a.version))
        mod = d / f"{art}-{a.version}.module"
        mod.write_text(rewrite_module((CACHE / f"{art}-{BASE}.module").read_text(), art, a.version, aar))
        for p in (aar, pom, mod):
            write_checksums(p)

    files = sorted(p for p in repo.rglob("*") if p.is_file() and a.version in str(p))
    sums = "".join(f"{sha(p, 'sha256')}  {p.relative_to(repo)}\n" for p in files)
    (repo / f"SHA256SUMS-{a.version}").write_text(sums)
    print(sums, end="")
    if a.check_against:
        other = pathlib.Path(a.check_against) / f"SHA256SUMS-{a.version}"
        if other.read_text() != sums:
            sys.exit("NOT REPRODUCIBLE: SHA256SUMS differ from " + str(other))
        print("REPRODUCIBLE: identical to", other)


if __name__ == "__main__":
    main()
