//! Continuous effects follow only subjects reached by this resolution.

use super::{Resolution, subjects};
use crate::state::GameState;
use baylee_cards_dsl::Duration;
use baylee_core::ids::{DamageSourceRef, ObjectId};

pub(super) fn target(state: &GameState, res: &Resolution, id: ObjectId) -> Option<DamageSourceRef> {
    subjects::target(state, res, id).filter(|reference| {
        subjects::is_current(state, *reference)
            && state
                .object(id)
                .is_some_and(|object| !object.status.contains(crate::object::Status::PHASED_OUT))
    })
}

pub(super) fn source_duration_starts(
    state: &GameState,
    res: &Resolution,
    duration: Duration,
) -> bool {
    match duration {
        Duration::WhileSourceOnBattlefield
        | Duration::WhileYouControlSource
        | Duration::WhileSourceTapped { .. } => super::source_version(state, res)
            .map(|version| DamageSourceRef {
                object: res.source,
                version,
            })
            .or_else(|| state.source_identity(res.source))
            .is_some_and(|source| subjects::on_battlefield(state, source)),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::event::Cause;
    use crate::object::ObjectKind;
    use crate::resolve::{Flow, SubjectContext, TargetLki};
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Effect, Filter, KeywordSet, Modifier, TargetSpec};
    use baylee_core::ids::{PlayerId, SeatSet};
    use smallvec::SmallVec;

    fn fixture() -> (GameState, Resolution, ObjectId) {
        let player = PlayerId::new(0);
        let mut state =
            GameState::from_preset(&preset(717, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let name = state.names.intern("Synthetic duration source");
        let source = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let version = state.object(source).unwrap().version;
        let target = state.zones.list(ZoneLocation::Library(player))[0];
        let res = Resolution {
            source,
            on_stack: source,
            controller: player,
            effects: vec![],
            pc: 0,
            targets: smallvec::smallvec![target],
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: SubjectContext::default().with_source_version(Some(version)),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
            coin_stake: None,
        };
        (state, res, target)
    }

    fn move_to(state: &mut GameState, object: ObjectId, destination: ZoneLocation) {
        if destination == ZoneLocation::Battlefield {
            state.object_mut(object).unwrap().kind = ObjectKind::Permanent;
        }
        state
            .move_object(object, destination, ZonePosition::Top, Cause::Effect)
            .unwrap();
    }

    #[test]
    fn continuous_modifier_follows_this_resolutions_successful_reanimation() {
        let (mut state, mut res, object) = fixture();
        move_to(&mut state, object, ZoneLocation::Graveyard(res.controller));
        res.effects = vec![
            Effect::GraveyardToBattlefield {
                target: TargetSpec::Object(&Filter::Any),
                owner_control: false,
                counters: None,
            },
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                duration: Duration::Indefinitely,
            },
        ];
        let mut flow = crate::resolve::run(&mut state, &mut res);
        while matches!(flow, Flow::Enter) {
            assert!(crate::entry::advance(&mut state).is_none());
            flow = crate::resolve::resume_entries(&mut state, &mut res);
        }
        assert!(matches!(flow, Flow::Complete));
        assert!(state.effects.iter().any(|effect| effect.modifier
            == Modifier::AddKeyword(KeywordSet::HASTE)
            && effect.filter.names(state.object(object).unwrap())));
    }

    #[test]
    fn continuous_subject_never_follows_an_unrelated_blink_or_hidden_zone_change() {
        for destination in [
            ZoneLocation::Exile(PlayerId::new(0)),
            ZoneLocation::Hand(PlayerId::new(0)),
        ] {
            let (mut state, mut res, object) = fixture();
            move_to(&mut state, object, ZoneLocation::Battlefield);
            let old = state.object(object).unwrap();
            res.target_lki = Some(vec![TargetLki {
                id: object,
                version: old.version,
                controller: old.controller,
                chars: old.characteristics().clone(),
            }]);
            move_to(&mut state, object, destination);
            move_to(&mut state, object, ZoneLocation::Battlefield);
            res.effects = vec![Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                duration: Duration::Indefinitely,
            }];
            assert!(matches!(
                crate::resolve::run(&mut state, &mut res),
                Flow::Complete
            ));
            assert!(
                !state
                    .effects
                    .iter()
                    .any(|effect| effect.modifier == Modifier::AddKeyword(KeywordSet::HASTE))
            );
        }
    }

    #[test]
    fn source_lifetime_cannot_start_for_the_sources_later_incarnation() {
        let (mut state, res, _) = fixture();
        assert!(source_duration_starts(
            &state,
            &res,
            Duration::WhileSourceOnBattlefield
        ));
        move_to(&mut state, res.source, ZoneLocation::Hand(res.controller));
        move_to(&mut state, res.source, ZoneLocation::Battlefield);
        assert!(!source_duration_starts(
            &state,
            &res,
            Duration::WhileSourceOnBattlefield
        ));
    }

    #[test]
    fn source_departure_ends_resolved_control_before_a_same_resolution_return() {
        use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
        use baylee_core::ids::EffectId;
        let (mut state, res, object) = fixture();
        move_to(&mut state, object, ZoneLocation::Battlefield);
        let modifier = Modifier::GainControl;
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: PlayerId::new(1),
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::object(&state, object),
            modifier,
        });
        move_to(&mut state, res.source, ZoneLocation::Hand(res.controller));
        move_to(&mut state, res.source, ZoneLocation::Battlefield);
        assert!(
            !state
                .effects
                .iter()
                .any(|effect| effect.modifier == Modifier::GainControl),
            "the old continuous effect cannot resume when its source returns"
        );
    }
}
