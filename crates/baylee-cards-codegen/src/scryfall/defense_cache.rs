//! Compatible schema-two migration: only Battle metadata requires a request.

use super::{CodegenError, Path, PathBuf, STAMP_FILE, ScryfallCard, fs};

fn complete_face(types: Option<&str>, defense: Option<&str>) -> bool {
    let Some(types) = types else { return false };
    let battle = types
        .split('—')
        .next()
        .unwrap_or(types)
        .split_whitespace()
        .any(|word| word == "Battle");
    if battle {
        defense.is_some_and(|n| n.parse::<u16>().is_ok())
    } else {
        defense.is_none()
    }
}

fn complete(card: &ScryfallCard) -> bool {
    match card.card_faces.as_deref() {
        Some(faces) if !faces.is_empty() => faces
            .iter()
            .all(|f| complete_face(f.type_line.as_deref(), f.defense.as_deref())),
        _ => complete_face(card.type_line.as_deref(), card.defense.as_deref()),
    }
}

/// Migrates schema two without refetching provably compatible non-Battle cards.
///
/// Every face is inspected, including back faces. Requests use the held
/// printing ID, preserving the cache's printing selection. Fetch failures
/// leave all records and the old stamp intact; interrupted writes leave the
/// old stamp, and retry skips any already repaired records. Older schemas
/// return `false` and retain the existing full-refresh path.
///
/// # Errors
/// Cache I/O, malformed payloads, or an authoritative response still missing
/// required metadata. No successful schema stamp is written on failure.
pub fn migrate_defense_cache(agent: &ureq::Agent, cache: &Path) -> Result<bool, CodegenError> {
    migrate_with(cache, |id| {
        let url = format!("{}/cards/{id}", super::API);
        super::get_json(agent, &url).map_err(|error| match error {
            super::FetchError::NotFound => CodegenError::CardNotFound(id.to_string()),
            super::FetchError::Other(message) => CodegenError::Http { url, message },
        })
    })
}

fn migrate_with(
    cache: &Path,
    mut fetch: impl FnMut(&str) -> Result<ScryfallCard, CodegenError>,
) -> Result<bool, CodegenError> {
    let stamp = cache.join(STAMP_FILE);
    if !fs::read_to_string(&stamp)
        .is_ok_and(|text| text.lines().next().is_some_and(|line| line.trim() == "2"))
    {
        return Ok(false);
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(cache)
        .map_err(CodegenError::io(cache))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(CodegenError::io(cache))?;
    paths.sort();
    let mut replacements = Vec::new();
    for path in paths {
        if path.extension().is_none_or(|ext| ext != "json")
            || path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("catalog-"))
        {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(CodegenError::io(&path))?;
        let held: ScryfallCard = serde_json::from_str(&text)?;
        if complete(&held) {
            continue;
        }
        let fresh = fetch(&held.id)?;
        if fresh.id != held.id || !complete(&fresh) {
            return Err(CodegenError::Defense {
                card: held.name,
                value: fresh.defense,
            });
        }
        replacements.push((path, fresh));
    }
    for (path, card) in replacements {
        let temporary = path.with_extension("part");
        fs::write(&temporary, serde_json::to_string_pretty(&card)?)
            .map_err(CodegenError::io(&temporary))?;
        fs::rename(&temporary, &path).map_err(CodegenError::io(&path))?;
    }
    let temporary = stamp.with_extension("part");
    fs::write(
        &temporary,
        "3\n\nCompatible defense migration from schema 2.\n",
    )
    .map_err(CodegenError::io(&temporary))?;
    fs::rename(&temporary, &stamp).map_err(CodegenError::io(&stamp))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Cache(PathBuf);
    impl Cache {
        fn new(version: u32) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "baylee-defense-cache-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join(STAMP_FILE), format!("{version}\n")).unwrap();
            Self(path)
        }
        fn put(&self, file: &str, json: &str) {
            fs::write(self.0.join(file), json).unwrap();
        }
    }
    impl Drop for Cache {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn card(json: &str) -> ScryfallCard {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn compatible_non_battles_are_unchanged_and_only_battle_printing_is_refetched() {
        let cache = Cache::new(2);
        let ordinary = r#"{"id":"normal","name":"Normal","type_line":"Creature"}"#;
        let double = r#"{"id":"dfc","name":"Front // Back","card_faces":[{"name":"Front","type_line":"Creature"},{"name":"Back","type_line":"Land"}]}"#;
        cache.put("normal.json", ordinary);
        cache.put("double.json", double);
        cache.put("catalog-battle-types.json", "[\"Siege\"]");
        cache.put(
            "siege.json",
            r#"{"id":"held-print","name":"Siege","type_line":"Battle — Siege"}"#,
        );
        let mut requests = Vec::new();
        assert!(migrate_with(&cache.0, |id| {
            requests.push(id.to_string());
            Ok(card(r#"{"id":"held-print","name":"Siege","type_line":"Battle — Siege","defense":"5"}"#))
        }).unwrap());
        assert_eq!(requests, ["held-print"]);
        assert_eq!(
            fs::read_to_string(cache.0.join("normal.json")).unwrap(),
            ordinary
        );
        assert_eq!(
            fs::read_to_string(cache.0.join("double.json")).unwrap(),
            double
        );
        assert!(!super::super::schema_stale(&cache.0));
    }

    #[test]
    fn battle_on_back_face_requires_refresh_and_failure_preserves_stamp_and_payloads() {
        let cache = Cache::new(2);
        let stale = r#"{"id":"back-print","name":"Front // Siege","card_faces":[{"name":"Front","type_line":"Creature"},{"name":"Siege","type_line":"Battle — Siege"}]}"#;
        cache.put("a.json", stale);
        cache.put(
            "z.json",
            r#"{"id":"second","name":"Siege2","type_line":"Battle"}"#,
        );
        let error = migrate_with(&cache.0, |id| {
            if id == "second" {
                return Err(CodegenError::CardNotFound(id.to_string()));
            }
            Ok(card(
                r#"{"id":"back-print","name":"Front // Siege","card_faces":[{"name":"Front","type_line":"Creature"},{"name":"Siege","type_line":"Battle — Siege","defense":"4"}]}"#,
            ))
        });
        assert!(error.is_err());
        assert_eq!(fs::read_to_string(cache.0.join(STAMP_FILE)).unwrap(), "2\n");
        assert_eq!(fs::read_to_string(cache.0.join("a.json")).unwrap(), stale);
        let mut requests = 0;
        assert!(
            migrate_with(&cache.0, |id| {
                requests += 1;
                Ok(card(&format!(
                    r#"{{"id":"{id}","name":"Siege","type_line":"Battle","defense":"4"}}"#
                )))
            })
            .unwrap()
        );
        assert_eq!(requests, 2);
        assert!(!super::super::schema_stale(&cache.0));
    }

    #[test]
    fn older_schema_uses_original_refresh_and_incomplete_authoritative_data_cannot_stamp() {
        let old = Cache::new(1);
        assert!(!migrate_with(&old.0, |_| panic!("older cache uses full refresh")).unwrap());
        assert!(super::super::schema_stale(&old.0));
        let cache = Cache::new(2);
        let stale = r#"{"id":"siege","name":"Siege","type_line":"Battle"}"#;
        cache.put("siege.json", stale);
        assert!(migrate_with(&cache.0, |_| Ok(card(stale))).is_err());
        assert_eq!(fs::read_to_string(cache.0.join(STAMP_FILE)).unwrap(), "2\n");
    }
}
