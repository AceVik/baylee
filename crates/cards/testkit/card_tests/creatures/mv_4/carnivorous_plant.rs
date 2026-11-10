//! `cards/creatures/mv_4/carnivorous_plant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Carnivorous Plant — {3}{G} — Creature — Plant Wall, 4/5, and its whole
/// printed text is one keyword: "Defender". The body and the keyword are read
/// off the permanent it becomes, but the sentence that matters is a rule about
/// the combat step, so the game is walked to an attack declaration where the
/// Plant is untapped and *not* summoning sick — a creature cast this turn is
/// held out of the offer by CR 302.6 whatever it prints, which would make the
/// same assertion true of a card with no text at all. The Elf beside it is the
/// control: same seat, same board, no defender, and the declaration names it.
#[test]
fn carnivorous_plant_defends_and_is_never_offered_as_an_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), quiet_creature()],
        )
        .hand(0, &[carnivorous_plant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {3}{G} off the four Forests, with the Elf named as the source kept back:
    // it is the control the attack declaration below needs, and a creature
    // tapped for mana may not attack either.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green, and the Elf still standing"
    );
    cast_with_floating(&mut engine, p0, carnivorous_plant());
    pass_until(&mut engine, stack_is_empty);

    let plant = on_battlefield(&engine, p0, carnivorous_plant()).expect("the Plant resolved");
    assert_eq!(pt(&engine, plant), (4, 5), "the body the card prints");
    assert!(
        keywords(&engine, plant).contains(KeywordSet::DEFENDER),
        "and the one keyword it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{G}} is four of the four the Forests made"
    );

    // A whole turn cycle first. The Plant arrived this turn, so summoning
    // sickness (CR 302.6) alone would keep it out of the declaration — the
    // same exclusion a card with no text would get, and no evidence at all
    // about the word "Defender".
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Plant's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, plant),
        "the untap step stood the Plant back up, so nothing but the keyword is \
         left to keep it out of combat"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is still out");
    assert!(
        attackers.contains(&elf),
        "an untapped creature with no text of its own is offered: {attackers:?}"
    );
    assert!(
        !attackers.contains(&plant),
        "\"Defender\" — an untapped 4/5 that could attack for four is not \
         offered at all: {attackers:?}"
    );
}
