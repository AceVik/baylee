//! `cards/creatures/mv_4/lightning_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lightning Elemental — `{3}{R}` — Creature — Elemental, 4/1: "Haste."
///
/// The whole printed card is one keyword, and the only place a board can read
/// it back is the attack declaration: summoning sickness (CR 302.6) keeps a
/// creature that came under its controller's control this turn off
/// `ChooseAttackers`, so the offer *is* the claim. The Festering Goblin cast
/// in the same main phase is the control — untapped, with no defender in its
/// way, and simply sick — and the four damage that lands on the opponent
/// afterwards says the 4/1 body really arrived rather than a label.
#[test]
fn lightning_elemental_attacks_the_turn_it_arrives_where_a_fresh_goblin_cannot() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), swamp()],
        )
        .hand(0, &[lightning_elemental(), festering_goblin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Mountains pay `{3}{R}` and nothing else. The Swamp is named as the
    // printing kept back because its `{B}` is the control creature's price, and
    // a pool that had already spent it would leave the Goblin uncasteable.
    tap_all_mana_but(&mut engine, p0, Some(swamp()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, four red"
    );
    cast_with_floating(&mut engine, p0, lightning_elemental());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "`{{3}}{{R}}` took every red the Mountains made"
    );

    // The control, cast in the same main phase off the Swamp: a creature with
    // no keyword at all that entered under the same controller on the same turn.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Swamp, and the Elemental beside it prints no mana ability"
    );
    cast_with_floating(&mut engine, p0, festering_goblin());
    pass_until(&mut engine, stack_is_empty);

    let elemental =
        on_battlefield(&engine, p0, lightning_elemental()).expect("the Elemental landed");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin landed");
    assert_eq!(pt(&engine, elemental), (4, 1), "the body the card prints");
    assert!(
        keywords(&engine, elemental).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );

    // Both arrived this turn, both are untapped, and neither has anything
    // holding it back — so the keyword is the only thing that can tell the two
    // of them apart in the offer below.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(
        player, p0,
        "the seat that cast them is the seat that declares"
    );
    assert!(
        attackers.contains(&elemental),
        "haste (CR 702.10c): the Elemental may attack the turn it came under \
         its controller's control, so the offer names it: {attackers:?}"
    );
    assert!(
        !attackers.contains(&goblin),
        "and the Goblin cast in the same main phase may not — untapped, with \
         nobody in its way, and sick (CR 302.6): {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elemental, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // Not `stack_is_empty`: the stack is already empty the moment attackers are
    // declared, while the four damage is dealt in the combat damage step
    // (CR 510.2) — which is why the walk goes as far as the end step.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        16,
        "a 4/1 nobody blocked: four damage in the very turn it arrived"
    );
    assert!(
        !is_tapped(&engine, goblin),
        "the control creature stayed home, untapped and untouched, so the \
         offer above was about sickness and not about a board that moved"
    );
    assert!(
        on_battlefield(&engine, p0, lightning_elemental()).is_some(),
        "and the Elemental is still standing: it attacked unblocked"
    );
}

/// A top card that is not a land asks nothing and goes to the hand; and the
/// trigger is "this creature **or another Elemental** you control": a
/// Lightning Elemental entering under a Reef looks as well.
#[test]
fn risen_reef_sends_a_nonland_to_the_hand_when_another_elemental_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(
            0,
            &[risen_reef(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lightning_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    cast_from_hand(&mut engine, p0, lightning_elemental());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !matches!(engine.pending(), Pending::ChooseCards { .. }),
        "a nonland top card is nothing to decide"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "the Elemental's arrival put the top card into the hand"
    );
}
