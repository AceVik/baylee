//! `cards/instants/mv_4/defensive_maneuvers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn defensive_maneuvers() -> CardIndex {
    card_index("812701c3-c9b7-4877-bf2e-c37e0f551c45")
}

/// Defensive Maneuvers gives creatures of the chosen type +0/+4 until end of
/// turn (surviving 3 damage from Lightning Bolt), skips other types, and wears
/// off at cleanup.
#[test]
fn defensive_maneuvers_gives_chosen_type_plus_zero_four_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[defensive_maneuvers(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");

    let white: Vec<ObjectId> = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| white.contains(&id));
    cast_with_floating(&mut engine, p0, defensive_maneuvers());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Defensive Maneuvers");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(pt(&engine, elf0), (1, 5));
    assert_eq!(pt(&engine, elf1), (1, 5));
    assert_eq!(pt(&engine, goblin0), (1, 1));

    // A 1/5 Elf survives 3 damage from Lightning Bolt.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[elf0], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());

    // After cleanup, both damage and the +0/+4 wear off so Elf returns to 1/1.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert_eq!(pt(&engine, elf1), (1, 1));
}
