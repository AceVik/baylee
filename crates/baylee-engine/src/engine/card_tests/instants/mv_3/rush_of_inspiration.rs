//! `cards/instants/mv_3/rush_of_inspiration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Without energy the spell draws two, then discards one at random.
#[test]
fn rush_of_inspiration_draws_two_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rush_of_inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, rush_of_inspiration());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        hand_after, hand_before,
        "the spell and random discard cost two cards, and two are drawn"
    );
    assert!(
        in_graveyard(&engine, p0, rush_of_inspiration()).is_some(),
        "resolved spell is in the graveyard"
    );
}

/// "{T}: Add {U} or {R}." — the back face asks which of the two, offers
/// exactly those two colours, and adds one mana of the colour chosen.
#[test]
fn crackling_falls_offers_blue_or_red_and_adds_the_one_chosen() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(rush_of_inspiration(), 1).expect("plays as Crackling Falls");
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" pins the entry"
    );
    engine.dev_state_mut(p0).unwrap().set_tapped(land, false);
    engine.refresh_offer();

    activate(&mut engine, p0, rush_of_inspiration(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(
        options.len(),
        2,
        "\"{{U}} or {{R}}\" offers exactly two colours: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the colours are blue and red: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red is on offer");
    assert!(is_tapped(&engine, land), "{{T}} is the cost of the ability");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the chosen colour arrives"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the colour not chosen does not"
    );
    assert_eq!(pool.total(), 1, "one mana in all");
}
