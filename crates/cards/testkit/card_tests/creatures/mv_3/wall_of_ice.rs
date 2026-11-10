//! `cards/creatures/mv_3/wall_of_ice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Ice prints `{2}{G}` for a 0/7 Wall carrying exactly one keyword:
/// "Defender (This creature can't attack.)". The board is built so that the
/// missing permanent in the attack declaration cannot be an accident of the
/// engine's timing: the turn is walked all the way round before the question
/// is asked, so the Wall is no longer summoning sick, and the untapped Elf
/// beside it under the same seat — offered by that very declaration — is the
/// control that says the offer was live. The `(0, 7)` body and the keyword are
/// both read off the battlefield object, the layer projection the card is
/// actually played through.
#[test]
fn wall_of_ice_lands_as_a_zero_seven_wall_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(913, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), quiet_creature()],
        )
        .hand(0, &[wall_of_ice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{G} off four Forests, with the Elf named as the printing kept back:
    // it is the control the declaration below is read against, and a creature
    // tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, wall_of_ice());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_ice()).expect("the Wall resolved");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, wall), (0, 7), "the body the card prints");
    assert!(
        types(&engine, wall).contains(TypeSet::CREATURE),
        "and it is a creature, so defender has something to restrict"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "Defender is on the permanent the card became"
    );
    assert!(
        !keywords(&engine, wall).contains(KeywordSet::FLYING),
        "and nothing the card does not print came with it"
    );

    // A whole turn cycle, so that CR 302.6 cannot account for the absence
    // below: a creature cast this turn may not attack either, and the printed
    // keyword is only readable once summoning sickness has worn off.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, elves), "the control is still standing");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack, so the offer is \
         live and this is the turn the Wall is not sick: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"This creature can't attack\" (CR 702.3b) — a 0/7 that is no longer \
         summoning sick is still not among the creatures a declaration may \
         name: {attackers:?}"
    );
}
