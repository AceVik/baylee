//! `cards/lands/pain/caves_of_koilos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Caves of Koilos prints two mana lines and only the second one costs
/// anything: `{T}: Add {C}`, and `{T}: Add {W} or {B}. This land deals 1
/// damage to you.` Both are pressed here in the order the untap step forces
/// — the colorless line on the turn the land is played, the colored one a
/// turn cycle later, which is also the control that the second activation is
/// available because the step stood the land back up and not because nothing
/// ever tapped it. The choice the colored line raises is exactly white and
/// black and nothing else, and the damage lands on the activating player's
/// life alone.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn caves_of_koilos_adds_colorless_for_free_and_charges_one_life_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[caves_of_koilos()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    play_land(&mut engine, p0, caves_of_koilos());
    let caves = on_battlefield(&engine, p0, caves_of_koilos()).expect("the land is on the table");
    assert!(
        !is_tapped(&engine, caves),
        "nothing on the card brings it in tapped"
    );

    // Neither line asks for mana, so both are offered on an empty pool — and
    // both are `abilities` entries rather than `mana_abilities`, because a
    // nonbasic land's printed `{T}: Add …` has an index to name (#159).
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the land's controller holds priority");
    assert!(
        legal.abilities.contains(&(caves, 0)) && legal.abilities.contains(&(caves, 1)),
        "the two printed lines are the whole of what it offers: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before either of them is taken"
    );

    // {T}: Add {C} — ability 0, the line with no rider on it.
    activate(&mut engine, p0, caves_of_koilos(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "one colorless for the tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so it has already resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"This land deals 1 damage to you\" is printed on the other line, \
         and taking {{C}} may not cost it"
    );
    assert!(
        is_tapped(&engine, caves),
        "{{T}} is the price of either line"
    );

    // A turn cycle, so that the untap step — and nothing else — stands the
    // land back up.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        is_tapped(&engine, caves),
        "the opponent's turn does not untap it"
    );
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the land's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, caves),
        "and the untap step ran, which is the only reason the activation \
         below is available at all"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{C}} emptied with the phase that made it (CR 500.5), so the \
         pool read below belongs to the second line alone"
    );

    // {T}: Add {W} or {B}. This land deals 1 damage to you. — ability 1.
    activate(&mut engine, p0, caves_of_koilos(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("two colours is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "\"{{W}} or {{B}}\": both are on the menu, {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else is — neither a third colour nor the colorless the \
         other line makes: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two colours it offered");
    // The colour is the only choice the activation makes of its own; if the
    // damage clause raises a question on the way out it is answered here, out
    // of the list that question itself published.
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = answer_one(&engine).unwrap_or_else(|rest| {
            panic!("the damage line asked something unanswerable: {rest:?}")
        });
        engine
            .apply(player, action)
            .expect("the answer came out of the question");
    }

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "one mana off one tap, so the colour not named is not there"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "still a mana ability, whatever the damage clause does beside it"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is the controller: no other seat pays for this mana"
    );
    assert!(is_tapped(&engine, caves), "one tap, one line");
}
