//! `cards/creatures/mv_3/raging_kavu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging Kavu is {1}{R}{G} for a 3/1 Kavu with flash and haste, and each
/// keyword is read in the window the other one does not open. Haste is the
/// attack declaration of the turn the 3/1 arrived, where an untapped Llanowar
/// Elves that entered the very same main phase is *not* offered — the two
/// creatures differ in nothing but the printed word. Flash is the end step's
/// own priority round, where the same sources still standing and the same
/// empty stack leave the Elves held in hand uncastable and the second Kavu
/// castable.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn raging_kavu_flashes_in_on_an_end_step_and_attacks_the_turn_it_arrives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(
            0,
            &[
                raging_kavu(),
                raging_kavu(),
                quiet_creature(),
                quiet_creature(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four of the eight lands, and only those: the four left standing are the
    // mana the end step's cast is measured against, and a land tapped here is
    // a land that cannot pay there.
    let mountains = all_on_battlefield(&engine, p0, mountain());
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        (mountains.len(), forests.len()),
        (4, 4),
        "four Mountains and four Forests, which is one {{G}} and two casts of \
         {{1}}{{R}}{{G}}"
    );
    let spent = [mountains[0], mountains[1], forests[0], forests[1]];
    tap_mana_where(&mut engine, p0, |id| spent.contains(&id));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Mountains and two Forests tapped, and the rest of the board still \
         standing"
    );

    // A main phase takes any creature, so both of these are in the offer with
    // the very same mana in the pool. Everything below turns on that.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let kavu_in_hand = in_hand(&engine, p0, raging_kavu()).expect("a Kavu is in hand");
    let elves_in_hand = in_hand(&engine, p0, quiet_creature()).expect("an Elves is in hand");
    assert!(
        legal.castable.contains(&kavu_in_hand) && legal.castable.contains(&elves_in_hand),
        "{{1}}{{R}}{{G}} and {{G}} are both payable here: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let sick = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves resolved");
    cast_with_floating(&mut engine, p0, raging_kavu());

    // Haste, and the control that makes it a reading rather than an
    // assumption: both creatures entered this main phase and both are
    // untapped, so the printed word is the only difference between them.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let kavu = on_battlefield(&engine, p0, raging_kavu()).expect("the first Kavu resolved");
    assert_eq!(pt(&engine, kavu), (3, 1), "the body the card prints");
    assert!(!is_tapped(&engine, sick), "the control is untapped");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&kavu),
        "haste: a 3/1 that arrived a moment ago may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&sick),
        "and the Elves that arrived in the same main phase may not — summoning \
         sickness (CR 508.1a) is what the printed haste buys off: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(kavu, Defender::Player(p1))],
            },
        )
        .expect("the offer it was just read out of");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the 3/1 connected: three damage to a seat that has nothing to block with"
    );

    // Flash: an end step is a priority round where a spell with the keyword
    // may be cast, and the sources the first main phase left standing are
    // still standing to pay with — the four lands, and **not** the Elves
    // beside them. The Elves arrived this turn and has no haste, so CR 302.6
    // keeps its {{T}} out of this pool exactly as it kept the creature out of
    // the attack above: one card, the same rule, read twice.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the two Mountains and two Forests held back, and the Elves that \
         arrived this turn cannot tap for its own {{G}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the end step hands priority back, got {:?}",
            engine.pending()
        )
    };
    let second_kavu =
        in_hand(&engine, p0, raging_kavu()).expect("the second Kavu is still in hand");
    let held_elves = in_hand(&engine, p0, quiet_creature()).expect("an Elves is still in hand");
    assert!(
        legal.castable.contains(&second_kavu),
        "flash: a creature costing {{1}}{{R}}{{G}} is castable in an end step: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&held_elves),
        "control: the Elves is in the same hand, is payable out of the same \
         pool and has the same empty stack, and is not castable here — an end \
         step is no main phase: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, raging_kavu());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        all_on_battlefield(&engine, p0, raging_kavu()).len(),
        2,
        "and it resolved: both copies are on the battlefield"
    );
}
