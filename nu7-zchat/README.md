# NU7 build of the Zcash Android SDK 3.0.0 (ZChat)

This branch (`nu7-zchat`) is the [Zcash Android wallet SDK](https://github.com/zcash/zcash-android-wallet-sdk)
v3.0.0 (MIT) with its native library rebuilt against the NU7 pre-release of librustzcash. It is built and used by
ZChat. It is **not** an Electric Coin Co. or ZODL release; if the official SDK ships NU7 support in time, ZChat
moves to it.

The Kotlin layer is unchanged: the published artifact `zcash-android-backend` is the official 3.0.0 AAR with its four
`libzcashwalletsdk.so` files replaced, and `zcash-android-sdk`, `zcash-android-sdk-incubator` and `lightwallet-client`
are the official 3.0.0 AARs, republished under the new version. Apps change only the SDK version.

## What changes compared with 3.0.0

- **NU7**: consensus branch `0x77190ad9`; testnet activation at height 4,465,026 (ZIP 259). Mainnet has no NU7
  height in this pin; it is set on 20 Oct 2026, and the library must be rebuilt from a pin that carries it before it
  can follow mainnet.
- **librustzcash**: the wallet crates come from commit `8dfc254da0818208d7256fc1d2c77be75e177805` (the tree of tag
  `zcash_keys-0.17.0-pre.0`) through `[patch.crates-io]`, because they had no crates.io pre-release on 2 Oct 2026.
  This brings unreleased wallet changes besides NU7, including three new one-way wallet-database migrations and
  transparent output and spend detection during block scanning.
- **Address strings**: the SDK keeps returning ZIP 316 Revision 0 Unified Addresses and viewing keys (`u1…`,
  `uview1…`, `uivk1…`; `utest1…` etc. on testnet), as 3.0.0 did, although `zcash_keys 0.17` produces Revision 2
  (`zu…`, `uvf…`, `uvi…`). Known gap: strings that the wallet database itself stores for addresses created after the
  upgrade are Revision 2 and are visible through `v_tx_outputs.to_address`.
- Unchanged: the Kotlin API, JNI function names, `DEFAULT_TX_EXPIRY_DELTA` (40), ZIP 325 metadata keys.

Commits: `backend-lib: build against the NU7 pre-release librustzcash cohort`, `backend-lib: keep returning ZIP 316
Revision 0 addresses and viewing keys`, `backend-lib: test the branch ids reported around NU7`.

## Rebuild

Needs Linux x86_64, rustup, Android NDK 27.0.12077973 (`ANDROID_NDK_HOME`, or `ANDROID_HOME` with `ndk/27.0.12077973`),
Python 3 and network access to crates.io, GitHub and Maven Central.

```bash
rustup toolchain install 1.92.0
rustup target add --toolchain 1.92.0 aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
for abi in arm64-v8a armeabi-v7a x86 x86_64; do nu7-zchat/build_so.sh "$abi"; nu7-zchat/verify_so.sh "nu7-zchat/out/$abi/libzcashwalletsdk.so"; done
nu7-zchat/package_aar.py --version 3.0.0-nu7-zchat.1 --repo /path/to/maven-repo
```

- `build_so.sh` mirrors the Gradle `cargo {}` block of `backend-lib/build.gradle.kts` (release profile, API 27,
  `-Wl,-z,max-page-size=16384`) and adds `--locked`, source-path remapping and a fixed `SOURCE_DATE_EPOCH`.
- `verify_so.sh` checks 16 KB alignment of every LOAD segment, that the network-upgrade name tables include `Nu7`,
  and that the librustzcash sources are the pinned commit.
- `package_aar.py` downloads the official 3.0.0 artifacts from Maven Central, checks them against
  `official-3.0.0.sha256`, strips the libraries with the NDK's `llvm-strip --strip-unneeded` (as the Android Gradle
  Plugin does for an AAR) and writes the four artifacts deterministically (official entry order and timestamps,
  stored entries).
- Compare your outputs with `EXPECTED-SHA256SUMS`. On the build machine a clean rebuild of the arm64 library and two
  packaging runs gave identical files; rebuilding on another machine has not been tried yet.

## Tests

`cd backend-lib && cargo +1.92.0 test --all-features` (68 pass; 14 live-wallet migration tests are ignored because
they need a real wallet database). `groundtruth-016/` derives the reference digests in
`backend-lib/src/main/rust/test_vectors.rs` with the crate versions of SDK 3.0.0's lock (`cargo run -- <path to
sdk-incubator-lib/.../fixture/WalletFixture.kt>`).

## Licences

The SDK and librustzcash are MIT / MIT-or-Apache-2.0. A licence report for every crate in `backend-lib/Cargo.lock`
(`cargo deny check licenses` with `backend-lib/deny.toml`) is part of the release checks and is not in this
directory yet.
