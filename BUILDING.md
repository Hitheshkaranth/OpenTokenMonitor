# Building OpenTokenMonitor

## Local Development

1. `npm install`
2. `npm run tauri dev`

This runs the frontend dev server and Tauri app in developer mode.

## macOS Production Build

Command:

`npm run tauri:build:mac`

Optional signing and notarization environment variables:

- `APPLE_SIGNING_IDENTITY` (for example: `Developer ID Application: Your Name (TEAMID)`)
- `APPLE_ID`
- `APPLE_PASSWORD` (app-specific password)
- `APPLE_TEAM_ID`
- `APPLE_CERTIFICATE` (base64-encoded `.p12`)
- `APPLE_CERTIFICATE_PASSWORD`

Notes:

- `bundle.macOS.signingIdentity` is set to `"-"` in config. If `APPLE_SIGNING_IDENTITY` is not set, the build remains ad-hoc/unsigned for local testing.
- If Apple notarization variables are set (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`), Tauri bundler will attempt notarization automatically.
- Do not commit certificates, team IDs, or passwords.
- `entitlements.plist` requests only `allow-jit` (WebKit needs it) plus outbound
  network. `disable-library-validation` and `allow-unsigned-executable-memory`
  were removed — if a future dependency needs one of them, add it back
  deliberately and say why, rather than restoring both.

## Windows Production Build

Command:

`npm run tauri:build:win`

Optional code-signing environment variables:

- `WINDOWS_CERTIFICATE` (base64-encoded `.p12`)
- `WINDOWS_CERTIFICATE_PASSWORD`

Notes:

- If Windows certificate variables are not set, NSIS artifacts are generated unsigned
  and users will hit SmartScreen's "Windows protected your PC" interstitial.
- In CI, `release.yml` imports `WINDOWS_CERTIFICATE` into the runner's certificate
  store and stamps the resulting thumbprint into `bundle.windows.certificateThumbprint`
  before building. Locally, install the certificate yourself and set that field.
- `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are for updater signatures, not Windows code signing.

## Artifact Paths

Build outputs are written under:

- `src-tauri/target/release/bundle/macos/`
- `src-tauri/target/release/bundle/dmg/`
- `src-tauri/target/release/bundle/nsis/`
- `src-tauri/target/release/bundle/msi/`

## Troubleshooting

If Gatekeeper rejects the app:

`spctl -a -t exec -vv <appname>.app`

If notarization fails, inspect notarization logs:

`xcrun notarytool log <submission-id> --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_PASSWORD"`

## CI / Releasing

Pull request and `main` branch validation are handled by `.github/workflows/ci.yml`.
That workflow runs install/build checks plus Rust formatting, clippy, and tests on Ubuntu.
It intentionally does not run full Tauri packaging (`tauri build`) to keep PR CI fast and lower-cost.

Release packaging/signing is handled by `.github/workflows/release.yml`.
To cut a release, create and push a SemVer tag:

`git tag v0.4.0 && git push --tags`

This triggers the release workflow for macOS (Apple Silicon + Intel), Windows, and Linux (x86_64).
You can also run it manually from GitHub Actions using `workflow_dispatch`.

Required repository secrets (GitHub: Settings -> Secrets and variables -> Actions):

- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `WINDOWS_CERTIFICATE`
- `WINDOWS_CERTIFICATE_PASSWORD`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Notes:

- macOS cert import uses `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD` when present.
- Tauri updater signature files (`*.sig`) are only generated when `TAURI_SIGNING_PRIVATE_KEY` is set.
- Unsigned development release artifacts are still produced if you leave all signing/notarization secrets unset.
- Never commit signing certificates, private keys, or credential values into the repository.

## Updater Signing

The auto-updater is **inert until you generate a keypair.** `plugins.updater.pubkey`
ships as the literal string `PUBKEY_PLACEHOLDER`; the app detects that at runtime
(`is_updater_configured`) and skips the update check entirely, so an unconfigured
build makes no network call rather than 404ing against the release endpoint.

To turn it on:

1. Generate a keypair:

   `npx tauri signer generate -w ~/.tauri/otm-updater.key`

   This writes a private key (`~/.tauri/otm-updater.key`) and a public key
   (`~/.tauri/otm-updater.key.pub`).

2. In `src-tauri/tauri.conf.json`, under `plugins.updater`, replace
   `"PUBKEY_PLACEHOLDER"` with the **contents of the `.pub` file**.
   Nothing else needs changing — `bundle.createUpdaterArtifacts` is already
   `true`, which is what makes Tauri emit the `.tar.gz` / `.sig` pair.

3. Set the release secrets:

   - `TAURI_SIGNING_PRIVATE_KEY` = contents of `~/.tauri/otm-updater.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = the key password, if you set one

Never commit the private key file or its contents.

### How `latest.json` is produced

The updater polls
`https://github.com/<owner>/<repo>/releases/latest/download/latest.json`.
The `updater-manifest` job in `release.yml` builds that file automatically: each
platform build stages its `.sig` files as a workflow artifact, the manifest job
reads them, maps each target onto a Tauri platform key
(`darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`, `linux-x86_64`), and
attaches the composed `latest.json` to the draft release before it publishes.

The job **fails the release** if no signatures are found — that means either
`TAURI_SIGNING_PRIVATE_KEY` is unset or `createUpdaterArtifacts` got turned off.
That is deliberate: a release that silently ships without an updater manifest is
how you end up unable to push a fix.

One detail worth knowing: Tauri names the macOS updater bundle
`OpenTokenMonitor.app.tar.gz` on both arm64 and x86_64. Release assets are a flat
namespace, so the collect step renames them to `<target>-OpenTokenMonitor.app.tar.gz`.
Without that, the second upload silently overwrites the first and half your Mac
users get the wrong architecture.

## Release Checklist

Packaging is not covered by CI — `.github/workflows/ci.yml` deliberately skips
`tauri build`. Before tagging, run the manual smoke pass in
[`RELEASE_CHECKLIST.md`](./RELEASE_CHECKLIST.md) on a clean VM per platform.
