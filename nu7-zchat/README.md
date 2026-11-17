# NU7 build of the Zcash Android SDK 3.0.0 (ZChat)

This branch (`publish/nu7-zchat-3`, built on `nu7-zchat-3`) is the [Zcash Android wallet SDK](https://github.com/zcash/zcash-android-wallet-sdk)
v3.0.0 (MIT) with its native library rebuilt against the NU7 releases of librustzcash and one small Kotlin change
that goes with it. It is built and used by ZChat. It is **not** an Electric Coin Co. or ZODL release; if the official
SDK ships NU7 support in time, ZChat moves to it.

Version `3.0.0-nu7-zchat.3` is built from this branch: `zcash-android-backend`, `zcash-android-sdk`,
`zcash-android-sdk-incubator` and `lightwallet-client`, each as `.aar`, `.pom`, `.module` and `-sources.jar`. An app
changes the SDK version and adds the repository that holds these files; no app code changes. (Version `.1` was the
official 3.0.0 AARs with only the native libraries replaced; `package_aar.py` built it and stays here for that.
Version `.2` used the NU7 pre-release crates and had no mainnet NU7 height.)

The artifacts are meant for Maven Central under the group `io.github.decentrathai` (gradle.properties
`ZCASH_PUBLISH_GROUP`), not under the official `cash.z.ecc.android` group, so they never replace the official SDK
by accident.

## What changes compared with 3.0.0

- **NU7**: consensus branch `0x77190ad9`; testnet activation at height 4,465,026 and mainnet activation at height
  3,543,405 (ZIP 259).
- **librustzcash**: the published NU7 releases from crates.io: zcash_client_backend 0.25.0,
  zcash_client_sqlite 0.23.0, pczt 0.10.0, zip321 0.10.0, zcash_pool_migration 0.2.0, zcash_keys 0.17.0,
  zcash_primitives and zcash_proofs 0.31.0, zcash_protocol and zcash_transparent 0.11.0, zcash_address 0.14.0. They
  bring wallet changes besides NU7, including three new one-way
  wallet-database migrations and transparent output and spend detection during block scanning.
- **Address strings**: the SDK keeps returning ZIP 316 Revision 0 Unified Addresses and viewing keys (`u1…`,
  `uview1…`, `uivk1…`; `utest1…` etc. on testnet), as 3.0.0 did, although `zcash_keys 0.17` produces Revision 2
  (`zu…`, `tu…`, `uvf…`, `uvi…`). That includes the strings the wallet database stores: addresses created by this
  build are stored in Revision 2, and `Synchronizer.getRecipients` and `getTransactionOutputs` return them in
  Revision 0. Stored rows are never rewritten. The crates decode both forms (tested for addresses and viewing keys).
- **Kotlin layer**: three source files differ from 3.0.0.
  - `backend-lib`: new `cash.z.ecc.android.sdk.internal.jni.RustUnifiedEncoding` with one JNI function,
    `toRevision0(String): String`. It needs the native library to be loaded, like every other JNI call.
  - `sdk-lib`: new internal `StoredAddressNormalizer` (the 512 most recently used strings are cached in front of
    that function); `TxOutputsView` passes `v_tx_outputs.to_address` through it.
  - The version is part of `BuildConfig.LIBRARY_VERSION` and of the Kotlin module name of `zcash-android-sdk`. So
    the SDK's wallet check that runs once per SDK version (`fixWitnesses`) runs once after an app switches to this
    build, and the JVM names of Kotlin `internal` members end in `$zcash_android_sdk_3_0_0_nu7_zchat_3_release`.
- **Compared with the official 3.0.0 AARs**, file by file: `lightwallet-client` and `zcash-android-sdk-incubator`
  are byte-identical to them. In `zcash-android-backend` the 421 files of `classes.jar` are byte-identical, and
  `RustUnifiedEncoding.class` and the four rebuilt libraries are new. In `zcash-android-sdk` the public and
  protected API (`javap` over every class) differs only by the internal classes named above.
- **Build files**: publications are signed only when a signing key is configured, and a build can leave the javadoc
  jar out (`-PZCASH_PUBLISH_JAVADOC_JAR=false`); the group, POM project page and SCM come from `gradle.properties`
  (`ZCASH_PUBLISH_GROUP`, `ZCASH_PUBLISH_GITHUB_REPO`); the signing key may have a passphrase
  (`ZCASH_ASCII_GPG_KEY_PASSWORD`), and a Maven Central publish without a key stops before uploading.
- Unchanged: the public Kotlin API, the JNI functions of 3.0.0 (the libraries export those 121 and `toRevision0`),
  `DEFAULT_TX_EXPIRY_DELTA` (40), ZIP 325 metadata keys.

## Rebuild

Needs Linux x86_64, rustup, JDK 17, the Android SDK with NDK 27.0.12077973 (`ANDROID_HOME`), Python 3 and network
access to crates.io, GitHub, Google's Maven repository and Maven Central. Build from a clean checkout of the commit
(`build-info.txt` and `BUILD-INFO-<version>.txt` then say `dirty=no`).

```bash
rustup toolchain install 1.92.0
rustup target add --toolchain 1.92.0 \
  aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
export ANDROID_HOME=/path/to/android-sdk
for abi in arm64-v8a armeabi-v7a x86 x86_64; do
  nu7-zchat/build_so.sh "$abi"
  nu7-zchat/verify_so.sh "nu7-zchat/out/$abi/libzcashwalletsdk.so"
done
nu7-zchat/build_aar.sh 3.0.0-nu7-zchat.3 /path/to/maven-repo nu7-zchat/out
```

- `build_so.sh` mirrors the Gradle `cargo {}` block of `backend-lib/build.gradle.kts` (release profile, API 27,
  `-Wl,-z,max-page-size=16384`) and adds `--locked`, source-path remapping and a fixed `SOURCE_DATE_EPOCH` (the time
  of the last commit that touches `backend-lib`).
- `verify_so.sh` checks one library, before or after stripping: 16 KB alignment of every LOAD segment; the
  network-upgrade name tables list `Nu7`; the consensus branch-id table continues with `0x77190ad9` and the testnet
  and mainnet NU7 heights are compiled in (`so_constants.py`); the wallet crates are exactly the stable releases
  named above (any `-pre`, `-rc`, `-alpha` or `-beta` crate fails); no path of the
  build machine; `RustUnifiedEncoding.toRevision0` is exported. The libraries of the official 3.0.0 AAR fail it.
- `build_aar.sh` runs the SDK's own Gradle build once: it publishes the four artifacts into the directory with
  `publishToMavenLocal` and runs the unit tests of `backend-lib` and `sdk-lib`. The libraries are the `build_so.sh`
  outputs; the Android Gradle Plugin strips them (`llvm-strip --strip-unneeded` of the NDK). It writes
  `SHA256SUMS-<version>` and `BUILD-INFO-<version>.txt` beside the artifacts. No javadoc jars: Dokka writes the
  sections of `deprecated.html` in an order that changes from run to run, so those jars and the `.module` files
  that record their hashes would differ between two builds of one commit.
- Compare your outputs with `EXPECTED-SHA256SUMS` (it holds `.1`, `.2` and `.3`; `.3` was built twice from two clean
  clones under the `io.github.decentrathai` group, since the group is part of every `.pom` and `.module`). What was
  checked is written at its top: the `.3` .aar and sources jars came out identical on two PCs. The `.aar` files are
  compressed zip files, so on a machine with another zlib they may differ in their bytes while every entry is the
  same.

### Use from an app

From Maven Central (`mavenCentral()` in the app's repositories):

```kotlin
implementation("io.github.decentrathai:zcash-android-sdk:3.0.0-nu7-zchat.3")
// optional, as with the official SDK:
implementation("io.github.decentrathai:zcash-android-sdk-incubator:3.0.0-nu7-zchat.3")
```

The backend and the lightwallet client follow as dependencies; `kotlin-bip39` stays
`cash.z.ecc.android:kotlin-bip39` from Maven Central. The `.module` files offer the API, runtime and sources
variants. The Kotlin packages are the official ones (`cash.z.ecc.android.sdk...`), so app code does not change.

Do not also depend on the official `cash.z.ecc.android:zcash-android-sdk` (directly or through another library):
the groups differ, so Gradle does not pick one of them, and the build fails with duplicate classes. Check with
`./gradlew :app:dependencies`, or run `verify_so.sh` on the `libzcashwalletsdk.so` of the built APK.

A local directory filled by `build_aar.sh` is a Maven repository too: add `maven { url = uri("/path/to/maven-repo") }`
before `mavenCentral()`, ideally inside `exclusiveContent { ... filter { includeGroup("io.github.decentrathai") } }`.

### Memory

`build_aar.sh` gives the Gradle JVM `-Xmx3g -XX:MaxMetaspaceSize=1g` on the command line, the values of this
repository's `gradle.properties`, because a user-level `~/.gradle/gradle.properties` overrides the project file. A
512 MiB Metaspace cap from such a file stopped this build with `OutOfMemoryError: Metaspace`. Measured on the build
machine (one Gradle JVM, Kotlin compiler in-process, two workers, every task executed): 330 MiB of Metaspace
without the javadoc jars and 511 MiB with them, a heap of up to 1.2 GB in use, and up to 2.8 GB resident memory for
that JVM; the unit-test JVM has a 512 MB heap. No Kotlin daemon is started.

## Tests

- Rust: `cd backend-lib && cargo +1.92.0 test --locked --all-features`: 71 pass, 14 are ignored (the live-wallet
  migration tests, which need a real wallet database). `cargo clippy --tests --all-features -- -D warnings` is
  clean; `backend-lib/clippy.toml` forbids the Revision 2 encoders outside `r0.rs`.
- Kotlin, run by `build_aar.sh`: `:sdk-lib:testReleaseUnitTest` 141 tests (`StoredAddressNormalizerTest` is 3 of
  them) and `:backend-lib:testReleaseUnitTest` 34 tests. One upstream class, `ConversionsTest`, is written for
  JUnit 5 and is not picked up by the module's JUnit 4 runner; that is unchanged from 3.0.0.
- Not covered by these: the JNI call from Kotlin (`RustUnifiedEncoding.toRevision0` is tested in Rust, and its
  export is checked in each library, but no instrumented test calls it on a device), and the SDK's own
  `androidTest` suites, which need a device or emulator.
- `groundtruth-016/` derives the reference digests in `backend-lib/src/main/rust/test_vectors.rs` with the crate
  versions of SDK 3.0.0's lock (`cargo run -- <path to sdk-incubator-lib/.../fixture/WalletFixture.kt>`).

## Licences

The SDK and librustzcash are MIT / MIT-or-Apache-2.0. `cargo deny check licenses` with `backend-lib/deny.toml`
passes on `backend-lib/Cargo.lock` (cargo-deny 0.20.2): every crate is under a licence on its allow list, which has
no copyleft licence (no GPL, LGPL or AGPL). `THIRD-PARTY-LICENSES.md` lists the 527 crates that reach the four
Android targets, with their licence texts; it is generated by cargo-about 0.9.3 with `about.toml` and `about.hbs`:

    cargo about generate --locked -c nu7-zchat/about.toml -m backend-lib/Cargo.toml \
      -o nu7-zchat/THIRD-PARTY-LICENSES.md nu7-zchat/about.hbs
