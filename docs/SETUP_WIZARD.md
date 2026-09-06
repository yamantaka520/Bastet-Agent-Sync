# 🐈 Google Drive setup wizard

The five-language desktop offers **Step-by-step wizard** and **Manual setup**. Both use the same native validation and saved state. Finishing setup does not start synchronization. Select sources, save and explicitly press Start to run the supported adapters. Language changes save independently during sync in 0.4.2.

## Guided steps

| Step | Action | Recorded only after |
| --- | --- | --- |
| 1. Login configuration | Use the distributor's configured Desktop OAuth client, or import your own downloaded Desktop OAuth JSON | Valid client metadata is available; imported credentials have been stored in the native credential store |
| 2. Google authorization | Open the system browser and grant app-file access | OAuth succeeds and the accessible-folder request succeeds |
| 3. Sync folder | Select an accessible folder, create one, or enter a Google Drive folder ID/link | Drive validates the folder; a created folder has a durable allocated ID |
| 4. Encryption and recovery | Create a space key, save the recovery kit, then verify the encrypted space; alternatively import another computer's kit | The key is in the native store, the recovery file is saved/read back, and the remote encrypted key proof is verified |
| 5. Review and finish | Review folder/space details and run the final check | Folder access and the stored key proof are checked again |

An unconfigured development build remains at step 1, with optional instructions to create a Google Cloud project, enable Drive API, configure consent/test users and download a **Desktop app** client JSON. No product client or test account is assumed. The built-in route avoids that developer setup when a distributor supplies a client. Import accepts an `installed` client document; web-client documents and arbitrary authorization endpoint overrides are not accepted. Secrets do not pass through the webview.

## Stop and resume

Completed steps, current page and guided/manual mode are saved after each successful action. On reopening, the app restores that record. **Continue where you left off** returns to the first incomplete step when viewing an earlier page. Returning to an earlier step does not undo completed steps. Unsubmitted folder text is not marked complete or persisted; press its verify/create action to save it.

Progress is separate from current network state. After an app restart the UI says the Google connection has not been checked in this session; previously completed steps stay recorded. Explicit reconnect can open the browser when needed. Other explicit remote actions may refresh the saved token but do not silently open a login flow. A failed/cancelled action preserves earlier completed steps. Cancelling a file chooser does not advance the encryption step.

`wizard.json` contains only setup metadata, never refresh tokens, client secrets or recovery keys. Atomic replacement and an OS file lock protect the record. Pending folder creation is kept under the setup session's operation directory. A prepared key/proof ID is saved before proof upload, so interruption and uncertain responses reuse the same key and ID. The separate encrypted snapshot queue retains its own upload journal.

## Restart

**Start again** displays the effect and requires an in-app confirmation. It archives the previous setup record under `wizard-history` and creates a fresh setup session. Existing Drive files, local keys, credentials and permissions are preserved. A malformed bounded JSON record can also be archived by explicit restart; it is never silently reset during loading. Old pending-operation records remain available on disk, not merged into the fresh session. Restart does not revoke Google access. The authorization section also offers **Forget local login**, which deletes only the saved refresh token while retaining setup progress and space keys.

Changing OAuth client identity archives the previous setup and resets downstream steps. Once a space key is prepared or imported, changing its folder/key requires restart; this prevents accidentally attaching an existing key proof to a different destination.

## Manual setup

**Manual setup** expands all five settings sections without forcing page-by-page navigation. You can import a custom client, reconnect, enter a folder ID or full Google folder link, select/create a folder, prepare/export a key or import an existing recovery kit, and run the final check from that page. It shares the guided state, so switching modes preserves completed work. Required authorization, folder checks and proof verification still apply. Manual means direct access to settings, not bypassing validation.

A folder link does not grant access. With `drive.file`, only app-accessible files are available. Google Picker for granting an arbitrary existing shared folder remains future work; inaccessible IDs fail visibly rather than broadening the OAuth scope.

## Recovery kit

The native save dialog writes `bastet-recovery.json` (or a user-chosen new filename) containing the space/folder/proof IDs and full recovery key. Keep it outside the shared Drive folder. It is a sensitive plaintext recovery artifact intentionally selected by the user, not an ordinary configuration backup. Existing different files are not overwritten. The wizard reads the saved file back before recording completion.

