# Mac App Store delivery plan

Status (2026-10-02): App Store preparation remains in progress; no build uploaded, TestFlight processing, App Review submission or public listing. The App Store Connect draft has saved five-language version copy, keywords, support and marketing URLs, free pricing, all 175 eligible territories, 2026 Bastet AI copyright, private review contact, manual release and the user's non-trader DSA declaration. Apple's account page reports regulatory requirements completed. Existing v0.6.0 remains the independently distributed Developer ID release.

## Account and submission gates

The Bundle ID and macOS App Store Connect record have been created through the authenticated Apple website. Traditional Chinese is the primary language and Developer Tools is the primary category. Free pricing was saved; release is manual. Two separate Store distribution certificates and a matching Mac App Store Connect provisioning profile were created and verified locally. Existing Developer ID credentials remain separate. Private account identifiers, keys and provisioning files stay out of this repository. The API key can read account resources but its identifier-creation request returned HTTP 403; website registration subsequently succeeded.

The five localized names and subtitles are saved as **Bastet AI Sync by Bastet AI**. App Store Connect returned duplicate-name errors while partially saving localization batches; each remaining locale was saved from a fresh form and checked separately. All five version descriptions and keywords, plus support and marketing URLs, are saved in the account draft. Their content and remaining gates are documented in [store copy](app-store/LISTING.md).
1. Bundle ID, App Store Connect record, product name, SKU, primary language and five localized names/subtitles are saved.
2. Separate application/installer distribution identities and the matching provisioning profile are created and locally verified. Keep these separate from Developer ID and updater-signing credentials; do not revoke existing certificates.
3. Complete and test the Store application changes below before creating an upload candidate. A template entitlement or successful compilation alone is not acceptance.
4. Produce a signed universal macOS `.app`, embed the matching provisioning profile, package a signed `.pkg`, validate it, and upload through supported Apple tooling. Record the actual App Store Connect build/version IDs and processing result. Never use a notarized DMG as the Store submission artifact.
5. Complete free pricing, territories, age rating, copyright, privacy/support URLs, privacy questionnaire, encryption assessment, review contact and review access. Business/legal answers must describe the actual final Store build; unresolved values are not silently defaulted.
6. Test the processed build with TestFlight where available, then submit the selected build and metadata to App Review. Release only after Apple approval; store submission and public availability are distinct states.

## Engineering gates, in order

| Gate | Required implementation and acceptance | Current evidence |
| --- | --- | --- |
| Store update channel | Compile out the direct updater plugin; reject direct updater commands; five-language UI refers to App Store updates. Default direct-download channel retains its signed updater. | Implemented behind `mac-app-store`: updater plugin and mutation commands are omitted; Store UI has no direct install/check controls. |
| Sandbox filesystem | Explicitly select agent/project/restore folders; persist security-scoped bookmarks, resolve/reauthorize stale grants and release scopes correctly. Test restart, moved folders, revoked access, symlinks and all read/write consumers. | Picker bookmarks, scoped discovery, save/preflight/worker access, stale-grant failures and reauthorization UI are implemented. An ad-hoc sandbox copy retained a picker-granted synthetic folder across a confirmed process restart; production-profile, move/revoke and provider acceptance remain pending. |
| Provider functionality | Test actual permitted captures and managed restores in the signed sandbox. The first Store release excludes AMOS sync by explicit product decision. Direct builds retain it. Reject AMOS commands in the backend and omit its Store selection controls. | Store excludes AMOS IPC, selection, CLI export/apply and worker exchange; fixture tests verify fail-closed guards. Other providers still need signed-sandbox acceptance. |
| Network and credentials | Minimum necessary network client/server rights for Google API and loopback OAuth; verify login/cancel/reconnect and Keychain access in the signed sandbox. | A dedicated desktop client passed real consent/callback, saved credential readback after App restart and authorized Drive access in an ad-hoc App Sandbox copy. Production-profile/TestFlight, cancellation and expired-token refresh remain separate gates. |
| Handoff and round trip | Verify terminal instructions, external agent access to prepared profiles and return capture under granted paths. | Managed handoff works in direct builds, but sandbox paths and external CLI access need separate testing. |
| Data and account isolation | Verify sandbox app-data location, recovery-kit access and explicit migration. Never overwrite active agent stores or assume direct/Store state is shared. | Planned. |
| Presentation and review | Real screenshots from the final candidate in five languages, working review setup with synthetic conversations, public support/privacy pages, truthful claims. | Five localized names, subtitles, descriptions and keywords are saved; support and marketing URLs are entered. Public product/privacy pages and five-language policy URLs are now saved. Screenshots and review demonstration remain pending. |

