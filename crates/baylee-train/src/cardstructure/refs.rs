//! References are built solely from `PlayerView` / legal candidates. The catalog
//! contains public rules, but a hidden card's identity never selects a row.
use crate::carddsl::UNKNOWN_CARD;
use crate::features4::{Candidate4, EncodedState4};
use baylee_view::{PublicObject, StackItem};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRef {
    pub key: String,
    pub role: u8,
    /// Public grant handle, used only to select the referenced ability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    /// Runtime index in a composed ordinary ability list, not a grant slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability: Option<u32>,
}
fn push(out: &mut Vec<RuleRef>, role: u8, key: String) {
    out.push(RuleRef {
        key,
        role,
        slot: None,
        ability: None,
    });
}
pub fn card_refs(cid: u32, face: u8, token: u16) -> Vec<RuleRef> {
    let mut out = vec![];
    if cid != UNKNOWN_CARD {
        push(&mut out, 1, format!("c/{cid}/{face}"));
        push(&mut out, 2, format!("i/{cid}/{face}"));
    } else if token > 0 {
        push(&mut out, 1, format!("t/{}", token - 1));
    }
    out
}
pub fn public_refs(obj: &PublicObject, cid: u32, face: u8, token: u16) -> Vec<RuleRef> {
    let mut out = card_refs(cid, face, token);
    if let Some(name) = obj.chosen_name {
        push(&mut out, 3, format!("c/{}/{}", name.card.get(), name.face));
        push(&mut out, 4, format!("i/{}/{}", name.card.get(), name.face));
    }
    if let Some(prep) = obj.prepared_spell {
        push(&mut out, 6, format!("c/{}/{}", prep.card.get(), prep.face));
    }
    if matches!(obj.stack_item, Some(StackItem::Ability { .. })) && !obj.ability_sources.is_empty()
    {
        for source in &obj.ability_sources {
            if let Some(key) = source_key(source) {
                push(&mut out, 5, key);
            }
        }
    } else if let Some(StackItem::Ability {
        ability,
        rules,
        token,
        ..
    }) = &obj.stack_item
    {
        if let (Some(ab), Some(rules)) = (ability, rules) {
            push(
                &mut out,
                5,
                if ab.is_listed_ability() {
                    format!("a/{}/{}/{}", rules.card.get(), rules.face, ab.index)
                } else {
                    format!("q/{}", ab.index)
                },
            );
        } else if let Some(tok) = token {
            push(&mut out, 5, format!("ta/{}/{}", tok.token, tok.index));
        }
    }
    for grant in &obj.grants {
        if let Some(rules) = grant.rules {
            let key = grant.ability.map_or_else(
                || format!("c/{}/{}", rules.card.get(), rules.face),
                |ab| format!("a/{}/{}/{ab}", rules.card.get(), rules.face),
            );
            out.push(RuleRef {
                key,
                role: 6,
                slot: Some(grant.slot),
                ability: None,
            });
        }
    }
    if obj.stack_item.is_none() {
        for source in &obj.ability_sources {
            if let Some(key) = source_key(source) {
                out.push(RuleRef {
                    key,
                    role: 6,
                    slot: None,
                    ability: Some(source.index),
                });
            }
        }
    }
    out
}

fn source_key(source: &baylee_view::AbilitySource) -> Option<String> {
    let mut key = if let Some(rules) = source.rules {
        format!("a/{}/{}/{}", rules.card.get(), rules.face, source.ability)
    } else {
        let token = source.token?;
        format!("ta/{}/{}", token.token, token.index)
    };
    if let Some(modifier) = source.copy_modifier {
        let _ = write!(key, "/copy/{modifier}");
    }
    Some(key)
}
pub fn candidate_refs(c: &Candidate4, state: &EncodedState4) -> Vec<RuleRef> {
    let face = c.named_face();
    let mut out = if face == 255 && c.entity_idx >= 0 {
        state.entity_rule_refs[c.entity_idx as usize].clone()
    } else {
        card_refs(c.card_id, if face == 255 { 0 } else { face }, 0)
    };
    if c.ability_idx >= 0 && c.entity_idx >= 0 {
        let e = c.entity_idx as usize;
        let cid = state.entity_card_ids[e];
        let fid = state.entity_face_ids[e];
        let token = state.entity_token_ids[e];
        if let Some(slot) = baylee_engine::choice::granted_slot(c.ability_idx as u32) {
            for grant in &state.entity_rule_refs[e] {
                if grant.slot == Some(slot) {
                    let mut r = grant.clone();
                    r.role = 7;
                    out.push(r);
                }
            }
        } else if c.ability_idx >= i64::from(u32::MAX - 11) {
            push(&mut out, 7, format!("q/{}", c.ability_idx));
        } else if let Some(source) = state.entity_rule_refs[e]
            .iter()
            .find(|r| r.ability.map(i64::from) == Some(c.ability_idx))
        {
            let mut selected = source.clone();
            selected.role = 7;
            out.push(selected);
        } else if cid != UNKNOWN_CARD {
            // Legal offers use the active concatenated door list; catalog
            // roots and captured stack provenance use per-face indices.
            let (face, index) = baylee_cards::by_index(baylee_core::ids::CardIndex::new(cid))
                .filter(|def| def.has_shared_type_line())
                .map_or((fid, c.ability_idx), |def| {
                    let left = state.entity_feat[e][39] != 0.0;
                    let right = state.entity_feat[e][40] != 0.0;
                    let left_count = def.faces[0].abilities.len() as i64;
                    if right && !left {
                        (1, c.ability_idx)
                    } else if left && right && c.ability_idx >= left_count {
                        (1, c.ability_idx - left_count)
                    } else {
                        (0, c.ability_idx)
                    }
                });
            push(&mut out, 7, format!("a/{cid}/{face}/{index}"));
        } else if token > 0 {
            push(&mut out, 7, format!("ta/{}/{}", token - 1, c.ability_idx));
        }
    }
    out.extend(c.extra_rules.clone());
    out
}

pub fn mode_refs(cid: u32, face: u8, kind: &baylee_engine::choice::CastModeKind) -> Vec<RuleRef> {
    use baylee_cards_dsl::AbilityDef as A;
    use baylee_engine::choice::CastModeKind as K;
    let mut out = vec![];
    let Some(card) = baylee_cards::by_index(baylee_core::ids::CardIndex::new(cid)) else {
        return out;
    };
    let (face, modes): (usize, Vec<usize>) = match kind {
        K::Alternative(i) => {
            push(&mut out, 7, format!("alt/{cid}/{face}/{i}"));
            return out;
        }
        K::Face(f) | K::PlayLandFace(f) => (*f, vec![]),
        K::Mode(m) => (usize::from(face), vec![*m]),
        K::Modes(bits) => (
            usize::from(face),
            (0..8).filter(|i| bits & (1 << i) != 0).collect(),
        ),
        K::EscapeGranted { face, modes, .. } => {
            (*face, (0..8).filter(|i| modes & (1 << i) != 0).collect())
        }
        _ => return out,
    };
    push(&mut out, 7, format!("c/{cid}/{face}"));
    for (i, ab) in super::catalog::face_abilities(card, face)
        .iter()
        .enumerate()
    {
        let list = match ab {
            A::ModalSpell { modes, .. }
            | A::ModalActivated { modes, .. }
            | A::ModalTriggered { modes, .. } => *modes,
            _ => continue,
        };
        for &m in &modes {
            if m < list.len() {
                push(&mut out, 7, format!("a/{cid}/{face}/{i}/m/{m}"));
            }
        }
    }
    out
}
