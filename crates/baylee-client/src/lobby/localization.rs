//! Visible-card localization for gateways without a translated catalog.
use super::{LobbyState, Press};
use baylee_client_core::card_face::CardTextEntry;
use bevy::prelude::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

type Answer = Arc<Mutex<Option<Vec<CardTextEntry>>>>;
#[derive(Default)]
pub(super) struct Localized {
    lang: String,
    tried: BTreeSet<String>,
    cached: Vec<CardTextEntry>,
    pending: Option<Answer>,
    next_request: f64,
    revision: u64,
    dirty: bool,
    flush_after: f64,
}

pub(super) fn update(
    mut state: ResMut<LobbyState>,
    rows: Query<&Press>,
    time: Res<Time>,
    mut local: Local<Localized>,
) {
    let lang = state.lobby.lang().code();
    if local.lang != lang {
        *local = Localized {
            lang: lang.into(),
            cached: crate::cardtext::cache::load(&format!("builder-{lang}")),
            ..default()
        };
        local.tried = local.cached.iter().map(|e| e.scryfall_id.clone()).collect();
    }
    if let Some(answer) = local.pending.as_ref().and_then(|p| p.lock().ok()?.take()) {
        local.pending = None;
        if !answer.is_empty() {
            for entry in answer {
                local.cached.retain(|e| e.scryfall_id != entry.scryfall_id);
                local.cached.push(entry);
            }
            local.dirty = true;
            local.flush_after = time.elapsed_secs_f64() + 0.7;
        }
    }
    let revision = state.lobby.builder().pool_revision();
    if revision != local.revision || (local.dirty && time.elapsed_secs_f64() >= local.flush_after) {
        if local.dirty {
            crate::cardtext::cache::store(&format!("builder-{lang}"), &local.cached);
            local.dirty = false;
        }
        // Idempotent cache replay must not invalidate a retained UI every frame.
        let changed = state
            .bypass_change_detection()
            .lobby
            .builder_mut()
            .localize(&local.cached);
        let language = state.lobby.lang();
        let types_changed = state
            .bypass_change_detection()
            .lobby
            .builder_mut()
            .localize_types(language);
        if changed || types_changed {
            state.set_changed();
        }
        local.revision = state.lobby.builder().pool_revision();
    }
    if cfg!(test)
        || lang == "en"
        || local.pending.is_some()
        || time.elapsed_secs_f64() < local.next_request
    {
        return;
    }
    let card = rows
        .iter()
        .filter_map(|p| match p {
            Press::Inspect(slot) => state.lobby.builder().card(*slot),
            _ => None,
        })
        .find(|c| {
            !local.tried.contains(&c.scryfall_id) && uuid::Uuid::parse_str(&c.oracle_id).is_ok()
        });
    let Some(card) = card else {
        return;
    };
    let id = card.scryfall_id.clone();
    let query = format!("oracleid:{} lang:{lang}", card.oracle_id);
    let url = format!(
        "https://api.scryfall.com/cards/search?unique=prints&order=released&include_multilingual=true&q={}",
        super::http::escape(&query)
    );
    let answer: Answer = default();
    let callback = Arc::clone(&answer);
    local.tried.insert(id.clone());
    local.pending = Some(answer);
    local.next_request = time.elapsed_secs_f64() + 0.3;
    let mut request = ehttp::Request::get(url);
    request.headers.insert("Accept", "application/json");
    request.headers.insert("User-Agent", "baylee-client/0.1");
    ehttp::fetch(request, move |response| {
        let mut entries = response
            .ok()
            .filter(|r| r.ok)
            .and_then(|r| r.text().map(best_translation))
            .unwrap_or_default();
        entries.truncate(1);
        for entry in &mut entries {
            entry.scryfall_id.clone_from(&id);
        }
        if let Ok(mut target) = callback.lock() {
            *target = Some(entries);
        }
    });
}

/// Prefer a complete translated printing over a recent promo lacking its type line.
fn best_translation(body: &str) -> Vec<CardTextEntry> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(cards) = value.get("data").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let quality = |card: &serde_json::Value| {
        let face = card
            .get("card_faces")
            .and_then(|f| f.get(0))
            .unwrap_or(card);
        ["printed_name", "printed_type_line", "printed_text"]
            .iter()
            .filter(|key| {
                face.get(**key)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|s| !s.is_empty())
            })
            .count()
    };
    let Some(best) = cards.iter().max_by_key(|card| quality(card)) else {
        return Vec::new();
    };
    crate::cardtext::scryfall::parse(&serde_json::json!({"data": [best]}).to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn translated_types_win_over_a_name_only_promo() {
        let entries = super::best_translation(
            r#"{"data":[{"id":"new","lang":"de","name":"Forest","printed_name":"Wald","type_line":"Basic Land — Forest"},{"id":"older","lang":"de","name":"Forest","printed_name":"Wald","type_line":"Basic Land — Forest","printed_type_line":"Basisland — Wald"}]}"#,
        );
        assert_eq!(entries[0].faces[0].type_line, "Basisland — Wald");
    }
}
