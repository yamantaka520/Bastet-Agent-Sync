# Store privacy policy draft

Prepared 2026-10-02 for the first Mac App Store candidate. This is a review draft, not a published policy URL or a completed App Store privacy questionnaire.

## What the app processes

Bastet AI Sync by Bastet AI is the Mac App Store edition of Bastet Agent Sync. It processes the local agent conversation, settings and skill files you select, along with the local paths and project mappings needed to synchronize and prepare supported conversations. The first Store edition does not synchronize Agent Memory OS.

You choose folders through the macOS folder picker. The app stores local permission bookmarks so it can reopen authorized folders. These bookmarks are kept in the app's local configuration, not sent as synchronization content. Local settings, synchronization journals, received copies and prepared conversation profiles may remain on the device.

## Google Drive and encryption

After you configure Google authorization and a synchronization space, the app communicates directly with Google's OAuth and Drive services. Synchronization starts when you explicitly start it. Selected payloads and device status reports are encrypted on the device with the space key using XChaCha20-Poly1305 before upload. Google still receives the network requests, file sizes, object identifiers, folder names and other service metadata needed to provide Drive. People with both access to the shared Drive space and its recovery key can decrypt synchronized content.

Google account information returned by Drive, such as the account name, email address and permission identifier, is used to identify the connected account. OAuth tokens and space keys are kept in the operating system credential store. The Store edition uses a separate credential service from the independently distributed app. Protect recovery kits: they contain the information required to join the encrypted space.

The app does not send synchronized content to a Bastet-hosted server and does not include advertising or analytics tracking. The Store edition receives software updates through the App Store. Opening support or project links uses an external browser, where the destination's own privacy practices apply.

## Controls and retention

You can pause synchronization, deselect sources, change folder permissions and disconnect Google authorization. Pausing or uninstalling the app does not delete existing Drive objects, copies on other devices, prepared conversation profiles or operating system credential entries. Manage remote copies and sharing through Google Drive; revoke application access through your Google account. Local downloaded cache cleanup does not delete the original source or remote archive.

The app preserves competing versions and does not automatically erase an archive when a source temporarily disappears. Copies remain until explicitly removed using the relevant app, filesystem or cloud controls. The developer cannot recover a lost space key or erase copies held on another person's device.

## Support and updates

Support is available through the [project issue tracker](https://github.com/yamantaka520/Bastet-Agent-Sync/issues). Issues are public; do not post private conversations, account tokens or recovery keys. This policy draft must be reconciled with the final binary and the developer's actual support practices before publication.

## Submission assessment still required

Apple's [privacy disclosure definitions](https://developer.apple.com/app-store/app-privacy-details/) apply separately to the store label. Encryption alone does not settle the disclosure of service metadata. Review Google authorization/account information, encrypted Drive payloads and metadata, device reports, optional support submissions and all bundled dependencies before publishing the label. The application also implements XChaCha20-Poly1305 outside Apple's operating system encryption APIs; complete Apple's [export compliance assessment](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance/) before assigning an encryption-exemption flag. The candidate label and evidence are in [the privacy assessment](PRIVACY_ASSESSMENT.md).
