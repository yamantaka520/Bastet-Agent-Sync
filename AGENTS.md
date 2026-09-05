# Bastet Agent Sync

- Read REQUIREMENTS.md and docs/MASTER_PLAN.md before implementation.
- Preserve the Bastet calico-cat identity and five locales: zh-Hant, zh-Hans, en, ja, ko.
- Keep public docs generic: no real user home paths, infrastructure addresses, credentials, or conversation contents.
- Distinguish implemented, tested, and planned behavior. Never simulate a successful sync.
- Never overwrite active agent stores. Test adapters against fixtures before importing real sessions.
- Every milestone updates docs/VALIDATION.md and CHANGELOG.md and records sources, topic, index, and append-only log in the user's BastetMind notebook. Notebook-only records must not be committed here.
