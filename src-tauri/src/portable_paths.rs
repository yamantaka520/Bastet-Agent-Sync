//! Unicode-preserving filenames that can be materialized on every supported OS.
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};
use unicode_normalization::UnicodeNormalization;

pub(crate) fn path_is_portable(path: &str) -> bool {
    !path.is_empty()
        && path.len() < 2048
        && !path
            .chars()
            .any(|c| c < '\u{20}' || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && path.split('/').all(|part| {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with([' ', '.'])
                || part.len() > 255
                || part.encode_utf16().count() > 255
            {
                return false;
            }
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                && !["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|suffix| {
                        matches!(
                            suffix,
                            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                        )
                    })
                })
        })
}
fn key(path: &str) -> String {
    path.nfc()
        .collect::<String>()
        .to_uppercase()
        .to_lowercase()
        .nfc()
        .collect()
}
pub(crate) fn paths_do_not_collide<'a>(paths: impl Iterator<Item = &'a str>) -> bool {
    let mut entries: BTreeMap<String, (String, bool)> = BTreeMap::new();
    for path in paths {
        let parts = path.split('/').collect::<Vec<_>>();
        for index in 0..parts.len() {
            let prefix = parts[..=index].join("/");
            let file = index == parts.len() - 1;
            let normalized = key(&prefix);
            if let Some((original, was_file)) = entries.get(&normalized) {
                if original != &prefix || file || *was_file {
                    return false;
                }
            } else {
                entries.insert(normalized, (prefix, file));
            }
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_and_portable_payloads_share_filename_constraints() {
        for name in [
            "a?.jsonl",
            "a*.jsonl",
            "a|b.md",
            "COM¹.txt",
            "LPT².md",
            "a/../b",
            "/a",
            "C:/a",
            "a\\b",
            "a\u{1f}b",
            "a.",
        ] {
            assert!(!path_is_portable(name), "{name}");
        }
        assert!(path_is_portable("sessions/專案/對話.jsonl"));
        assert!(!path_is_portable(&format!("{}.md", "界".repeat(100))));
        for paths in [
            vec!["A/a", "a/b"],
            vec!["a", "a/b"],
            vec!["a/b", "A"],
            vec!["é/a", "e\u{301}/b"],
            vec!["a/A", "a/a"],
        ] {
            assert!(!paths_do_not_collide(paths.into_iter()));
        }
        assert!(paths_do_not_collide(["a/b", "a/c", "別/d"].into_iter()));
    }
}
