//! `cards/enchantments/mv_2/blizzard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blizzard() -> CardIndex {
    card_index("698feaaa-2fff-4809-81d1-4caad44c6c0a")
}

/// Blizzard (698feaaa-2fff-4809-81d1-4caad44c6c0a) — {G}{G} Enchantment:
/// - "Cast this spell only if you control a snow land."
///
/// With {G}{G} floating and only plain Forests of its own, P0 cannot cast it,
/// and P1's Snow-Covered Forest does not count; once P0 plays a
/// Snow-Covered Forest, the same mana casts it.
#[test]
fn blizzard_can_be_cast_only_while_you_control_a_snow_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[index::SNOW_COVERED_FOREST])
        .hand(0, &[blizzard(), index::SNOW_COVERED_FOREST])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    let card = in_hand(&engine, P0, blizzard()).expect("Blizzard in hand");
    assert!(
        engine.apply(P0, PlayerAction::CastSpell { card }).is_err(),
        "P0 controls no snow land; the opponent's does not count"
    );

    play_land(&mut engine, P0, index::SNOW_COVERED_FOREST);
    cast_with_floating(&mut engine, P0, blizzard());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, blizzard()).is_some(),
        "with a snow land of P0's, the floating {{G}}{{G}} cast it"
    );
}

/// - "Cumulative upkeep {2}"
/// - "Creatures with flying don't untap during their controllers' untap steps."
///
/// Every flier, not only P0's: P1's tapped Wall of Swords stays tapped through
/// P1's untap step, and P0's through P0's, while P0's ground Elf untaps. The
/// age counter arrives at P0's upkeep, and declining the {2} sacrifices it.
#[test]
fn blizzard_keeps_flying_creatures_from_untapping_and_has_cumulative_upkeep_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                index::SNOW_COVERED_FOREST,
                wall_of_swords(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[wall_of_swords()])
        .hand(0, &[blizzard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, blizzard());
    pass_until(&mut engine, stack_is_empty);

    let blizz = on_battlefield(&engine, P0, blizzard()).expect("Blizzard seated");
    let flier = on_battlefield(&engine, P0, wall_of_swords()).expect("my flier seated");
    let their_flier = on_battlefield(&engine, P1, wall_of_swords()).expect("their flier seated");
    let ground = on_battlefield(&engine, P0, llanowar_elves()).expect("ground Elf seated");

    for id in [flier, their_flier, ground] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }

    reach_their_main_phase(&mut engine, P1);
    assert!(
        is_tapped(&engine, their_flier),
        "P1's flier did not untap during P1's untap step"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert!(
        is_tapped(&engine, flier),
        "P0's flier did not untap during P0's untap step"
    );
    assert!(
        !is_tapped(&engine, ground),
        "creature without flying untapped normally"
    );
    assert_eq!(
        counters_on(&engine, blizz, baylee_cards_dsl::counters::AGE),
        1
    );

    // Decline to pay cumulative upkeep {2} -> Blizzard is sacrificed.
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline cumulative upkeep");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, blizzard()).is_none());
    assert!(in_graveyard(&engine, P0, blizzard()).is_some());
}
