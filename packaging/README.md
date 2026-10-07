# Package managers

| Channel | Install command | Source of truth |
|---|---|---|
| winget (Windows) | `winget install Hitheshkaranth.OpenTokenMonitor` | [`winget/manifests/…`](./winget/manifests/h/Hitheshkaranth/OpenTokenMonitor) — first submission only |
| Homebrew (macOS) | `brew install --cask hitheshkaranth/tap/opentokenmonitor` | [`homebrew-tap/`](./homebrew-tap) — contents of the `Hitheshkaranth/homebrew-tap` repo |
| AUR (Arch Linux) | `yay -S opentokenmonitor-bin` | [`aur/PKGBUILD`](./aur/PKGBUILD) |

After the one-time setup below, the `package-managers` job in
[`release.yml`](../.github/workflows/release.yml) updates all three whenever a
release is published. Each channel is skipped until its secret exists.

## One-time setup

### Homebrew

1. Publish the tap (the repo name must start with `homebrew-`):

   ```bash
   cd packaging/homebrew-tap
   git init -b main && git add -A && git commit -m "opentokenmonitor 0.4.1 (new cask)"
   gh repo create Hitheshkaranth/homebrew-tap --public --source . --push \
     --description "Homebrew casks for OpenTokenMonitor"
   ```

   Its `Test casks` workflow audits and installs the cask on Apple Silicon and Intel runners.
2. Create a fine-grained token with **Contents: read and write** on `homebrew-tap` only,
   and save it in this repo as the `HOMEBREW_TAP_TOKEN` secret.

### winget

1. Submit the first version (new packages must be added by a pull request to
   `microsoft/winget-pkgs`; later versions are automated):

   ```powershell
   winget install Microsoft.WingetCreate
   wingetcreate submit --token <classic PAT with public_repo> `
     packaging\winget\manifests\h\Hitheshkaranth\OpenTokenMonitor\0.4.1
   ```

   On the pull request, reply `@microsoft-github-policy-service agree` to sign the CLA,
   then wait for validation and moderator approval (usually a few days).
2. Keep your fork of `microsoft/winget-pkgs` (created by the step above) and save the
   same classic token as the `WINGET_TOKEN` secret.

### AUR

1. Create an account at <https://aur.archlinux.org/register> and add your SSH public key
   under **My Account → SSH Public Key**.
2. Push the first version:

   ```bash
   git clone ssh://aur@aur.archlinux.org/opentokenmonitor-bin.git
   cp packaging/aur/PKGBUILD packaging/aur/.SRCINFO opentokenmonitor-bin/
   cd opentokenmonitor-bin && git add -A && git commit -m "Initial import: 0.4.1" && git push
   ```

3. Save a private key that the AUR accepts as the `AUR_SSH_PRIVATE_KEY` secret
   (a dedicated deploy key is safer than your personal one — add its public half to
   your AUR account too). AUR commits are authored with your GitHub no-reply address.

## Notes

- **winget** detects installs through the per-user uninstall key `OpenTokenMonitor`
  (publisher `opentokenmonitor`), written by the NSIS installer.
- **AUR** repackages the release `.deb`. `libayatana-appindicator` is a real dependency
  even though `namcap` can't see it: the tray icon loads it at runtime.
- When editing the PKGBUILD by hand, regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`.
