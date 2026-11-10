//! `cards/lands/tapland/snowfield_sinkhole.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snowfield Sinkhole is a Snow Land — Plains Swamp printing one line:
/// "{T}: Add {W} or {B}", and it enters tapped. The entry is played as a real
/// land drop rather than seated on the battlefield, because
/// `starting_battlefield` places a permanent with no entry at all and no
/// replacement effect would ever be asked. The tap is read a turn later —
/// a land that arrives tapped has no `{T}` to spend until its controller's
/// untap step (CR 502.3) — and both colours are named in turn, so "or" is
/// proven from both sides instead of from the menu alone.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn snowfield_sinkhole_enters_tapped_and_taps_for_white_or_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4211, forest())
        .hand(0, &[snowfield_sinkhole()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, snowfield_sinkhole());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and a real land drop is what asks"
    );
    let chars = engine
        .state()
        .object(land)
        .expect("the land is on the battlefield")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::LAND),
        "a land: {:?}",
        chars.types
    );
    assert!(
        chars.supertypes.contains(SupertypeSet::SNOW),
        "a *snow* land, which is what the printed supertype is for: {:?}",
        chars.supertypes
    );

    // A land that arrived tapped has no {T} left this turn, so the very line
    // under test is not on offer yet — the control for the activation below.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "the {{T}} was spent by the entry: {:?}",
        legal.abilities
    );

    // A whole turn cycle, so the untap step stands the Sinkhole back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran and the Sinkhole is standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so what the tap adds is the whole of the pool"
    );

    // Ability 0 is the printed "{T}: Add {W} or {B}", and its whole price is
    // its own tap, so it is offered without a single mana floating.
    activate(&mut engine, p0, snowfield_sinkhole(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two basic land types the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "{{W}} or {{B}}, and nothing else: {options:?}"
    );

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
        0,
        "the other colour of the pair was not made"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "{{T}} was the price");

    // The other half of the "or", on a fresh turn and a fresh pool.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied at the end of the step it was filled in (CR 500.5)"
    );
    activate(&mut engine, p0, snowfield_sinkhole(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "the second tap asks the same question, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&ManaColor::White),
        "and offers the same two colours: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the white half of the printed line, from the same land"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
