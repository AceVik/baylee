//! `cards/lands/horizon/nurturing_peatland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nurturing Peatland — Land, `Coverage::Implemented`. It prints two lines:
/// "{T}, Pay 1 life: Add {B} or {G}", where the life is a real cost and the
/// two colours are a question the card asks rather than a default the engine
/// picks, and "{1}, {T}, Sacrifice this land: Draw a card", which the first
/// activation's own tap makes unpayable until the untap step has stood the
/// land back up. Both are played in one game, so the untap, the life, the
/// colour and the sacrifice are all read off the same permanent.
#[test]
#[allow(clippy::too_many_lines)] // both printed lines, played in one game
fn nurturing_peatland_pays_a_life_for_a_colour_and_then_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .life(0, 20)
        .hand(0, &[nurturing_peatland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, nurturing_peatland());
    assert!(
        !is_tapped(&engine, land),
        "a Peatland enters untapped, so its {{T}} is live the turn it is played"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before it does anything"
    );

    // `{T}, Pay 1 life: Add {B} or {G}`. The whole price is the land's own tap
    // plus one life, so nothing is tapped beforehand and the empty pool makes
    // "one green and nothing else" an exact claim.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "a printed mana ability is an ordinary `(source, index)` entry and not \
         the CR 305.6 shortcut: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, nurturing_peatland(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activates names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "the two the card names, and not \"any colour\": {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two the ability offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off one tap"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is a cost: twenty down to nineteen as the ability is \
         activated"
    );
    assert!(
        is_tapped(&engine, land),
        "and the land's own tap is the rest of the price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );

    // The land is tapped and `{1}, {T}, Sacrifice this land` needs it up, so
    // the walk goes a whole round: the Forest is tapped only after the untap
    // step has stood the Peatland back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step put the land back up, which is the only reason the \
         second line is payable at all"
    );

    // `legal.abilities` is filtered by `can_afford`, which reads the pool, so
    // the {1} is made before the offer is read.
    tap_all_mana_but(&mut engine, p0, Some(nurturing_peatland()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest is one green mana, and the Peatland was kept back untapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "{{1}} and a tap are both affordable, so the second line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, nurturing_peatland(), 1);
    assert!(
        !stack_is_empty(&engine),
        "\"Draw a card\" is no mana ability, so this one waits on the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, nurturing_peatland()).is_none(),
        "the land sacrificed itself to pay"
    );
    assert!(
        in_graveyard(&engine, p0, nurturing_peatland()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} came out of the pool"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "into the hand of the seat that sacrificed the land — a card gone from \
         the library without arriving in hand would satisfy the count above"
    );
}
