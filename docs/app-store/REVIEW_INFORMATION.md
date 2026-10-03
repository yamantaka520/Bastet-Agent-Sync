# Apple App Review information

Submitted on 2026-10-03 in both the review response and App Review Notes. The named video and sample ZIP are attached to the response. Notes additionally point to that response. Apple confirmed WAITING_FOR_REVIEW after that resubmission; the subsequent October 4 entitlement clarification is recorded below.

1. Recording: Bastet-0.6.0-build1-Review.mp4 was captured on a physical MacBook Pro (Apple M2 Max), macOS 27.0.1 (26A434), October 3, 2026, using TestFlight 0.6.0 (1). Capture began before launch and demonstrates source-folder permission, project mapping, Google authorization, Drive/encryption setup, explicit Start sync, one synthetic bundle upload, pause and restoration to a new profile. Privacy edits crop unrelated desktop edges, cover chat screens and personal identifiers, remove audio and trim idle footage at the beginning/end. App operations remain in their original order and speed. This is one-Mac upload/local-snapshot restoration; it does not demonstrate second-device reception or model continuation.

2. Purpose/audience: Bastet AI Sync is a desktop utility for people using local AI coding agents across computers. It synchronizes selected local agent data through an encrypted space in the user's Google Drive, preserves separate versions/branches and prepares supported conversations in independent local profiles. It does not provide a chatbot or model inference service.

3. Access/setup: No Bastet account, registration/deletion flow, subscription, paid feature or in-app purchase exists. Cloud sync requires the reviewer's own Google account and browser authorization; no Google password is entered into Bastet. Extract BastetReviewSample.zip outside any live agent profile. Through the Claude Code card's Choose path picker, select SampleClaudeConfig, select that source and save. Add the README's project mapping to the existing ReceivingProject folder using Choose local project. In Google Drive setup, use the built-in configuration, authorize Google, create a dedicated review folder, create a space key, save the recovery kit outside Drive and finish verification. Explicitly Start sync, then Pause sync, View conversation snapshots, expand Claude Code and Restore to a new folder under RestoredProfilesParent. The app creates a separate profile; never choose an active agent store as the restore target. The synthetic sample needs no Claude account or model call for this file workflow. Actual model continuation requires a compatible installed agent and its own account. Interface inspection does not require Google sign-in.

4. External dependencies: Google browser OAuth and Drive API use drive.file access for the chosen cloud destination. Credentials stay in the local OS credential store and selected payloads are encrypted before upload. Compatible agents are independently installed third-party tools; Bastet does not sign into their services or call their model APIs. The Mac App Store edition excludes Agent Memory OS sync and receives updates through the App Store. Successful Google authorization is not a claim of Google brand-verification approval.

5. Regions: The release is configured for 174 territories, excluding France. Features are the same across the selected territories, with Traditional/Simplified Chinese, English, Japanese and Korean interfaces. Google account, network and service availability can affect cloud access by location.

6. Content/business: This is a private file utility, not a public content platform or regulated medical/financial/gambling service. It has no public feed, posting, messaging or content catalogue. App-specific artwork and the supplied synthetic sample are original. User-selected files remain their owners' material; compatible-agent names describe interoperability, not affiliation. Bundled software retains its open-source notices.

Support: https://bastet.tw/agent-sync/
Privacy: https://bastet.tw/agent-sync/privacy/

# Evidence and submission record

- Exact TestFlight build 0.6.0 (1) launched on a physical MacBook Pro (Apple M2 Max), macOS 27.0.1 (26A434). User-operated recording began before launch and includes the sample-folder picker, project mapping, Google setup, encrypted space, explicit Start sync, pause and separate-profile restore.
- One synthetic bundle was uploaded. The separate restored profile preserved the one message and session ID, and mapped the project path to the selected receiving directory. Downloaded bundles remained zero; this is not second-device reception or model continuation.
- The final 7:26 delivery video was sampled to verify the launch, Google consent with personal identifier masked, setup and restore results. Unrelated chat screens have an explicit privacy card; audio and idle edges were removed. The original recording remains private.
- Both attachments were accepted in the sent App Review reply. The six answers were also saved in Notes; both fields are below 4,000 characters. Update Review Content and Resubmit to App Review completed.
- Current status is WAITING_FOR_REVIEW after resubmission; the earlier 2.1 information request remains in the review history. Release remains manual, 174 territories excluding France. Brand-verification approval is not inferred from successful Google authorization.

# Source pointers

REQUIREMENTS.md (purpose and setup); docs/MASTER_PLAN.md (scope and limits); docs/APP_STORE.md (submitted build, Store-specific scope, OAuth/review status); docs/VALIDATION.md (isolated sandbox evidence); docs/app-store/LISTING.md (five-language listing); src/App.tsx (source folder picker); src/CloudPanel.tsx (Google wizard); src/NativeSessions.tsx (received versions and restore).

## Guideline 2.4.5 — entitlement clarification, 2026-10-04

A technical explanation was sent through App Review and the following item was appended to the existing Notes, preserving all six earlier answers and attachment references. The saved Notes contain 3,945 characters, within the 4,000-character limit.

7. Network server entitlement: Google desktop OAuth needs a temporary HTTP listener at 127.0.0.1:<random port>/oauth/callback for the browser response. It uses state/PKCE, waits up to 180s, and closes when authorization ends. No LAN/public listener.

The full technical rationale and source pointers are in [the Store delivery record](../APP_STORE.md#network-server-entitlement-clarification--2026-10-04). The unchanged `0.6.0 (1)` was resubmitted at 02:38 Asia/Taipei; the authenticated review page confirms WAITING_FOR_REVIEW. This is not approval or Store publication.
