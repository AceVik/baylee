//! `cards/2001/src/ody/howling_gale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn howling_gale() -> CardIndex {
    card_index("06118cec-ae63-4720-988d-df9cc15ccfab")
}

fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(slot))
            .unwrap();
    }
}

/// Deals 1 damage to each creature with flying (both players') and each player,
/// sparing ground creatures; Flashback {1}{G} repeats the damage and exiles
/// Howling Gale instead of returning it to the graveyard.
#[test]
fn howling_gale_damages_fliers_and_each_player_then_flashback_exiles() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                air_elemental(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[wall_of_swords(), llanowar_elves()])
        .hand(0, &[howling_gale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_flier = on_battlefield(&engine, P0, air_elemental()).unwrap();
    let my_ground = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_flier = on_battlefield(&engine, P1, wall_of_swords()).unwrap();
    let their_ground = on_battlefield(&engine, P1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, P0, howling_gale());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(my_flier).unwrap().damage, 1);
    assert_eq!(engine.state().object(their_flier).unwrap().damage, 1);
    assert_eq!(engine.state().object(my_ground).unwrap().damage, 0);
    assert_eq!(engine.state().object(their_ground).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
    let gale = in_graveyard(&engine, P0, howling_gale()).expect("resolved spell in graveyard");

    // 1 mana is not enough for Flashback {1}{G}.
    tap_all_mana(&mut engine, P0);
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: gale })
            .is_err()
    );

    // Next P0 turn: untap all Forests and Flashback for {1}{G}.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    start_flashback(&mut engine, P0, gale);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(my_flier).unwrap().damage, 1);
    assert_eq!(engine.state().object(their_flier).unwrap().damage, 1);
    assert_eq!(engine.state().object(my_ground).unwrap().damage, 0);
    assert!(on_battlefield(&engine, P1, llanowar_elves()).is_some());
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, P0, howling_gale()).is_none());
    assert_eq!(engine.state().object(gale).unwrap().zone, Zone::Exile);
}
