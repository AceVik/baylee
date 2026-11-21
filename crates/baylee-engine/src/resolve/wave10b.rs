//! Wave 10b resolution helpers:
//! - Awaken N (`CR 702.113a`): `Effect::AwakenLand`
//! - Cipher (`CR 702.99a–c`): `Effect::Cipher` & `Effect::CipherCastEncoded`
//! - Champion (`CR 702.72a–b`): `Effect::Champion` & `Effect::ReturnChampioned`

use super::{
    AwaitingOp, Cause, ChoicePrompt, GameState, ObjectId, ObjectKind, Pending, Resolution,
    ZoneLocation, ZonePosition, eval,
};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::object::Rider;
use crate::zone::Zone;
use baylee_cards_dsl::{CounterKind, Duration, Filter, KeywordSet, Layer, Modifier};
use baylee_core::ids::{EffectId, PlayerId};
use baylee_core::types::TypeSet;

/// `Effect::AwakenLand` (`CR 702.113a`): put `count` +1/+1 counters on each
/// legal targeted land you control and make it a 0/0 Elemental creature with
/// haste indefinitely ("It's still a land", `CR 205.1b`).
pub(super) fn resolve_awaken_land(state: &mut GameState, res: &Resolution, count: u16) {
    let you = res.controller;
    let targets = res.targets.clone();
    for land in targets {
        let still_same = super::subjects::target(state, res, land)
            .is_some_and(|r| super::subjects::on_battlefield(state, r))
            && state.object(land).is_some_and(|o| o.controller == you);
        if !still_same {
            continue;
        }
        if count > 0 {
            crate::replacement::put_counters_by(state, you, land, CounterKind::P1P1, count);
        }
        if state
            .object(land)
            .is_none_or(|o| o.zone != Zone::Battlefield)
        {
            continue;
        }
        let filter = EffectFilter::object(state, land);
        for (layer, modifier) in [
            (Layer::Type, Modifier::AddType(TypeSet::CREATURE)),
            (
                Layer::Type,
                Modifier::ReplaceCreatureTypes(
                    baylee_core::generated::subtypes::creature::ELEMENTAL,
                ),
            ),
            (Layer::Ability, Modifier::AddKeyword(KeywordSet::HASTE)),
            (Layer::PtSet, Modifier::SetPT(0, 0)),
        ] {
            let ts = state.next_timestamp();
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer,
                filter,
                modifier,
                timestamp: ts,
                duration: Duration::Indefinitely,
                origin: EffectOrigin::Resolution,
            });
        }
        state.invalidate_projections();
    }
}

/// `Effect::Cipher` (`CR 702.99b`): if the resolving spell is represented by a
/// card, its controller may exile it encoded on a creature they control.
pub(super) fn resolve_cipher(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let spell = state.object(res.on_stack)?;
    if spell.zone != Zone::Stack || !spell.is_card() {
        return None;
    }
    let you = res.controller;
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == you && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::CipherEncode);
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::CipherEncode,
        total: None,
    })
}

/// Resume `AwaitingOp::CipherEncode` (`CR 702.99b–c`).
pub(super) fn resume_cipher_encode(state: &mut GameState, res: &Resolution, chosen: &[ObjectId]) {
    let Some(&host) = chosen.first() else {
        return;
    };
    let you = res.controller;
    let Some(host_version) = state
        .object(host)
        .filter(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT)
                && o.controller == you
                && o.characteristics().types.contains(TypeSet::CREATURE)
        })
        .map(|o| o.version)
    else {
        return;
    };
    let Some(owner) = state
        .object(res.on_stack)
        .filter(|o| o.zone == Zone::Stack && o.is_card())
        .map(|o| o.owner)
    else {
        return;
    };
    if let Some(obj) = state.object_mut(res.on_stack) {
        obj.kind = ObjectKind::Card;
    }
    if state
        .move_object(
            res.on_stack,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .is_ok()
        && let Some(card) = state
            .object_mut(res.on_stack)
            .filter(|o| o.zone == Zone::Exile)
    {
        card.riders.push(Rider::CipherEncodedOn {
            host,
            version: host_version,
        });
    }
}

