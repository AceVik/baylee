//! Optional control acquisition is chosen on resolution, without targeting.

use super::{AwaitingOp, Pending, Resolution};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::state::GameState;
use baylee_cards_dsl::{Duration, Filter, Modifier, PlayerRel};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId, PlayerId};

/// The offered exact objects and the recipient of their control.
#[derive(Clone, Debug, Hash)]
pub struct ControlChoice {
    /// Player who may take control.
    pub player: PlayerId,
    /// Original source incarnation, independent of successful subject moves.
    pub source: Option<DamageSourceRef>,
    /// The objects the public choice actually offered.
    pub options: Vec<DamageSourceRef>,
    /// Lifetime of the new control effect.
    pub duration: Duration,
}

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    player: PlayerRel,
    filter: &'static Filter,
    duration: Duration,
) -> Option<Pending> {
    let player = super::players_of(player, state, res.controller, res)
        .first()
        .copied()?;
    if state.has_left(player)
        || !super::continuous_subjects::source_duration_starts(state, res, duration)
    {
        return None;
    }
    let source = super::source_version(state, res)
        .map(|version| DamageSourceRef {
            object: res.source,
            version,
        })
        .or_else(|| state.source_identity(res.source));
    let options: Vec<_> = state
        .battlefield_seen()
        .filter_map(|id| {
            let object = state.object(id)?;
            (crate::eval::matches_with_context(
                filter,
                state,
                object,
                res.controller,
                res.rule_context(),
            ) && crate::permanent_guards::permits_resolution_effect(
                state,
                EffectFilter::object(state, id),
                Modifier::GainControl,
                player,
            ))
            .then_some(DamageSourceRef {
                object: id,
                version: object.version,
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    let question = Pending::ChooseCards {
        player,
        options: options.iter().map(|reference| reference.object).collect(),
        min: 0,
        max: 1,
        total: None,
        prompt: crate::choice::ChoicePrompt::Generic,
    };
    res.awaiting = Some(AwaitingOp::ControlChoice(ControlChoice {
        player,
        source,
        options,
        duration,
    }));
    Some(question)
}

pub(super) fn finish(state: &mut GameState, choice: &ControlChoice, chosen: &[ObjectId]) {
    let Some(reference) = chosen
        .first()
        .and_then(|chosen| {
            choice
                .options
                .iter()
                .find(|reference| reference.object == *chosen)
        })
        .copied()
    else {
        return;
    };
    if state.has_left(choice.player) || !super::subjects::on_battlefield(state, reference) {
        return;
    }
    if choice.duration == Duration::WhileSourceOnBattlefield
        && !choice
            .source
            .is_some_and(|source| super::subjects::on_battlefield(state, source))
    {
        return;
    }
    let filter = EffectFilter::ObjectIs(reference.object, reference.version);
    if !crate::permanent_guards::permits_resolution_effect(
        state,
        filter,
        Modifier::GainControl,
        choice.player,
    ) {
        return;
    }
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: choice.source.map(|source| source.object),
        controller: choice.player,
        origin: EffectOrigin::Resolution,
        layer: Modifier::GainControl.layer(),
        timestamp,
        duration: choice.duration,
        filter,
        modifier: Modifier::GainControl,
    });
}

pub(super) fn sacrifice_n_or_else(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    count: u8,
    otherwise: &'static [baylee_cards_dsl::Effect],
) -> Option<Pending> {
    if count == 0 {
        return None;
    }
    let options = super::chosen::options(state, res.controller, filter, res.controller, res.source);
    if options.len() > usize::from(count) {
        res.awaiting = Some(AwaitingOp::SacrificeAllChosen);
        return Some(Pending::ChooseCards {
            player: res.controller,
            options,
            min: count,
            max: count,
            total: None,
            prompt: crate::choice::ChoicePrompt::Generic,
        });
    }
    super::sacrifice_together(state, &options);
    if options.len() < usize::from(count) {
        return super::run_nested(state, res, otherwise);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, creature, land, preset};
    use crate::resolve::{Flow, SubjectContext};
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Amount, Effect};
    use baylee_core::ids::SeatSet;
    use smallvec::SmallVec;

    fn fixture(lands: usize) -> (GameState, Resolution) {
        let source = creature(850_005, "Synthetic control source", 5, 7, &[]);
        let resource = land(850_006, "Synthetic sacrifice resource", &[]);
        let lookup = SyntheticLookup::new(vec![source, resource]);
        let board: Vec<_> = std::iter::once(850_005)
            .chain(std::iter::repeat_n(850_006, lands))
            .collect();
        let state = GameState::from_preset(&preset(724, &board), &lookup).unwrap();
        let source = state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                state
                    .object(*id)
                    .unwrap()
                    .card
                    .is_some_and(|card| card.index.get() == 850_005)
            })
            .unwrap();
        let version = state.object(source).unwrap().version;
        let res = Resolution {
            source,
            on_stack: source,
            controller: PlayerId::new(0),
            effects: vec![],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: Some(PlayerId::new(1)),
            target_lki: None,
            subject: SubjectContext::default().with_source_version(Some(version)),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            coin_stake: None,
            draw_watch: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
        };
        (state, res)
    }

    #[test]
    fn mandatory_sacrifice_performs_available_part_and_branches_only_if_incomplete() {
        for count in 0..=3 {
            let (mut state, mut res) = fixture(count);
            let before = state.players[0].life;
            let pending = sacrifice_n_or_else(
                &mut state,
                &mut res,
                &Filter::LAND,
                2,
                &[Effect::GainLife {
                    amount: Amount::Fixed(5),
                }],
            );
            if count == 3 {
                let Some(Pending::ChooseCards {
                    player,
                    options,
                    min,
                    max,
                    ..
                }) = pending
                else {
                    panic!("a mandatory choice among three resources");
                };
                assert_eq!(player, res.controller);
                assert_eq!((min, max, options.len()), (2, 2, 3));
                assert!(matches!(
                    crate::resolve::resume(&mut state, &mut res, &options[..2]),
                    Flow::Complete
                ));
            } else {
                assert!(pending.is_none());
            }
            let remaining = state
                .battlefield_seen()
                .filter(|id| {
                    state
                        .object(*id)
                        .unwrap()
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::LAND)
                })
                .count();
            assert_eq!(remaining, count.saturating_sub(2));
            assert_eq!(
                state.players[0].life,
                before + if count < 2 { 5 } else { 0 }
            );
        }
    }

    #[test]
    fn optional_control_choice_can_take_the_source_and_ends_when_it_leaves() {
        let (mut state, mut res) = fixture(0);
        let Some(Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        }) = start(
            &state,
            &mut res,
            PlayerRel::Chosen,
            &Filter::YOUR_CREATURE,
            Duration::WhileSourceOnBattlefield,
        )
        else {
            panic!("opponent makes a non-targeting optional choice");
        };
        assert_eq!((player, min, max), (PlayerId::new(1), 0, 1));
        assert_eq!(
            options,
            vec![res.source],
            "the source itself is a legal creature choice"
        );
        let Some(AwaitingOp::ControlChoice(choice)) = res.awaiting.take() else {
            panic!("control continuation");
        };
        finish(&mut state, &choice, &[res.source]);
        state.refresh_characteristics();
        assert_eq!(
            state.object(res.source).unwrap().controller,
            PlayerId::new(1)
        );
        state
            .move_object(
                res.source,
                ZoneLocation::Graveyard(res.controller),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert!(
            !state
                .effects
                .iter()
                .any(|effect| effect.modifier == Modifier::GainControl)
        );
    }
}
