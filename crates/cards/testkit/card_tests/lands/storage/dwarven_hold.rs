//! `cards/lands/storage/dwarven_hold.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The cycle is one card printed five times, and the only thing that
/// differs is the colour it banks.
///
/// Worth playing all five rather than reading the files: they are generated
/// from the printed text by one rule, so a colour read off the wrong
/// sentence would be identical in every file and invisible to any amount of
/// re-reading. Each land is kept tapped for one upkeep and then spent.
#[test]
fn every_land_of_the_cycle_pays_out_in_its_own_colour() {
    let p0 = PlayerId::new(0);
    for (seed, card, color) in [
        (885, bottomless_vault(), ManaColor::Black),
        (886, dwarven_hold(), ManaColor::Red),
        (887, hollow_trees(), ManaColor::Green),
        (888, icatian_store(), ManaColor::White),
        (889, sand_silos(), ManaColor::Blue),
    ] {
        let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let land = play_land(&mut engine, p0, card);

        let (_, options) = walk_to_the_untap_question(&mut engine);
        assert_eq!(options, vec![land], "{color:?}");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .expect("leaving it tapped");
        on_to_this_turn_s_main(&mut engine, p0);
        assert_eq!(
            counters_on(&engine, land, counters::STORAGE),
            1,
            "{color:?} banked one at its upkeep"
        );

        let (_, options) = walk_to_the_untap_question(&mut engine);
        assert_eq!(options, vec![land], "{color:?}");
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("untapping it");
        on_to_this_turn_s_main(&mut engine, p0);

        spend_storage(&mut engine, p0, land, 2, 1);
        assert_eq!(
            engine.state().players[0].mana_pool.available(color),
            1,
            "one counter, one {color:?}"
        );
    }
}
