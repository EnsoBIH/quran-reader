# Releases

`.github/workflows/release.yml` builds the desktop app with
[`tauri-action`](https://github.com/tauri-apps/tauri-action) on three runners and uploads
it to a **draft** GitHub release:

| Runner | Bundles | Signed |
| --- | --- | --- |
| `macos-15` | `.dmg` and `.app.tar.gz` (updater), universal (Apple silicon + Intel) | Developer ID, notarized |
| `windows-2025` | NSIS `-setup.exe` (per user, no admin) and `.msi` | updater signature only (no Authenticode certificate yet) |
| `ubuntu-22.04` | `.AppImage`, `.deb`, `.rpm` | updater signature only |

Every bundle carries a signature of the project's updater key (`.sig`), and `latest.json`
lists them for the updater (DECISIONS.md D-027). The last step of each build fails if an
installer is 15 MB or larger (`apps/desktop/scripts/bundle-size.mjs`, D-026) and lists all
sizes in the job summary.

Pull requests that touch `apps/desktop/src-tauri/`, `Cargo.lock` or the workflow run the
same build unsigned and keep the bundles as workflow artifacts; so does a manual run
(Actions → Release → Run workflow).

## One-time setup

### 1. Updater key

The updater only installs an update signed with the key whose public half is in
`apps/desktop/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). It is empty until the
maintainer creates the key; the release job refuses to run without it.

```bash
pnpm --filter @quran-reader/desktop tauri signer generate -w ~/.tauri/quran-reader.key
```

- Put the **public** key (the contents of `~/.tauri/quran-reader.key.pub`) into
  `plugins.updater.pubkey` and commit it.
- Add the **private** key and its password as repository secrets
  `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Keep a backup of the private key offline. If it is lost, installed apps can never be
  updated again: users would have to download a build with a new key by hand.

### 2. Apple signing and notarization

Needs an Apple Developer Program membership. Repository secrets:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | the "Developer ID Application" certificate with its private key, exported as `.p12`, base64-encoded (`base64 -i cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | the password of the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | the Apple ID used for notarization |
| `APPLE_PASSWORD` | an [app-specific password](https://support.apple.com/102654) of that Apple ID |
| `APPLE_TEAM_ID` | the 10-character team ID |

The bundler imports the certificate into a temporary keychain, signs the app with the
hardened runtime and waits for notarization.

### 3. GitHub Pages

The landing page (`.github/workflows/site.yml`) is pushed to the `gh-pages` branch. Once,
in Settings → Pages: *Source: Deploy from a branch*, branch `gh-pages`, folder `/ (root)`.
The site is then at <https://ensobih.github.io/quran-reader/>, and each pull request that
changes the site gets a preview at `…/quran-reader/pr-preview/pr-<number>/` with a link in
a comment; it is removed when the pull request is closed. Settings → Actions → General →
Workflow permissions must allow *Read and write*.

## Cutting a release

1. Content review signed off for this version (`docs/content-review.md`).
2. Bump `version` in the root `Cargo.toml` (the app, `quran-core` and `xtask` share it;
   `tauri.conf.json` takes it from there), commit, merge to main.
3. Tag and push: `git tag v1.0.0 && git push origin v1.0.0`. The tag has to match the
   version, or the workflow stops.
4. When the workflow is green, check the draft release: every platform's files, their
   sizes, `latest.json`. Install the `.dmg` on a Mac that has never run the app
   (Gatekeeper must not complain) and one Windows and Linux build.
5. Edit the notes, then **Publish**. Only then does
   `releases/latest/download/latest.json` point at it, and installed apps offer the update.

## Screenshot baselines

The Arabic rendering tests (`apps/desktop/e2e`, D-028) compare against PNGs committed in
`apps/desktop/e2e/screenshots/<engine>/`. After an intended change of the rendering, a new
runner image or a Playwright update, regenerate them:

- Set the repository variable `E2E_UPDATE_SNAPSHOTS` to `all` (Settings → Variables),
  re-run the CI job "Arabic rendering", download the `screenshots-<engine>` artifacts, look
  at every changed image, commit them, and delete the variable again.
- Locally, for the engine of your OS: `pnpm --filter @quran-reader/desktop test:e2e
  --update-snapshots`. Only commit images from the CI runners: fonts and engine builds on
  your machine differ.
