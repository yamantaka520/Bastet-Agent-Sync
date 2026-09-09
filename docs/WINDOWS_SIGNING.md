# Windows publisher signing

Microsoft Artifact Signing provides Authenticode signatures for future Windows packages. This is separate from Tauri's updater signatures. Existing published releases are immutable; enabling this workflow does not sign or replace old downloads.

## Repository configuration

Set these GitHub Actions repository variables: `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `ARTIFACT_SIGNING_ENDPOINT`, `ARTIFACT_SIGNING_ACCOUNT`, and `ARTIFACT_SIGNING_PROFILE`. The existing `TAURI_SIGNING_PRIVATE_KEY` secret remains the updater signing key. Do not commit tenant-specific values or a client secret.

The Microsoft Entra application's federated credential must match the repository's `main` branch. Assign its service principal the **Artifact Signing Certificate Profile Signer** role at the intended certificate profile or signing account. The profile must provide Public Trust signatures and have completed identity validation. Creating an account alone does not establish signing eligibility.

Match the exact issuer, subject and audience printed by the Azure login step. GitHub's subject can include immutable owner/repository IDs; the Azure GitHub form may generate a name-only subject that does not match. In that case use an **Other issuer** federated credential with the exact observed subject. Do not remove the immutable IDs or widen the branch restriction to work around the mismatch. Azure error `AADSTS700213` indicates this trust mismatch before any signing request occurs.

## Read-only management inspection

The optional `azure-signing-inspect.yml` workflow reads the account and profile trust types through the same main-branch OIDC credential. Configure `ARTIFACT_SIGNING_RESOURCE_ID` as a repository variable; keep its value out of source documents. Account-scoped Reader permits inspection; Contributor permits profile management and does not replace the separate signing role. The inspection only issues GET requests and projects status/type fields, without exporting identity documents or certificates.

Public Trust requires a qualifying public identity validation. Private identity validation cannot substitute for it. Check Microsoft's current [eligibility and identity-validation instructions](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart) before creating another profile; Azure hosting region does not establish publisher geographic eligibility. Identity validation is completed in the Azure portal.

## Execution and verification

The main-branch-only Windows workflow requests `id-token: write`; other build jobs do not. It authenticates with Azure CLI through GitHub OIDC, installs ArtifactSigning 0.1.8 from PowerShell Gallery, and generates a temporary Tauri configuration with an absolute signing callback path. Cloud keys stay in the signing service.

Before compilation, a small generated DLL probes the same signing callback and Windows trust validation. A successful service-side signing response alone is insufficient; an untrusted root or missing timestamp blocks the job. Public Trust Test and Private Trust profiles do not establish public distribution trust.

Tauri invokes the callback during packaging, so application and installer signing occurs before the updater artifacts are signed. The callback uses SHA-256 and RFC 3161 timestamps and rejects an invalid or untimestamped result. Signing is mandatory: no unsigned fallback is uploaded.

Before upload, the workflow verifies both installers, installs NSIS into an isolated runner folder, verifies the installed application and uninstaller, launches the application, and administratively extracts MSI to verify its application. It also checks that unsigned input fails verification. The release workflow waits for this Windows job and the other architecture builds before creating a draft. Standalone dispatch builds test artifacts only and does not publish a release.

Trusted signatures establish file integrity and publisher identity; SmartScreen reputation and application behavior are separate. See [dated evidence](VALIDATION.md) for actual results, rather than treating configured settings as proof of successful signing.

Sources: [GitHub immutable subjects](https://github.blog/changelog/2026-04-23-immutable-subject-claims-for-github-actions-oidc-tokens/), [Microsoft OIDC integration](https://github.com/Azure/artifact-signing-action/blob/main/docs/OIDC.md), [Microsoft signing action](https://github.com/Azure/artifact-signing-action), [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).
