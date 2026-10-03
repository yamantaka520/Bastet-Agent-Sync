# iCloud Drive and OneDrive folders

Version 0.7.0 adds an encrypted local-folder route to the actual sync worker; it is separate from the old synthetic folder diagnostic. Native cloud delivery and two-device acceptance remain independent checks in [validation](VALIDATION.md).

## Setup

1. Enable the provider's desktop sync client and choose a dedicated folder within its synchronized drive. Keep that folder and its contents downloaded locally. Bastet does not sign into Apple or Microsoft and cannot verify your provider account or cloud quota from a filesystem path.
2. Select **iCloud Drive folder** or **OneDrive folder** in the sync destination menu, then choose the folder using the native picker. The Store build retains a security-scoped bookmark; a moved or revoked folder requires choosing it again.
3. On the first computer, create a space and save its recovery kit outside the shared folder. Cancelling the Save dialog does not mark setup complete. The kit contains the decryption key, not an Apple/Microsoft password.
4. On another computer, select the corresponding downloaded folder and join using that provider's recovery kit. The encrypted proof must be present and decrypt correctly before setup can complete. A Google Drive recovery kit cannot join a folder-provider space.
5. Select sources, save setup, then explicitly start synchronization. Pausing is required before changing the destination. Existing agent data is never used as the destination for restored sessions.

The credentials entry also provides another recovery-kit export. Google authorization and its existing space remain separate when a folder provider is selected. Switching providers selects a different space; it does not migrate existing Google cloud history automatically.

## What completion means

The app encrypts and validates immutable snapshots, then writes them to the selected folder. A completed local cycle establishes that the app finished its local work. The provider's desktop client remains responsible for upload/download, authentication, storage availability and delivery to another computer. An encrypted proof validates the space and key; it does not confirm cloud delivery.

Incomplete, unavailable or corrupt objects must not be imported or treated as deletion. Leave the provider running and keep the shared folder downloaded. Existing missing-source, branch preservation, conflict and isolated-restore rules still apply. The app's HTTP bandwidth meter/limits cover its direct Google transport, not the separate iCloud or OneDrive client. Folder storage measurement reports locally visible object sizes, not cloud quota usage.

## Platforms and limits

- iCloud Drive requires a working local iCloud Drive installation. Apple documents macOS and Windows setup; Linux iCloud parity is not provided by this feature.
- OneDrive folder mode uses the same encrypted adapter with separate provider configuration. Direct Microsoft Graph sign-in is not implemented.
- Setup and unit tests cannot establish that a provider accepted a file or delivered it to another device. Native provider, sandbox restart and physical two-device checks must be recorded separately.
- The recovery kit must remain available outside the synchronized object folder. Loss of every copy of the key makes the encrypted data unrecoverable.

Official references: [iCloud Drive setup](https://support.apple.com/118443), [keep iCloud files downloaded on Windows](https://support.apple.com/guide/icloud-windows/icw8531ad6b7/icloud), [Apple dataless-file behavior](https://developer.apple.com/documentation/technotes/tn3150-getting-ready-for-data-less-files), and [Microsoft's app-folder API](https://learn.microsoft.com/en-us/graph/onedrive-sharepoint-appfolder) for the separate future direct-login route.
