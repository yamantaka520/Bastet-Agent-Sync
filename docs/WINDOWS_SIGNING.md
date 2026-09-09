# Windows publisher signing

Microsoft Artifact Signing provides Authenticode signatures for future Windows packages. This is separate from Tauri's updater signatures. Existing published releases are immutable; enabling this workflow does not sign or replace old downloads.

## Repository configuration

Set these GitHub Actions repository variables: `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `ARTIFACT_SIGNING_ENDPOINT`, `ARTIFACT_SIGNING_ACCOUNT`, and `ARTIFACT_SIGNING_PROFILE`. The existing `TAURI_SIGNING_PRIVATE_KEY` secret remains the updater signing key. Do not commit tenant-specific values or a client secret.

The Microsoft Entra application's federated credential must match the repository's `main` branch. Assign its service principal the **Artifact Signing Certificate Profile Signer** role at the intended certificate profile or signing account. The profile must provide Public Trust signatures and have completed identity validation. Creating an account alone does not establish signing eligibility.

## Execution and verification

The main-branch-only Windows workflow requests `id-token: write`; other build jobs do not. It authenticates with Azure CLI through GitHub OIDC, installs ArtifactSigning 0.1.8 from PowerShell Gallery, and generates a temporary Tauri configuration with an absolute signing callback path. Cloud keys stay in the signing service.

Tauri invokes the callback during packaging, so application and installer signing occurs before the updater artifacts are signed. The callback uses SHA-256 and RFC 3161 timestamps and rejects an invalid or untimestamped result. Signing is mandatory: no unsigned fallback is uploaded.

Before upload, the workflow verifies both installers, installs NSIS into an isolated runner folder, verifies and launches the installed application, and administratively extracts MSI to verify its application. It also checks that unsigned input fails verification. The release workflow waits for this Windows job and the other architecture builds before creating a draft. Standalone dispatch builds test artifacts only and does not publish a release.

Trusted signatures establish file integrity and publisher identity; SmartScreen reputation and application behavior are separate. See [dated evidence](VALIDATION.md) for actual results, rather than treating configured settings as proof of successful signing.

Sources: [Microsoft OIDC integration](https://github.com/Azure/artifact-signing-action/blob/main/docs/OIDC.md), [Microsoft signing action](https://github.com/Azure/artifact-signing-action), [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).
