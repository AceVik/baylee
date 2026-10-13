//! `cards/lands/utility/shizo_death_s_storehouse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shizo, Death's Storehouse is a **legendary** land, and the type line is
/// the part worth playing: CR 704.5j applies the legend rule to any
/// permanent with the supertype, not only to creatures, and a land written
/// as a plain `Land` would sit on the battlefield in pairs with every other
/// reading of it correct.
///
/// So two are played, one turn apart, and the survivor is the one its
/// controller keeps. The mana ability is asserted on that survivor, because
/// a land that lost the legend rule and a land that never made mana are
/// different failures and the test should not be able to confuse them.
#[test]
fn shizo_is_legendary_and_the_survivor_still_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[shizo_death_s_storehouse(), shizo_death_s_storehouse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let first = play_land(&mut engine, p0, shizo_death_s_storehouse());
    assert!(
        !entered_tapped(&engine, first),
        "Shizo prints no enters-tapped clause"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, shizo_death_s_storehouse()).len(),
        1,
        "one is one"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    let second = play_land(&mut engine, p0, shizo_death_s_storehouse());

    // CR 704.5j is a state-based action that asks rather than picks: the
    // controller keeps one. Keeping the newcomer is the answer that makes
    // the assertion below about the rule and not about which object the
    // engine happened to leave alone.
    let Pending::LegendChoice { player, options } = engine.pending().clone() else {
        panic!(
            "two legendary permanents ask the legend rule, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their controller chooses");
    assert_eq!(options.len(), 2, "both Shizos are on the offer");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the one it named was one of the two");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let standing = all_on_battlefield(&engine, p0, shizo_death_s_storehouse());
    assert_eq!(
        standing.len(),
        1,
        "the legend rule (CR 704.5j) is about permanents and not about creatures"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and the one that lost went to its owner's graveyard"
    );

    activate(&mut engine, p0, shizo_death_s_storehouse(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{T}}: Add {{B}} on the one that is still there"
    );
}

#[test]
fn shizo_grants_fear_with_the_printed_blocking_exceptions_until_end_of_turn() {
    use crate::engine::card_tests::legends_kit as kit;
    let emry = card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507");
    let mut engine = kit::game(
        920,
        &[shizo_death_s_storehouse(), emry],
        &[llanowar_elves(), festering_goblin(), ornithopter()],
        &[],
        &[],
    );
    let target = kit::obj(&engine, kit::P0, emry);
    kit::float(&mut engine, kit::P0, &[(ManaColor::Black, 1)]);
    kit::use_ability(
        &mut engine,
        kit::P0,
        shizo_death_s_storehouse(),
        1,
        &[target],
        &[],
    );
    for (card, allowed) in [
        (llanowar_elves(), false),
        (festering_goblin(), true),
        (ornithopter(), true),
    ] {
        let blocker = kit::obj(&engine, kit::P1, card);
        assert_eq!(
            crate::combat::can_block(engine.state(), kit::P1, blocker, target),
            allowed
        );
    }
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(!keywords(&engine, target).contains(KeywordSet::FEAR));
}

/// "Target legendary creature gains fear until end of turn." The menu holds
/// every legendary creature, whoever controls it, and no creature that is not
/// legendary, on either side.
#[test]
fn shizo_offers_legendary_creatures_only() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let emry = card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507");
    let mut engine = kit::game(
        957,
        &[shizo_death_s_storehouse(), emry, llanowar_elves()],
        &[index::BIRGI_GOD_OF_STORYTELLING, festering_goblin()],
        &[],
        &[],
    );
    let (own_legend, own_plain) = (
        kit::obj(&engine, kit::P0, emry),
        kit::obj(&engine, kit::P0, llanowar_elves()),
    );
    let their_legend = kit::obj(&engine, kit::P1, index::BIRGI_GOD_OF_STORYTELLING);
    let their_plain = kit::obj(&engine, kit::P1, festering_goblin());
    kit::float(&mut engine, kit::P0, &[(ManaColor::Black, 1)]);
    activate(&mut engine, kit::P0, shizo_death_s_storehouse(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&own_legend),
        "a legendary creature of mine"
    );
    assert!(
        options.contains(&their_legend),
        "no controller is printed: an opposing legendary creature too"
    );
    assert!(
        !options.contains(&own_plain),
        "Llanowar Elves is not legendary"
    );
    assert!(
        !options.contains(&their_plain),
        "Festering Goblin is not legendary"
    );
}
