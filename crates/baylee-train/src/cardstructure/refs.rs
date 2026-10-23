//! References are built solely from `PlayerView` / legal candidates. The catalog
//! contains public rules, but a hidden card's identity never selects a row.
use crate::carddsl::UNKNOWN_CARD;
use crate::features4::{Candidate4, EncodedState4};
use baylee_view::{PublicObject, StackItem};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRef {
    pub key: String,
    pub role: u8,
    /// Public grant handle, used only to select the referenced ability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}
fn push(out: &mut Vec<RuleRef>, role: u8, key: String) {
    out.push(RuleRef {
        key,
        role,
        slot: None,
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
    if let Some(StackItem::Ability {
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
            });
        }
    }
    out
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
        } else if cid != UNKNOWN_CARD {
            push(&mut out, 7, format!("a/{cid}/{fid}/{}", c.ability_idx));
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
    for (i, ab) in card.abilities_for_face(face).iter().enumerate() {
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