No temporary sandbox exception or unrestricted helper is assumed acceptable. If a supported Store design cannot retain a core feature, document the concrete reduced scope and obtain a product decision before changing its public promise. Public v0.6.0 assets remain immutable.

## Store information still needed

- Saved price, territory, copyright, private App Review contact, manual release and user's non-trader DSA declaration are account-draft state only. Apple's account page reports all regulatory requirements completed; this is not evidence of build approval or public availability.
- The public [privacy policy](https://bastet.tw/agent-sync/privacy/) and [product/support page](https://bastet.tw/agent-sync/) are live, and policy/control URLs are saved in all five Store locales. Final privacy-label publication, encryption and content-rights answers, real screenshots and reviewer-access instructions remain open. The app encrypts payloads with XChaCha20-Poly1305 in addition to HTTPS, so do not copy an arbitrary `ITSAppUsesNonExemptEncryption=false` example.
- Google authorization usable by reviewers and intended users, with the required configured client/audience. Existing operational OAuth access does not prove a public-review onboarding flow.

## References

- [Apple App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/) — §2.4.5 sandbox, package and update requirements.
- [Apple App Sandbox](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox) and [file access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox).
- [Apple provisioning profiles](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile).
- [App Store Connect app resources](https://developer.apple.com/documentation/appstoreconnectapi/apps) — create the app record on the website.
- [Tauri App Store guide](https://v2.tauri.app/distribute/app-store/) — Store-specific configuration, signing, provisioning and packaging.
- [Existing macOS direct distribution](MACOS_SIGNING.md), [current validation](VALIDATION.md), [native data boundaries](NATIVE_SESSIONS.md).

## Confirmed Store scope (2026-10-02)

The user selected free download and explicitly chose to omit Agent Memory OS synchronization from the first Mac App Store release. Direct updater and AMOS plugin commands are excluded from the opt-in Store feature; the direct-download default channel retains its updater and AMOS behavior. Store UI copy is localized in all five languages. Other providers still require signed-sandbox acceptance; this scope decision does not certify them. No changes to the external Agent Memory OS project are required for this Store milestone.

## Reproducible local candidate

Use the Apple application and installer distribution identities already installed in an unlocked keychain. Do not commit certificates, keys, profiles or account identifiers. `scripts/mac-app-store-build.sh` requires:

- `BASTET_STORE_PROFILE`: a Mac App Store Connect provisioning profile for the configured bundle identifier.
- `BASTET_STORE_APP_IDENTITY` and `BASTET_STORE_INSTALLER_IDENTITY`: exact Store signing identity names.
- Optional `BASTET_STORE_KEYCHAIN`: a separate keychain; its temporary search-list change is restored on exit.
- Optional `BASTET_STORE_OUTPUT_DIR`: a private build directory. Prefer a temporary, nonsynchronized directory on hosts where a file provider adds Finder metadata to application bundles.

The script validates the profile, generates private local signing configuration, builds both Mac architectures with `mac-app-store`, checks the embedded profile and signed entitlements, and creates a signed `.pkg`. It neither uploads nor notarizes. Its isolated Cargo target prevents Store binaries from replacing direct-release outputs. Both Rust target standard libraries and a selected Xcode installation must be available. Do not move an existing Cargo target without rebuilding generated plugin-permission paths.

The Store configuration currently uses marketing version `0.6.0` and build `1`; increase the build number for subsequent uploads. App Store Connect's initially generated `1.0` draft was changed to `0.6.0`. The original public v0.6.0 release assets remain unchanged.

If the native `security cms` decoder cannot load the local certificate chain, preparation can verify the CMS signature with OpenSSL. That fallback does not independently establish Apple's signer trust; the packaging check additionally requires the selected trusted signing identity to match a certificate embedded in the profile.

## Apple validation and disclosure preparation — 2026-10-02

Apple’s server-side package validation returned **VERIFY SUCCEEDED with no errors** for the rebuilt universal `0.6.0` build `1` candidate. The first validation had rejected private filesystem modes; the build now normalizes directory/file/executable permissions and re-signs the final app before packaging. The signed app includes `Contents/Resources/PrivacyInfo.xcprivacy`, verified against the source manifest. Three focused packaging tests pass. Validation is not upload, processing, TestFlight acceptance or App Review approval.

The age questionnaire is saved with Apple’s calculated global 4+ rating (regional ratings vary). Two conservative privacy categories are saved as an unpublished draft: Other User Content and Other Data Types, linked to the user for App Functionality, with no tracking. See the [source assessment](app-store/PRIVACY_ASSESSMENT.md). The policy URL has since been saved in all five locales; final privacy-label publication remains pending.

The product decision is to provide Bastet-managed Google sign-in. A dedicated project, Drive API and desktop OAuth client are configured after explicit owner authorization, separate from the prior shared application project. Only `drive.file` is configured. The rebuilt universal Store candidate embeds this client and passes Apple server-side package validation. Real consent/callback and saved-credential reconnection after restart pass in an isolated ad-hoc sandbox. The owner authorized switching the audience to External/Production after publishing the product/privacy pages and verifying website ownership. Google reports no data-access verification is needed for the sole non-sensitive scope. Brand verification has not passed: automatic checks conflict with anonymous-page evidence, and an additional-review draft is prepared. Fresh public/reviewer onboarding and production-profile acceptance remain separate gates.

An exploratory, unsaved Apple encryption questionnaire selecting non-standard plus non-OS standard encryption and the configured France territory requested both a BIS CCATS approval and a French encryption declaration. This records the website response, not a legal classification. XChaCha20’s IETF document is an expired draft; do not treat ChaCha20’s RFC as automatically establishing XChaCha20’s status. Resolve the applicable classification and documents before completing the declaration. Do not silently remove territories or alter the existing encrypted data format.

## Remaining acceptance and submission work

- Production-profile sandbox acceptance: picker grants across restart, moved/revoked folders, OAuth loopback, Store Keychain service, real provider capture/restore. A separate ad-hoc sandbox test copy launched, passed an isolated core UI check and retained a picker-granted synthetic folder across a confirmed process restart; this does not establish production-profile, Store-keychain or Google Drive acceptance. Same-path handoff reauthorization is implemented, but native restart/revoke/provider acceptance remains pending.
- External handoff parents moved to a new path are not automatically remapped. Failed grants retain the registry and report partial status without stopping healthy handoffs; same-path reauthorization has a Store UI action.
- Confirm final privacy-manifest/API reasons and App Store privacy disclosures against the final binary. A [privacy policy draft](app-store/PRIVACY_DRAFT.md) is available for review but is not a published policy endpoint. Complete encryption and age/content answers, reviewer setup and real screenshots. Google OAuth configuration must permit public/reviewer use. The saved review contact is private and is intentionally not reproduced in this repository.
- Upload only after local acceptance, verify Apple's processing result, then submit to App Review. No uploaded build, processed TestFlight version or review approval is claimed by this preparation milestone.
