# Veyron for macOS

`Veyron.app` is a native macOS window around the Veyron console (`/console`), packaged as a signed,
notarized DMG by the script below. The DMG is **not** attached to the GitHub releases (v0.4.0 and v0.4.1 carry only the
four CLI binaries, `SHA256SUMS`, its cosign bundle (v0.4.1) and an SBOM (v0.4.1)); build it with `scripts/package-macos-app.sh` (see below). It adds what a browser tab lacks: its own Dock icon and window, a menu bar status item,
real file pickers for image/ISO uploads, downloads straight to disk, native `prompt()`/`confirm()` dialogs,
fullscreen VNC/serial consoles, and certificate pinning for Veyron's self-signed TLS.

Source: `macos/Veyron` (Swift package, macOS 13+, universal arm64 + x86_64). It has no Xcode project.

## Using it

1. Open the DMG, drag **Veyron** to Applications.
2. First launch asks for your server, e.g. `192.0.2.10:30151` or `veyron.example.com`. HTTPS is required
   (plain `http` only for `localhost`).
3. A fresh install uses a self-signed certificate. The first connection shows its SHA-256 fingerprint; confirm
   it matches your server and the app pins it (trust on first use). If the certificate later changes you get
   a loud warning instead of a silent connection. **Settings** (⌘,) shows the pinned fingerprint and can forget it.
4. Sign in to the console as usual. The login lives in the app's own web data, not in your browser.

Menus: View → Reload (⌘R), Back/Forward (⌘[ ⌘]), Zoom (⌘= ⌘- ⌘0). Server → Open in Browser (⇧⌘O),
Change Server. The menu bar item shows whether the server's `/api/v1/health` answers (checked every 30 s,
never prompting for trust).

Links to other sites open in your browser; the window only ever shows the Veyron console.

## Building

```bash
./scripts/package-macos-app.sh build      # universal Veyron.app, ad-hoc signed, in dist/macos (local testing)
cd macos/Veyron && swift test             # VeyronCore: address parsing, navigation policy, certificate trust
```

`VeyronCore` holds the logic that can be tested without a GUI (server address normalization, which links stay in
the app, trust-on-first-use decisions); `Veyron` is the SwiftUI/WebKit shell.

## Signing and notarizing

```bash
./scripts/package-macos-app.sh release    # ships sources to the Mac mini, then signs, notarizes and packages there
```

This runs on the signing Mac over `ssh mini` (override with `VEYRON_SIGN_HOST`):

1. universal release build and `swift test`,
2. `codesign` with the Developer ID Application identity, hardened runtime, secure timestamp,
3. notarize the app with `xcrun notarytool submit --keychain-profile Macora-Notary --wait`, then `stapler staple`,
4. build `Veyron-<version>.dmg` (with an Applications link), sign, notarize and staple it,
5. `spctl` and `stapler validate` checks, then fetch `dist/macos/Veyron-<version>.dmg` and its `.sha256`.

Settings: `VEYRON_SIGN_IDENTITY`, `VEYRON_NOTARY_PROFILE`. The notary API key stays in the signing Mac's
keychain profile; the script never reads or copies it. The version comes from `Cargo.toml`; the build number is
the git commit count. A rejected notarization prints Apple's log.

To check a downloaded DMG: `spctl -a -vvv -t open --context context:primary-signature Veyron-<version>.dmg`
should print `source=Notarized Developer ID`.
