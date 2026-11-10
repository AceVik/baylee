//! `cards/creatures/mv_4/advanced_hoverguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Advanced Hoverguard is a `{3}{U}` 2/2 Drone with flying and one activated
/// line: "`{U}`: This creature gains shroud until end of turn." The ability is
/// only offered once its `{U}` is *in the pool* — `can_afford` reads the pool
/// and not the untapped lands — so the fifth Island is deliberately kept back
/// out of the cast, because `tap_all_mana` would have spent every source on the
/// Drone itself and the activation would then be refused for a reason that has
/// nothing to do with shroud. The grant is read off the projected keywords, and
/// read again a turn later: "until end of turn" is half of what the card prints.
#[test]
fn advanced_hoverguard_flies_and_trades_one_blue_for_shroud_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[advanced_hoverguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(
        islands.len(),
        5,
        "five Islands: four to cast with, one to keep"
    );
    tap_mana_except(&mut engine, p0, islands[4]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands tapped and the fifth kept back for the ability"
    );

    cast_with_floating(&mut engine, p0, advanced_hoverguard());
    pass_until(&mut engine, stack_is_empty);
    let drone = on_battlefield(&engine, p0, advanced_hoverguard()).expect("the Drone resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} came out of the pool the four Islands filled"
    );
    assert_eq!(pt(&engine, drone), (2, 2), "the body the card prints");
    let printed = keywords(&engine, drone);
    assert!(printed.contains(KeywordSet::FLYING), "Flying, as printed");
    assert!(
        !printed.contains(KeywordSet::SHROUD),
        "the Drone carries no shroud of its own — the {{U}} ability is what buys it"
    );

    // One source is left standing and it is the only one: `tap_all_mana` takes
    // every mana ability whose whole price is its own {T}, which on this board
    // is that single untapped Island and nothing else.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the kept Island, and exactly the {{U}} the ability charges"
    );

    activate(&mut engine, p0, advanced_hoverguard(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} is paid as the ability is activated"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining a keyword is no mana ability"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, drone).contains(KeywordSet::SHROUD),
        "\"This creature gains shroud until end of turn\""
    );
    assert!(
        keywords(&engine, drone).contains(KeywordSet::FLYING),
        "and the printed flying is untouched by the grant"
    );

    // Until end of turn, and no longer: the other seat's main phase is past the
    // cleanup step that ends the duration.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, drone).contains(KeywordSet::SHROUD),
        "the grant is temporary — a shroud that outlived its turn would be a \
         different card"
    );
}
