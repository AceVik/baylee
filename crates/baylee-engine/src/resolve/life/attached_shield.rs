//! An Aura's untargeted shield follows its exact host, including source LKI.
use crate::prevention::{Shield, ShieldKind, Shielded};
use crate::resolve::{Resolution, amount2, source_attachment_lki, source_version, subjects};
use crate::state::GameState;
use baylee_cards_dsl::Amount;
use baylee_core::ids::DamageSourceRef;

pub(super) fn add(state: &mut GameState, res: &Resolution, amount: Amount) {
    let Some(host) = crate::eval::attached_for_ability(
        state,
        res.source,
        source_version(state, res),
        source_attachment_lki(state, res.on_stack),
    ) else {
        return;
    };
    let host = DamageSourceRef {
        object: host.id,
        version: host.version,
    };
    if !subjects::on_battlefield(state, host) {
        return;
    }
    let amount = amount2(&amount, state, res.controller, res);
    if amount == 0 {
        return;
    }
    let origin = super::origin(state, res);
    state.shields.push_from(
        Shield {
            protects: Shielded::Object(host.object, host.version),
            kind: ShieldKind::Next(amount),
            controller: res.controller,
        },
        Some(origin),
    );
}
