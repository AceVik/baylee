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
    local.tried.insert(id.clone());
    // The game's Scryfall door sends the same search.
    let Some(request) = crate::cardtext::scryfall::search(&card.oracle_id, lang) else {
        return;
    };
    let answer: Answer = default();
    let callback = Arc::clone(&answer);
    local.pending = Some(answer);
    local.next_request = time.elapsed_secs_f64() + 0.3;
    let lang = lang.to_string();
    ehttp::fetch(request, move |response| {
        // The gateway's own rule over Scryfall's rows, as the game's door
        // reads them, filed under the printing the deck builder asked about.
        let mut entries: Vec<CardTextEntry> = response
            .ok()
            .filter(|r| r.ok)
            .and_then(|r| {
                r.text()
                    .and_then(|body| crate::cardtext::scryfall::entry(&lang, body))
            })
            .into_iter()
            .collect();
        for entry in &mut entries {
            entry.scryfall_id.clone_from(&id);
        }
        if let Ok(mut target) = callback.lock() {
            *target = Some(entries);
        }
    });
}

#[cfg(test)]
mod tests {
    /// The deck builder reads Scryfall through the game's door, so a
    /// name-only promo no longer hides an older printing's translated type
    /// line (#239, `card_entry`'s most complete local printing).
    #[test]
    fn translated_types_win_over_a_name_only_promo() {
        let entry = crate::cardtext::scryfall::entry(
            "de",
            r#"{"data":[{"id":"new","lang":"de","released_at":"2024-01-01","name":"Forest","printed_name":"Wald","type_line":"Basic Land — Forest"},{"id":"older","lang":"de","released_at":"2020-01-01","name":"Forest","printed_name":"Wald","type_line":"Basic Land — Forest","printed_type_line":"Basisland — Wald"}]}"#,
        )
        .expect("an entry");
        assert_eq!(entry.faces[0].type_line, "Basisland — Wald");
    }
}