On another computer, configure authorization, select the same accessible folder, then choose **Import recovery kit**. The backend verifies folder identity and remote key proof before storing the imported key. A wrong key does not replace a working local key. The kit and imported client JSON are read only by Rust through native file dialogs. They are not added to project docs, logs or renderer state.

## Evidence and remaining gates

Fixture tests cover per-step reload, mode persistence, malformed-record restart, cancelled export, failure/retry, stable key/proof IDs, joining a space, rejected wrong recovery keys and frontend completion gates. These are not Google consent or physical-device tests. Native UI smoke and CI results are recorded in [Validation](VALIDATION.md).

Real-account macOS consent and selected transfers have been exercised; three-platform interactive credential-store checks and physical two-computer recovery remain incomplete. Google Picker remains pending. Background scheduling and additive local conversation restoration are implemented within the [current adapter scope](NATIVE_SESSIONS.md). The wizard's final check verifies setup; it does not transfer Agent data.

Primary setup reference: [Google Drive desktop OAuth setup](https://developers.google.com/workspace/drive/api/quickstart/python). Encryption and API boundaries: [Cloud security contract](CLOUD_SECURITY.md).

## Account identity and reconnect

The wizard reads `about.get` user permission ID, display name and email after authorization or token refresh using the existing `drive.file` scope. Identity is bound to the local wizard; reconnecting with a different permission ID stops before folder listing or mutation. Name/email changes do not change identity. The UI distinguishes saved identity from a connected session; expiry/revocation can still be discovered by the next request. Account metadata is stored locally with wizard progress and archives, never in recovery kits or cloud bundles.

To recover an account mismatch, remove the local login and reconnect with the original account. To intentionally switch accounts, restart setup; old progress and keys remain preserved. Old progress without identity still loads; use explicit Connect to bind the account after verifying any selected folder. Automatic refresh cannot silently adopt an unbound account. This migration verifies current access, not the historical identity of an older setup.

Official contracts: [about.get and its scopes](https://developers.google.com/workspace/drive/api/reference/rest/v3/about/get), [Drive user identity](https://developers.google.com/workspace/drive/api/reference/rest/v3/User). Google Picker grants remain planned. Pasting a folder URL does not grant access.

## Cancel and retry browser authorization

While Connect is running, **Cancel browser authorization wait** requests cancellation of the browser callback phase only. Accepted cancellation closes that listener before code exchange, preserves saved wizard progress and credentials, and enables another explicit Connect with fresh OAuth state and PKCE. Closing the browser alone is not detectable; the user can cancel in the app, or the wait times out after three minutes. The browser tab is not closed automatically.

Refresh, token exchange and account/folder checks are bounded network operations and are not aborted by this button. Outside the callback phase the command returns false and the UI explains that cancellation is unavailable; it never reports success for an already-finished wait. A separate in-memory cancellation channel remains callable while the setup transaction is locked. Completion and cancellation serialize before consuming a received code. Callback sockets explicitly use blocking mode with short read timeouts and a two-second total header-read deadline, so partial requests cannot indefinitely stall cancellation.

## Imported client credential recovery

Import persists the OAuth JSON through the native credential store and verifies its readable application representation before completing step 1. Since 0.4.1 a successful write populates the process cache, so this readback can be served from memory; Prepare credential access explicitly clears the cache and rereads native items. A first explicit authorization opens browser consent directly; saved-token refresh is attempted only when resuming a previously authorized setup. Credential-store failures identify whether the imported client or saved login could not be read; browser launch errors have a separate message. After replacing a development build, quit the old process and open the new build before testing credential access. Do not rebuild its running app bundle during a credential test. The observed initial failure was before browser launch; its underlying OS keychain error was not captured, so a code-signature cause is a hypothesis, not a proven diagnosis.

## Credential preparation (0.4.1)

After selecting a client, **Prepare credential access** can read the credentials saved so far before syncing. OS prompts may occur for separate entries; preparation does not finish missing wizard steps, verify Google access or start the worker. A failure stays visible and can be retried. See [credential cache lifecycle](CLOUD_SECURITY.md#credential-access-in-041).

Current 0.5.0 scheduling, audit and optional portable-package controls are documented in [SYNC_CONTROL.md](SYNC_CONTROL.md). The underlying contract above remains applicable.
