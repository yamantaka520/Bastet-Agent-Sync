# 🐈 Cloud transport and encryption — 0.4.2

OAuth, native credential storage, encrypted Drive exchange, the resumable wizard and an explicit Start/Pause worker are implemented. Selected supported local agent data is uploaded after Start. Real-account macOS setup and selected transfers have been exercised; full physical two-device acceptance remains open. See [evidence](VALIDATION.md) and [adapter scope](NATIVE_SESSIONS.md).

## Connection boundary

The distributor supplies a Google **Desktop app** OAuth client through `BASTET_GOOGLE_CLIENT_ID` and, when required by that client, `BASTET_GOOGLE_CLIENT_SECRET` at build time. Desktop client metadata is distributed with the binary and cannot authenticate the application itself. No product client is committed or configured in the default build. The built-in route does not require end users to create a Cloud project. An optional custom-client route imports a Desktop OAuth JSON through a native file dialog and stores its contents in the system credential store.

A first explicit Connect opens browser authorization; reconnecting a previously authorized setup tries its saved refresh token. When reauthorization is required, Rust opens the system browser with a fresh 256-bit state and PKCE S256 verifier. A short-lived listener binds only `127.0.0.1` on an OS-assigned port, validates the callback path and unique state/code parameters, and expires after three minutes. The webview cannot navigate arbitrary URLs or read tokens. This listener exists only during login, not as a background service. Callback unit tests and the RFC PKCE vector are verified; configured-client macOS consent and token exchange were exercised; other OS interactive acceptance remains separate.

Only `drive.file` is requested. Folder inventory contains files the application can access, not an enumeration of the entire account. Existing shared folders may require Google Picker authorization, which is not implemented. Creating a folder is an explicit action. A local durable journal retains the allocated Drive ID before creation; an uncertain response is retried with that ID and a conflict is reconciled by metadata. Changing the name while an operation is unresolved is refused. Completed records are retained; repeating the same name returns the recorded result, not a new folder. This journal is local metadata, not a cloud mutex.

The GUI now saves a verified folder/space binding through the [resumable setup wizard](SETUP_WIZARD.md). It supports new keys, recovery-kit export/import and a final proof check. Account identity display and mismatch protection are implemented; Picker and device listing remain pending. Folder inventory alone is not a successful setup or sync.

## Secrets and recovery

Refresh tokens use native Keychain on macOS, Credential Manager on Windows, and Secret Service on Linux via explicitly enabled keyring backends. An unavailable or locked store produces an error; no plaintext file fallback exists. Secret Service requires a running desktop service even with vendored build dependencies. Access tokens remain in Rust memory with an early expiry margin. Secrets are not serialized to the renderer, logs or settings. Forget local login deletes the local refresh token; it does not revoke Google's grant. Native credential-store round trips still require per-platform interactive verification; unit tests use an isolated in-memory implementation.

Each space uses a random 256-bit key. A `bas1_` recovery code encodes that full random key, not a human password or a password-derived key. Import rejects malformed codes. Existing keys are not silently overwritten. The wizard stores space keys in the native store and exports/imports recovery kits through native dialogs. Its separate sample diagnostic still creates and discards an ephemeral key without touching the user's keychain. A recovery code must ultimately be exchanged outside the shared Drive folder and never included in support reports. Without a retained key/recovery code, ciphertext is unrecoverable. Member revocation and key rotation are future work.

## Encrypted envelope v1

XChaCha20-Poly1305 from RustCrypto encrypts the entire validated M2 Bundle with a fresh random 24-byte nonce per encryption. Associated data binds a fixed protocol label/version and the space ID. The receiver checks envelope version, expected space, nonce length, authenticated decryption, inner snapshot space and M2 hashes/limits before returning a Bundle. Wrong keys, modified ciphertext and modified nonces fail closed.

Only envelope version, opaque space ID, nonce, ciphertext length and Drive metadata are visible to the cloud. Agent/profile/conversation names and logical content hashes are inside ciphertext. Random Drive object IDs name objects. Cloud operators can still observe sizes, counts and timing. Shared-key authentication does not identify individual authors: anyone holding the space key can forge another device's snapshot. This is not a signed per-device protocol or an independently audited product.

The encoded envelope is bounded to 128 MiB and decrypted wire data to 64 MiB. Plaintext temporary crypto buffers and recovery/token wrapper values use zeroizing containers where practical; this does not promise erasure of every allocator, OS or library copy. M2's local replica remains plaintext. No claim of whole-disk encryption or encrypted local cache is made.

