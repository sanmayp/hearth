# Tauri (desktop plus Android)

Status: shell is scaffolded. No Android toolchain on the host yet. No APK yet.

## What is done

- `cargo tauri init` ran in `web/`. The shell lives in `web/src-tauri/`.
- `tauri.conf.json` keeps Hearth values. `frontendDist` points at `../build`.
- Package name is `hearth`. Lib name is `hearth_lib`.
- The shell stands outside the backend workspace. It builds on its own.
- Tauri CLI 2.12.4 sits in `.tooling/cargo/bin`. It is local only.

## What is missing (owner runs these on the host, outside the sandbox)

1. JDK 17 or newer. Only JDK 8 is present now.
2. Android SDK command line tools, platform, and build tools. Set `ANDROID_HOME`.
3. Android NDK. Set `NDK_HOME`.
4. Rust targets: `rustup target add aarch64-linux-android x86_64-linux-android`.
5. Accept the SDK licenses. Install `adb` from platform tools.

Desktop `cargo check` also wants system dbus libs. The Android build does not need them.

## Android build (after the toolchain lands)

Only one Android build runs at a time.

1. `cd web && cargo tauri android init`.
2. Build with the tailnet API base (never localhost, never a baked key).
   Example shape only. The owner confirms the exact URL first:
   `PUBLIC_HEARTH_API_BASE=http://127.0.0.1:8187 cargo tauri android build --debug`.
3. The key is entered on the Settings page at runtime. It never ships in the APK.

## Install on the S25

The phone is on the tailnet as `sanjays-s25-ultra`. Use USB adb:

`adb install -r web/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk`

Check the exact APK path after the build. The name varies by ABI split.

## Release updates (in-app)

Releases carry a version code from `web/src-tauri/version-code.txt`.
The counter starts at 1001, above all debug builds. Each publish adds one.

Keys (made once on 2026-11-01, live in `~/.config/lifeos`, mode 600):

- `hearth-release.keystore` (RSA 4096, alias `hearth`, valid to 2056)
- `update-signing.key` (Ed25519, signs the manifest)
- `.storepass` (holds the keystore password, made random, never printed)

Recreate commands, for reference only:

`keytool -genkeypair -keystore ~/.config/lifeos/hearth-release.keystore -alias hearth -keyalg RSA -keysize 4096 -validity 10950 -storetype PKCS12`
`openssl genpkey -algorithm ed25519 -out ~/.config/lifeos/update-signing.key`

Only the release key fingerprint ever leaves that folder. Keys never
enter git, builds, logs, or chat. Rotate means replace the keystore
and `.storepass` together, then publish once.

One publish command (`HEARTH_KEYSTORE_PASSWORD` in env overrides the file):

`scripts/publish-apk.sh "Release notes here."`

It builds the release APK in the image, signs it, checks code plus
key, writes and signs the manifest, and serves both from the update
folder. It keeps the last two APKs. It prints code, size, hash, and
fingerprint only.

Owner taps on the phone: Settings shows the update. Tap Update. Tap
Install once in the system dialog. The first release needs a fresh
install: it carries a new signing key, and Android refuses to update
a debug-signed app with it. Later releases update in place.

The app checks on open and once a day while open. No background
service runs. Checks verify the manifest signature first, then the
host, the code, and the file hash. Anything bad stops the update.
