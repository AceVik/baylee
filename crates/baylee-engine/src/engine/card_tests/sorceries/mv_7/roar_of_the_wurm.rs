//! `cards/sorceries/mv_7/roar_of_the_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn roar_of_the_wurm() -> CardIndex {
    card_index("b8b9e6fd-b3ed-4fbc-8753-74018fa33caa")
}

fn wurms(engine: &Engine<RegistryLookup>, player: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|object| {
                object.controller == player
                    && object.card.is_none()
                    && object
                        .characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::WURM)
            })
        })
        .collect()
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, roar: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: roar })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// The ordinary spell costs {6}{G} and creates one green 6/6 Wurm under its
/// caster's control. Flashback costs {3}{G}, creates a distinct second token,
/// and exiles the exact graveyard incarnation instead of returning it there.
#[test]
fn roar_of_the_wurm_creates_a_wurm_and_flashes_back_for_four_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 11])
        .hand(0, &[roar_of_the_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, roar_of_the_wurm());
    pass_until(&mut engine, stack_is_empty);
    let first = wurms(&engine, p0);
    assert_eq!(first.len(), 1, "one spell creates exactly one Wurm");
    assert_eq!(pt(&engine, first[0]), (6, 6));
    assert!(
        engine
            .state()
            .object(first[0])
            .unwrap()
            .characteristics()
            .colors
            .contains(baylee_core::color::Color::Green)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    let roar = in_graveyard(&engine, p0, roar_of_the_wurm()).unwrap();

    start_flashback(&mut engine, p0, roar);
    assert_eq!(engine.state().object(roar).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    let all = wurms(&engine, p0);
    assert_eq!(all.len(), 2, "each resolution creates its own token");
    assert!(all.contains(&first[0]), "the first token remains in play");
    assert_ne!(all[0], all[1]);
    for &wurm in &all {
        let characteristics = engine.state().object(wurm).unwrap().characteristics();
        assert_eq!(pt(&engine, wurm), (6, 6));
        assert!(
            characteristics
                .colors
                .contains(baylee_core::color::Color::Green)
        );
        assert!(
            characteristics
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::WURM)
        );
    }
    assert_eq!(engine.state().object(roar).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, roar_of_the_wurm()).is_none());
}

/// The ordinary {6}{G} cost does not make the graveyard card available, and
/// three green mana cannot pay its {3}{G} flashback cost.
#[test]
fn roar_of_the_wurm_flashback_is_not_offered_with_only_three_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 3])
        .hand(0, &[roar_of_the_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let roar = hand_to_graveyard(&mut engine, p0, roar_of_the_wurm());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&roar));
    assert!(!legal.payable.contains(&roar));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: roar })
            .is_err()
    );
    assert_eq!(engine.state().object(roar).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert!(wurms(&engine, p0).is_empty());
}
