//! `cards/enchantments/mv_3/arnjlot_s_ascent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arnjlot_s_ascent() -> CardIndex {
    card_index("f2cedb38-12b0-4489-a62b-9b6a3776d695")
}

/// Arnjlot's Ascent (f2cedb38-12b0-4489-a62b-9b6a3776d695) — {1}{U}{U} Enchantment:
/// - "Cumulative upkeep {U}"
/// - "{1}: Target creature gains flying until end of turn."
#[test]
fn arnjlot_s_ascent_grants_flying_until_end_of_turn_and_has_cumulative_upkeep_blue() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), gray_ogre()])
        .hand(0, &[arnjlot_s_ascent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, arnjlot_s_ascent());
    pass_until(&mut engine, stack_is_empty);

    let ascent = on_battlefield(&engine, P0, arnjlot_s_ascent()).expect("Ascent seated");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Gray Ogre seated");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));

    activate(&mut engine, P0, arnjlot_s_ascent(), 1);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, ogre).contains(KeywordSet::FLYING),
        "{{1}}: Target creature gains flying until end of turn"
    );

    // End of turn removes flying.
    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));

    // On P0's next upkeep, cumulative upkeep puts an age counter on Ascent and
    // asks whether to pay {U}.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert_eq!(
        counters_on(&engine, ascent, baylee_cards_dsl::counters::AGE),
        1
    );

    // Decline to pay -> Ascent is sacrificed.
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline cumulative upkeep");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, arnjlot_s_ascent()).is_none());
    assert!(in_graveyard(&engine, P0, arnjlot_s_ascent()).is_some());
}
