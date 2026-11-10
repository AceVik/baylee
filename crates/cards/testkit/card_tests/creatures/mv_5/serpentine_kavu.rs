//! `cards/creatures/mv_5/serpentine_kavu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serpentine Kavu is a `{4}{G}` 4/4 whose whole text is "`{R}`: This creature
/// gains haste until end of turn."
///
/// Nothing in the card file says the keyword actually reaches the permanent, so
/// the grant is played: five Forests pay `{4}{G}` down to an empty pool and the
/// one Mountain is held back as the source the `{R}` comes out of — the offer is
/// read with that red already floating, because `legal.abilities` is filtered
/// through `can_afford` and that reads the pool rather than the untapped land.
/// The Kavu then attacks on the turn it arrived, which a summoning-sick creature
/// may only do when something granted it haste (CR 302.6), and the keyword is
/// gone again a turn later, which is the printed duration and not a static.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn serpentine_kavu_pays_red_for_a_haste_that_lets_it_attack_once() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), mountain()],
        )
        .hand(0, &[serpentine_kavu()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests and not the Mountain: `{4}{G}` is exactly the five, and the
    // one land left standing is the source the `{R}` is about to come from.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, five green, and the Mountain still standing"
    );
    cast_with_floating(&mut engine, p0, serpentine_kavu());
    pass_until(&mut engine, stack_is_empty);

    let kavu = on_battlefield(&engine, p0, serpentine_kavu()).expect("the Kavu resolved");
    assert_eq!(pt(&engine, kavu), (4, 4), "the body the card prints");
    assert!(
        !keywords(&engine, kavu).contains(KeywordSet::HASTE),
        "and the card prints no keyword of its own"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the five to the last mana, so the offer below is not a gift"
    );

    // The `{R}` is a real price: the offer is filtered through `can_afford`,
    // which reads the pool and never the untapped land beside it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(kavu, 0)),
        "an empty pool pays no {{R}}, so the line is not offered at all: {:?}",
        legal.abilities
    );

    // A basic Mountain is the CR 305.6 shortcut, so it is named in
    // `mana_abilities` and carries no index to activate.
    let land = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Mountain was the one source kept untapped");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "one red, off the one Mountain on this board"
    );
    assert_eq!(pool.total(), 1, "and nothing else is floating with it");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kavu, 0)),
        "with {{R}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, serpentine_kavu(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(
        !keywords(&engine, kavu).contains(KeywordSet::HASTE),
        "granting a keyword is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, kavu).contains(KeywordSet::HASTE),
        "\"This creature gains haste until end of turn\""
    );

    // Haste means what it says: the Kavu arrived this turn, so the keyword is
    // the only way into the attack declaration (CR 302.6).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&kavu),
        "a 4/4 that arrived this turn may attack, which is the whole of the card: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(kavu, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    // Not `stack_is_empty`: the stack is already empty the moment attackers are
    // declared, so that predicate would stop the walk before the damage step.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        16,
        "four unblocked damage on the very turn the Kavu arrived"
    );

    // "until end of turn": the creature is still standing a turn later and the
    // keyword is not — a static or a permanent grant would still be on it here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, serpentine_kavu()).is_some(),
        "the Kavu survived the turn it attacked in"
    );
    assert!(
        !keywords(&engine, kavu).contains(KeywordSet::HASTE),
        "the grant lasted the turn it was made in and no longer"
    );
}
