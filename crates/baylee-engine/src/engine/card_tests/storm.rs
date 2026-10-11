//! Cast counts, counted copy continuations, and magecraft's separate event.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::TargetPrompt;
use crate::event::{Cause, GameEvent};
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup(artist: bool) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                index::BRAIN_FREEZE,
                index::LIGHTNING_BOLT,
                index::FLUSTERSTORM,
            ],
        )
        .hand(1, &[index::LIGHTNING_BOLT, index::COUNTERSPELL])
        .battlefield(
            0,
            if artist {
                &[index::STORM_KILN_ARTIST]
            } else {
                &[]
            },
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for seat in [P0, P1] {
        let player = &mut e.dev_state_mut(seat).unwrap().players[usize::from(seat.get())];
        player.mana_pool.add(ManaColor::Blue, 20);
        player.mana_pool.add(ManaColor::Red, 20);
    }
    e.refresh_offer();
    e
}

fn choose_player(e: &mut Engine<RegistryLookup>, seat: PlayerId, target: PlayerId) {
    match e.pending() {
        Pending::ChoosePlayer { .. } => e.apply(seat, PlayerAction::ChoosePlayer(target)).unwrap(),
        Pending::ChooseTargets { .. } => e
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![target],
                },
            )
            .unwrap(),
        _ => panic!("{:?}", e.pending()),
    }
}

fn copied(e: &Engine<RegistryLookup>) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::SpellCopied { .. }))
        .count()
}

fn settle(e: &mut Engine<RegistryLookup>) {
    for _ in 0..20_000 {
        if stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                min: 0,
                reason: TargetPrompt::Retarget { .. },
                ..
            } => {
                if let Some(res) = &e.resolution {
                    // Storm's count stays in one instruction, not a vector of N operations.
                    if res
                        .effects
                        .iter()
                        .any(|op| matches!(op, baylee_cards_dsl::Effect::CopyThisSpell { .. }))
                    {
                        assert_eq!(res.effects.len(), 1);
                    }
                }
                e.apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![],
                    },
                )
                .unwrap();
            }
            Pending::YesNo { player, .. } => e.apply(player, PlayerAction::YesNo(false)).unwrap(),
            other => panic!("unexpected storm question: {other:?}"),
        }
    }
    panic!("storm failed to settle");
}

#[test]
fn storm_counts_both_players_casts_and_magecraft_counts_copies_separately() {
    let mut e = setup(true);
    let library = e.state().zones.list(ZoneLocation::Library(P1)).len();
    cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
    choose_player(&mut e, P0, P1);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, P1, index::LIGHTNING_BOLT);
    choose_player(&mut e, P1, P0);
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, P0, index::BRAIN_FREEZE);
    choose_player(&mut e, P0, P1);
    settle(&mut e);
    assert_eq!(copied(&e), 2);
    assert_eq!(e.state().per_turn.spells_cast, vec![2, 1]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P1)).len(),
        library - 9
    );
    let treasures = e
        .state()
        .battlefield_view()
        .iter()
        .filter(|id| {
            e.state().object(**id).is_some_and(|o| {
                o.controller == P0
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::artifact::TREASURE)
            })
        })
        .count();
    assert_eq!(
        treasures, 4,
        "our two casts and two copies; not the opponent's spell"
    );
}

#[test]
fn storm_zero_and_large_counts_do_not_retrigger_from_the_copies() {
    for count in [0, 1, 2, 257] {
        let mut e = setup(false);
        e.dev_state_mut(P0).unwrap().per_turn.spells_cast[1] = count;
        cast_with_floating(&mut e, P0, index::BRAIN_FREEZE);
        choose_player(&mut e, P0, P1);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Stack).len(),
            2,
            "storm triggers even for zero earlier casts"
        );
        settle(&mut e);
        assert_eq!(copied(&e), usize::try_from(count).unwrap());
        assert_eq!(e.state().per_turn.spells_cast, vec![1, count]);
    }
}

#[test]
fn a_countered_storm_spell_is_still_copied_with_its_original_target() {
    let mut e = setup(false);
    e.dev_state_mut(P0).unwrap().per_turn.spells_cast[0] = 2;
    let library = e.state().zones.list(ZoneLocation::Library(P1)).len();
    let spell = in_hand(&e, P0, index::BRAIN_FREEZE).unwrap();
    cast_with_floating(&mut e, P0, index::BRAIN_FREEZE);
    choose_player(&mut e, P0, P1);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, P1, index::COUNTERSPELL);
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![spell],
        },
    )
    .unwrap();
    settle(&mut e);
    assert_eq!(
        copied(&e),
        2,
        "the later Counterspell does not increase storm"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P1)).len(),
        library - 6
    );
    assert!(in_graveyard(&e, P0, index::BRAIN_FREEZE).is_some());
}

#[test]
fn flusterstorm_copies_tax_independently_without_countering_abilities() {
    let mut e = setup(false);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let bolt = in_hand(&e, P1, index::LIGHTNING_BOLT).unwrap();
    cast_with_floating(&mut e, P1, index::LIGHTNING_BOLT);
    choose_player(&mut e, P1, P0);
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, P0, index::FLUSTERSTORM);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt],
        },
    )
    .unwrap();
    settle(&mut e);
    assert_eq!(copied(&e), 1);
    assert!(in_graveyard(&e, P1, index::LIGHTNING_BOLT).is_some());
    assert_eq!(e.state().players[0].life, 20);
}

#[test]
fn a_storm_trigger_can_be_removed_without_removing_its_spell() {
    let mut e = setup(false);
    e.dev_state_mut(P0).unwrap().per_turn.spells_cast[0] = 2;
    let library = e.state().zones.list(ZoneLocation::Library(P1)).len();
    cast_with_floating(&mut e, P0, index::BRAIN_FREEZE);
    choose_player(&mut e, P0, P1);
    let trigger = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    assert_eq!(
        e.state().object(trigger).unwrap().kind,
        crate::object::ObjectKind::AbilityOnStack
    );
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            trigger,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    settle(&mut e);
    assert_eq!(copied(&e), 0);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P1)).len(),
        library - 3
    );
}
