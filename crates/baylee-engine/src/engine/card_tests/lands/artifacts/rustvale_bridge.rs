//! `cards/lands/artifacts/rustvale_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rustvale Bridge — Artifact Land, `Coverage::Implemented`: it enters
/// tapped, it is indestructible, and it prints `{T}: Add {R} or {W}`.
///
/// All three lines are played rather than read. The land has to be *played*,
/// because `SeatSpec::starting_battlefield` places a permanent with
/// `Cause::Setup` — a placement, not an entry — so no enter modifier is
/// consulted there and a swept Bridge would read untapped. A destroy effect
/// is aimed at it from across the table, since indestructible looks the same
/// on a permanent nothing ever tried to kill; and the mana line is only
/// reachable after a real untap step, because a land that entered tapped
/// gives nothing in the turn it arrived.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn rustvale_bridge_enters_tapped_survives_vindicate_and_taps_for_red_or_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4211, forest())
        .battlefield(1, &[plains(), plains(), swamp()])
        .hand(0, &[rustvale_bridge()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bridge = play_land(&mut engine, p0, rustvale_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "`EnterModifier::Tapped`: a placement through `starting_battlefield` \
         would have hidden exactly this reading"
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "the type line is artifact *and* land"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "and the printed indestructible reaches the permanent"
    );

    // The plainest thing that would take an ordinary artifact land off the
    // table: "{1}{W}{B}: Destroy target permanent", cast from the seat whose
    // board is both halves of that cost.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&bridge),
        "\"target permanent\" reaches across the table: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .expect("the Bridge was one of the options the effect enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        on_battlefield(&engine, p0, rustvale_bridge()),
        Some(bridge),
        "CR 702.12b: an indestructible permanent is not destroyed — and it is \
         the same object still, not a replacement the walk stumbled over"
    );
    assert!(
        in_graveyard(&engine, p0, rustvale_bridge()).is_none(),
        "so nothing about it reached a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "and Vindicate did resolve: it is in its caster's graveyard"
    );

    // Its arrival turn gave it nothing, so the mana line waits for its
    // controller's own untap step (CR 502.3).
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step is what stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so the one mana below can only be the Bridge's"
    );

    // The whole price of the ability is its own {T}, which is why it is
    // offered without a mana pool behind it.
    activate(&mut engine, p0, rustvale_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "both halves of the printed line: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and two is the whole menu — a fixed list, not a reading of the lands"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not the other half of the same ability"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(is_tapped(&engine, bridge), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
