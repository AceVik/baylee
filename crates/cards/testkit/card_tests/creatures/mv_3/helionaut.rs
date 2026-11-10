//! `cards/creatures/mv_3/helionaut.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Helionaut prints two lines — flying, and "{1}, {T}: Add one mana of any
/// color." — and the scenario plays both off one board: the seated Soldier's
/// mana is made first, so four white Plains read as three white and one named
/// color afterwards, and the copy in hand is then cast for its {2}{W} out of
/// the mana the ability left floating. The {1} is paid from the pool and not
/// from an untapped land, which is what makes "one white less" an exact
/// statement about the price; the body and the keyword are read on both
/// objects, because a cast copy and a seated copy are the same card only if
/// nothing about either changed.
#[test]
fn helionaut_flies_and_taps_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), helionaut()])
        .hand(0, &[helionaut()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let seated = on_battlefield(&engine, p0, helionaut()).expect("the seated Soldier stands");
    assert_eq!(pt(&engine, seated), (1, 2), "the body the card prints");
    assert!(
        keywords(&engine, seated).contains(KeywordSet::FLYING),
        "flying is printed on the card and projected onto the permanent"
    );
    assert!(
        types(&engine, seated).contains(TypeSet::CREATURE),
        "a Human Soldier is a creature"
    );

    // The price of the mana line is {1} *and* its own tap, so `tap_all_mana`
    // does not press it (#159): only the four Plains go, which leaves the
    // Soldier standing to pay the {T} below.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, and the Soldier's own line is not a route the helper takes"
    );
    assert!(
        !is_tapped(&engine, seated),
        "so the source is still untapped"
    );

    activate(&mut engine, p0, helionaut(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colorless is no colour at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        3,
        "one of the four white paid the {{1}}, so exactly three are left"
    );
    assert_eq!(pool.total(), 4, "one mana spent and one mana added");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, seated),
        "{{T}} is the other half of the price"
    );

    // And the card is castable for what it prints: {2}{W} out of the pool the
    // ability left behind, arriving as the same 1/2 flier the seated copy is.
    cast_with_floating(&mut engine, p0, helionaut());
    pass_until(&mut engine, stack_is_empty);
    let soldiers = all_on_battlefield(&engine, p0, helionaut());
    assert_eq!(
        soldiers.len(),
        2,
        "the second Soldier resolved onto the table"
    );
    let summoned = *soldiers
        .iter()
        .find(|id| **id != seated)
        .expect("the object that was not the seated one");
    assert_eq!(pt(&engine, summoned), (1, 2), "and is the card it prints");
    assert!(
        keywords(&engine, summoned).contains(KeywordSet::FLYING),
        "with flying, like the copy beside it"
    );
    assert!(
        is_tapped(&engine, seated),
        "the ability's tap stands after the cast"
    );
    assert!(
        !is_tapped(&engine, summoned),
        "the cast spent mana and nothing else: the new Soldier entered untapped"
    );
}
