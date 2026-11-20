//! `cards/creatures/mv_5/riptide_chronologist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn riptide_chronologist() -> CardIndex {
    card_index("e1c6e606-c4e4-4067-8cb9-46753785dfeb")
}

/// Riptide Chronologist sacrifices itself for `{U}` to untap every creature
/// of the chosen type across all players' battlefields while leaving tapped
/// creatures of other types tapped.
#[test]
fn riptide_chronologist_sacrifices_to_untap_chosen_type_for_all_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                riptide_chronologist(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // Have P1 attack with its Llanowar Elves so it enters P0's turn tapped.
    reach_their_main_phase(&mut engine, p1);
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    attack(&mut engine, &[opp_elf]);
    block(&mut engine, &[]);
    through_combat(&mut engine);

    // Now on P0's turn: P1's Elf is tapped; attack with P0's Festering Goblin
    // and tap P0's Llanowar Elves for mana before activating in second main.
    reach_their_main_phase(&mut engine, p0);
    let chronologist =
        on_battlefield(&engine, p0, riptide_chronologist()).expect("Chronologist seated");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let my_goblin = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");

    attack(&mut engine, &[my_goblin]);
    block(&mut engine, &[]);
    through_combat(&mut engine);

    // Tap both Island and Llanowar Elves (Goblin is tapped from combat, opp_elf from P1's attack).
    tap_mana_where(&mut engine, p0, |_| true);
    assert!(is_tapped(&engine, my_elf));
    assert!(is_tapped(&engine, my_goblin));
    assert!(is_tapped(&engine, opp_elf));

    activate(&mut engine, p0, riptide_chronologist(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    assert_eq!(
        zone_of(&engine, chronologist),
        Zone::Graveyard,
        "sacrificed as part of activation cost"
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(!is_tapped(&engine, my_elf), "controller's Elf untapped");
    assert!(!is_tapped(&engine, opp_elf), "opponent's Elf untapped");
    assert!(
        is_tapped(&engine, my_goblin),
        "non-Elf Goblin remained tapped"
    );
}
