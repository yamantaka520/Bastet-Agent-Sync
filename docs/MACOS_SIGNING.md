# macOS Developer ID signing and notarization

macOS distribution uses Developer ID Application signing and Apple notarization. The standalone `macos-signing.yml` workflow tests Apple Silicon and Intel packages independently of Windows. It uploads verified test artifacts and does not publish a release. The full release workflow reuses the same macOS job. Version 0.6.0 uses the explicit deferred Windows publisher-signing route; required signing remains the workflow default until an eligible provider is available.

## Credentials

Configure repository secrets `APPLE_CERTIFICATE` (base64-encoded password-protected PKCS#12 with certificate and private key), `APPLE_CERTIFICATE_PASSWORD`, and `APPLE_API_PRIVATE_KEY` (App Store Connect team API key). Keep the existing `TAURI_SIGNING_PRIVATE_KEY`; updater signatures are separate from Apple signatures.

Configure repository variables `APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_API_KEY` (key ID), and `APPLE_API_ISSUER`. Never commit private keys, passwords, personal certificate subjects or account-specific identifiers. Secrets are available only to the main-branch signing workflow. Importing the PKCS#12 must succeed in macOS Keychain; some OpenSSL default PKCS#12 encryption settings are incompatible with Keychain import.

## Verification sequence

1. Import the certificate into a temporary runner keychain, write the notarization key with restricted permissions, and generate a temporary Tauri configuration overriding the shared ad-hoc identity with Developer ID and hardened runtime.
2. Authenticate to the Apple notarization service and sign a small executable before expensive compilation.
3. Run application tests and build each target. Tauri signs and notarizes the app before generating its updater archive and signature.
4. Require Developer ID authority, expected team, timestamp, hardened runtime, expected architecture, valid sealed resources, stapled ticket and Gatekeeper acceptance on the app.
5. Submit the signed DMG separately for notarization, require Accepted, staple and validate its ticket, assess it with Gatekeeper, and verify the app mounted from it.
6. Extract the updater archive and apply the same app checks. Upload only after the checks pass. Always remove temporary keychain and API-key material.

No local trust bypass, quarantine removal or ad-hoc fallback establishes acceptance. A successful authentication check alone is not notarization. Older release assets are not overwritten. Published 0.6.0 results are recorded in [validation](VALIDATION.md).

Sources: [Apple Developer ID](https://developer.apple.com/help/account/certificates/create-developer-id-certificates), [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/).