## API behavior and remaining integration

Production HTTP clients use fixed Google HTTPS endpoints, no redirects, 10-second connection and 30-second total request limits, and bounded response bodies. Download verifies the expected parent/type before authenticated decryption. Listing handles pagination, rejects repeated page tokens and incomplete searches, and caps results. API errors are reduced to stable codes without returning provider response bodies.

Uploads use multipart/related and caller-provided preallocated IDs. No update/delete endpoint exists. The caller must persist and reuse the same ID after an uncertain upload; the one-shot encrypted exchange queue persists and reuses these IDs. HTTP fixture tests verify encrypted upload/download and selected errors against a loopback server, not Google. The worker schedules non-overlapping retries and reconnects when its token is no longer connected. The standalone HTTP backoff policy is separate; Retry-After integration and transparent mid-request refresh remain pending. 403 errors are conservatively surfaced.

The one-shot encrypted exchange queue connects validated M2 replicas through an `Objects` interface implemented by Drive. It requires an explicit folder/space/proof binding and verifies an encrypted key proof before any exchange. It inventories and decrypts remote objects under a total limit, preserves upload IDs in a locked local journal, and reconciles uncertain uploads even when listing is stale. Two isolated replicas preserve concurrent branches and repeated exchange adds no objects. Upload-only still reads remote objects for inventory but does not import them; download-only never publishes. Missing remote objects do not delete local history. The queue is invoked by the native worker using saved selected paths after explicit Start. The renderer cannot submit arbitrary bundles to it. Space setup, key recovery and start/pause orchestration are implemented. M2's local-folder transport still writes plaintext development snapshots and is not silently upgraded to encrypted cloud storage.

## Verification and release gates

1. Configure the product client, consent screen, Drive API and test accounts; run browser consent, cancel, expired/revoked-token and reconnect tests.
2. Verify native secret-store save/restart/recovery on all three OS targets.
3. Complete Picker/shared-folder grants and finish cross-platform real-account wizard acceptance.
4. Run a real two-device encrypted transfer, interrupted/ambiguous upload, quota delay and recovery from the exported key.
5. Complete physical two-device continuation acceptance for the implemented adapters; fixture and additive-import tests do not establish every native resume workflow.

## Primary references

- [Google native desktop OAuth and PKCE](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Drive scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)
- [Drive error handling](https://developers.google.com/workspace/drive/api/guides/handle-errors)
- [RustCrypto chacha20poly1305 0.10.1](https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/)
- [Keyring 3.6.3 native backend features](https://docs.rs/keyring/3.6.3/keyring/)

## Credential access in 0.4.1

The native credential store remains the persistent authority. Successful reads and writes are cached per account in process-local zeroizing memory. Reads, writes, removals and clearing are serialized; failures and missing entries are not cached. Failed mutations evict the affected cached value. Cache clearing overwrites owned secret buffers; no cache is serialized to disk or exposed to the renderer.

The setup panel offers **Prepare credential access**. It clears the prior cache/connection, then reads the configured client, the saved login (if authorized), and the space key (if bound). It performs no Google network request or agent synchronization. Partial failure clears the newly prepared cache. Normal syncing also populates the cache automatically. Forget login, restart setup, client replacement and normal process exit clear the cache; closing to tray keeps it available. If credentials are edited externally or the system keychain is subsequently locked, already cached values remain usable until cleared or the process exits. Press Prepare again to reread external changes.

macOS users can select Always Allow for each requested Bastet item. This is per local OS/app/item authorization, not a blanket grant across devices. Ad-hoc signing remains configured: a new build may prompt again. Stable Developer ID distribution and actual repeated-cycle prompt-count acceptance remain outstanding.

## 0.5.0 typed auxiliary objects

Device reports and portable packages use the same space encryption but separate MIME types (`application/vnd.bastet.device-encrypted`, `application/vnd.bastet.portable-encrypted`). Older session clients list only `application/octet-stream` and ignore both. Each report update verifies the existing encrypted stream identity and parent/type before replacing its own preallocated object. Portable packages retain immutable queue semantics and never execute or overwrite profile settings. Device names and selected agent names are encrypted metadata shared with holders of the space key. See [scope and retention](SYNC_CONTROL.md).
