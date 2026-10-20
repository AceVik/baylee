//! `cards/creatures/mv_1/balduvian_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn balduvian_shaman() -> CardIndex {
    card_index("3354c3d3-5713-4139-beb2-4fa17f6b47d9")
}

fn crusade() -> CardIndex {
    card_index("4692740f-be90-459f-8d90-c4ae71771595")
}

fn bad_moon() -> CardIndex {
    card_index("fc5d3341-cbce-49e5-93cc-8add92479dca")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Balduvian Shaman (3354c3d3-5713-4139-beb2-4fa17f6b47d9) — {U} 1/1 Creature — Human Cleric Shaman:
/// - "{T}: Change the text of target white enchantment you control that doesn't
///   have cumulative upkeep by replacing all instances of one color word with
///   another. That enchantment gains 'Cumulative upkeep {1}.'"
#[test]
fn balduvian_shaman_rewrites_color_word_on_white_enchantment_you_control_without_cumulative_upkeep_and_grants_cumulative_upkeep_one()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                balduvian_shaman(),
                crusade(),
                bad_moon(),
                savannah_lions(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[crusade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_crusade = on_battlefield(&engine, P0, crusade()).expect("my Crusade seated");
    let their_crusade = on_battlefield(&engine, P1, crusade()).expect("their Crusade seated");
    let moon = on_battlefield(&engine, P0, bad_moon()).expect("Bad Moon seated");
    let lions = on_battlefield(&engine, P0, savannah_lions()).expect("Savannah Lions seated");
    let elves = on_battlefield(&engine, P0, llanowar_elves()).expect("Llanowar Elves seated");

    // Two Crusades on the battlefield -> white creatures get +2/+2 (Savannah Lions is 4/3),
    // green creatures get +0/+0 (Llanowar Elves is 1/1).
    assert_eq!(pt(&engine, lions), (4, 3));
    assert_eq!(pt(&engine, elves), (1, 1));

    activate(&mut engine, P0, balduvian_shaman(), 0);
    let menu = aim_at(&mut engine, P0, my_crusade);
    assert!(
        menu.contains(&my_crusade),
        "white enchantment you control without cumulative upkeep is legal"
    );
    assert!(!menu.contains(&moon), "non-white enchantment is excluded");
    assert!(
        !menu.contains(&their_crusade),
        "opponent's white enchantment is excluded"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // WUBRG order: white (0) -> green (4) is 0 * 4 + (4 - 1) = 3.
    engine
        .apply(P0, PlayerAction::ChooseNumber(3))
        .expect("replace white with green");
    pass_until(&mut engine, stack_is_empty);

    // My Crusade now pumps green creatures instead of white creatures!
    assert_eq!(
        pt(&engine, lions),
        (3, 2),
        "only opponent's Crusade pumps white now"
    );
    assert_eq!(
        pt(&engine, elves),
        (2, 2),
        "my Crusade now pumps green creatures +1/+1"
    );

    // On P0's next upkeep, my Crusade now has Cumulative upkeep {1}: an age
    // counter is placed on it and P0 is asked whether to pay {1}.
    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert_eq!(
        counters_on(&engine, my_crusade, baylee_cards_dsl::counters::AGE),
        1,
        "Crusade gained Cumulative upkeep {{1}} and received an age counter"
    );

    // Also verify that once Balduvian Shaman untaps on P0's turn, my Crusade is
    // no longer a legal target because it now has cumulative upkeep.
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline cumulative upkeep -> sacrifice Crusade");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, crusade()).is_none());
}
