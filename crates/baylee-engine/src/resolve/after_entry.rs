//! Remaining work within an instruction that first creates a permanent.
use super::{GameState, Pending, Resolution};
use baylee_core::ids::SubtypeId;

/// An instruction cannot inspect its entrants until their replacements finish.
#[derive(Clone, Copy, Debug, Hash)]
pub enum AfterEntry {
    /// Choose a Jace token and give it the already evaluated loyalty amount.
    EmpowerJace {
        /// Counters to place after creation has completed.
        amount: u16,
    },
    /// Choose an Army and apply the already evaluated amass instruction.
    Amass {
        /// Creature subtype the Army gains.
        subtype: SubtypeId,
        /// Counters to place after creation has completed.
        amount: u16,
    },
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    after: AfterEntry,
) -> Option<Pending> {
    match after {
        AfterEntry::EmpowerJace { amount } => super::empower::after_entry(state, res, amount),
        AfterEntry::Amass { subtype, amount } => {
            super::amass::after_entry(state, res, subtype, amount)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::resolve::{AwaitingOp, Flow, SubjectContext};
    use crate::state::ReplacementEntry;
    use crate::text_changes::TextChangeMap;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{
        Amount, CounterKind, Duration, Effect, Filter, Modifier, ReplacementRule,
    };
    use baylee_core::ids::{EffectId, ObjectId, PlayerId, SeatSet};
    use smallvec::SmallVec;

    fn resolution(effect: Effect) -> Resolution {
        Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: PlayerId::new(0),
            effects: vec![
                effect,
                Effect::GainLife {
                    amount: Amount::Fixed(7),
                },
            ],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: SubjectContext::default(),
            text: TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
            coin_stake: None,
        }
    }

    #[test]
    fn created_jaces_and_armies_finish_entry_before_selection_and_later_instructions() {
        let player = PlayerId::new(0);
        for (effect, kind) in [
            (
                Effect::EmpowerJace {
                    token: &baylee_cards::tokens::JACE_PLANESWALKER_BLUE,
                    amount: 3,
                },
                CounterKind::Loyalty,
            ),
            (
                Effect::Amass {
                    token: &baylee_cards::tokens::ARMY_0_0_BLACK,
                    subtype: baylee_core::generated::subtypes::creature::ORC,
                    amount: Amount::Fixed(3),
                },
                CounterKind::P1P1,
            ),
        ] {
            let mut state =
                GameState::from_preset(&preset(578, &[]), &SyntheticLookup::new(vec![])).unwrap();
            let name = state.names.intern("replacement source");
            let source = state.create_bare(
                player,
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            let modifier = Modifier::AdditionalCounterPlacement { n: 1 };
            let timestamp = state.next_timestamp();
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: player,
                origin: EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::Dsl(&Filter::ControlledByYou),
                modifier,
            });
            for rule in [
                ReplacementRule::DoubleTokenCreation {
                    controller_filter: &Filter::ControlledByYou,
                },
                ReplacementRule::DoubleCounterPlacement {
                    object_filter: &Filter::ControlledByYou,
                },
            ] {
                state.replacement_rules.push(ReplacementEntry {
                    source,
                    controller: player,
                    rule,
                });
            }
            let before = state.players[0].life;
            let mut res = resolution(effect);
            assert!(matches!(
                crate::resolve::run(&mut state, &mut res),
                Flow::Enter
            ));
            assert_eq!(res.pc, 0, "the creation instruction is not finished");
            assert_eq!(state.players[0].life, before);
            assert!(crate::entry::advance(&mut state).is_none());
            let Flow::Wait(Pending::ChooseCards { options, .. }) =
                crate::resolve::resume_entries(&mut state, &mut res)
            else {
                panic!("choose among the two committed tokens before SBAs");
            };
            assert_eq!(options.len(), 2);
            for &id in &options {
                assert_eq!(state.object(id).unwrap().counters.get(kind), 0);
            }
            assert!(matches!(
                crate::resolve::resume(&mut state, &mut res, &[options[1]]),
                Flow::Wait(Pending::YesNo { .. })
            ));
            assert_eq!(
                state.players[0].life, before,
                "counter ordering precedes the following instruction"
            );
            crate::replacement::counter_order::answer(&mut state, true);
            res.awaiting = None;
            assert!(matches!(
                crate::resolve::run(&mut state, &mut res),
                Flow::Complete
            ));
            assert_eq!(state.object(options[1]).unwrap().counters.get(kind), 8);
            assert_eq!(state.object(options[0]).unwrap().counters.get(kind), 0);
            assert_eq!(
                state.players[0].life,
                before + 7,
                "the following instruction runs exactly once"
            );
        }
    }

    #[test]
    fn after_entry_amount_and_subtype_distinguish_continuations() {
        let fingerprint = |after| {
            AwaitingOp::Entry {
                next: None,
                after: Some(after),
            }
            .search_fingerprint()
        };
        assert_ne!(
            fingerprint(AfterEntry::EmpowerJace { amount: 1 }),
            fingerprint(AfterEntry::EmpowerJace { amount: 2 })
        );
        assert_ne!(
            fingerprint(AfterEntry::Amass {
                subtype: baylee_core::generated::subtypes::creature::ORC,
                amount: 2
            }),
            fingerprint(AfterEntry::Amass {
                subtype: baylee_core::generated::subtypes::creature::ZOMBIE,
                amount: 2
            }),
        );
    }
}
