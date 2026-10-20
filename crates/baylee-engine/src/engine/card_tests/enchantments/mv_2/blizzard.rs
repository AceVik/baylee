//! `cards/enchantments/mv_2/blizzard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blizzard() -> CardIndex {
    card_index("698feaaa-2fff-4809-81d1-4caad44c6c0a")
}

/// Blizzard (698feaaa-2fff-4809-81d1-4caad44c6c0a) — {G}{G} Enchantment:
/// - "Cast this spell only if you control a snow land." (`Coverage::Partial`:
///   cast restriction not supported)
/// - "Cumulative upkeep {2}"
/// - "Creatures with flying don't untap during their controllers' untap steps."
#[test]
fn blizzard_keeps_flying_creatures_from_untapping_and_has_cumulative_upkeep_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), wall_of_swords(), llanowar_elves()])
        .hand(0, &[blizzard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, blizzard());
    pass_until(&mut engine, stack_is_empty);

    let blizz = on_battlefield(&engine, P0, blizzard()).expect("Blizzard seated");
    let flier = on_battlefield(&engine, P0, wall_of_swords()).expect("flying creature seated");
    let ground = on_battlefield(&engine, P0, llanowar_elves()).expect("ground Elf seated");

    // Tap both the flying creature and the non-flying creature.
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(flier)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(ground)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    // Advance to P1's main phase, then to P0's next upkeep (after P0's untap step).
    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );

    assert!(
        is_tapped(&engine, flier),
        "creature with flying did not untap during its controller's untap step"
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