/// `Effect::CipherCastEncoded` (`CR 702.99b–c`): while the encoded card remains
/// in exile, the creature's controller may copy it and cast the copy without
/// paying its mana cost.
pub(super) fn resolve_cipher_cast_encoded(
    state: &GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let cipher_ref = super::subjects::event(state, res)?;
    state
        .object(cipher_ref.object)
        .filter(|o| o.zone == Zone::Exile && o.version == cipher_ref.version)?;
    super::hidden_exile::continue_play(state, res, vec![cipher_ref], true)
}

/// `Effect::Champion` (`CR 702.72a`): sacrifice this permanent unless you exile
/// another permanent you control matching `filter`.
pub(super) fn resolve_champion(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
) -> Option<Pending> {
    let you = res.controller;
    let ctx = res.rule_context();
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            id != res.source
                && state.object(id).is_some_and(|o| {
                    o.controller == you && eval::matches_with_context(filter, state, o, you, ctx)
                })
        })
        .collect();
    if options.is_empty() {
        sacrifice_champion_self(state, res);
        return None;
    }
    res.awaiting = Some(AwaitingOp::ChampionExile);
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Champion,
        total: None,
    })
}

/// Resume `AwaitingOp::ChampionExile` (`CR 702.72a`).
pub(super) fn resume_champion_exile(state: &mut GameState, res: &Resolution, chosen: &[ObjectId]) {
    let you = res.controller;
    if let Some(&card) = chosen.first()
        && let Some(owner) = state
            .object(card)
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && o.controller == you
                    && card != res.source
            })
            .map(|o| o.owner)
    {
        let host_version = super::source_version(state, res)
            .or_else(|| state.object(res.source).map(|o| o.version))
            .unwrap_or(0);
        let components = state.merged_components(card);
        let rider = Rider::ChampionedBy {
            host: res.source,
            version: host_version,
        };
        if let Some(obj) = state.object_mut(card) {
            obj.kind = ObjectKind::Card;
            obj.riders.push(rider);
        }
        let _ = state.move_object(
            card,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
        for component in components {
            if let Some(obj) = state
                .object_mut(component)
                .filter(|o| o.zone == Zone::Exile)
                && !obj.riders.contains(&rider)
            {
                obj.riders.push(rider);
            }
        }
    } else {
        sacrifice_champion_self(state, res);
    }
}

fn sacrifice_champion_self(state: &mut GameState, res: &Resolution) {
    let Some(source) = super::subjects::source(state, res) else {
        return;
    };
    if super::subjects::on_battlefield(state, source)
        && state
            .object(source.object)
            .is_some_and(|o| o.controller == res.controller)
    {
        let _ = state.sacrifice_object(source.object, Cause::Effect);
    }
}

/// `Effect::ReturnChampioned` (`CR 702.72a`): return cards exiled by this
/// permanent's `Champion` ability to the battlefield under their owners' control.
pub(super) fn resolve_return_championed(state: &mut GameState, res: &Resolution) {
    let host = res.source;
    let host_version = super::source_version(state, res);
    let mut returning = Vec::new();
    for seat in 0..state.players.len() {
        let p = PlayerId::new(u8::try_from(seat).expect("seat fits u8"));
        for &card in state.zones.list(ZoneLocation::Exile(p)) {
            let matches_host = state.object(card).is_some_and(|o| {
                o.riders.iter().any(|r| match *r {
                    Rider::ChampionedBy {
                        host: h,
                        version: v,
                    } => h == host && host_version.is_none_or(|ver| v == ver),
                    _ => false,
                })
            });
            if matches_host {
                returning.push(card);
            }
        }
    }
    for card in returning {
        if let Some(obj) = state.object_mut(card) {
            obj.kind = ObjectKind::Permanent;
            obj.riders.retain(|r| {
                !matches!(
                    r,
                    Rider::ChampionedBy {
                        host: h,
                        version: v
                    } if *h == host && host_version.is_none_or(|ver| *v == ver)
                )
            });
            obj.set_controller(obj.owner);
        }
        let _ = state.move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
    }
}
