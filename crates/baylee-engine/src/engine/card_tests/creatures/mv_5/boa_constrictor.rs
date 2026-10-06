//! `cards/creatures/mv_5/boa_constrictor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boa Constrictor is a `{4}{G}` 3/3 Snake whose whole printed text is
/// "{T}: This creature gets +3/+3 until end of turn."
///
/// The creature is cast rather than seated, which is what makes the `{T}` have
/// to be earned: the turn it lands it is summoning sick (CR 302.6) and the only
/// line the card prints is nowhere in the offer, and only after a full turn
/// cycle — untap step included — does pressing it turn the body into a 6/6.
/// The Elf beside it is the control that the pump is `Filter::This` and not a
/// board sweep, and a second turn cycle proves the printed duration: the grant
/// is gone while the creature is still standing, so what expired was the bonus
/// and not the body.
#[test]
fn boa_constrictor_pumps_only_itself_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_creature(),
            ],
        )
        .hand(0, &[boa_constrictor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, boa_constrictor());
    pass_until(&mut engine, stack_is_empty);
    let boa = on_battlefield(&engine, p0, boa_constrictor()).expect("the Boa resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert_eq!(pt(&engine, boa), (3, 3), "the body the card prints");
    assert_eq!(pt(&engine, elf), (1, 1), "and nothing has pumped the Elf");

    // Summoning sickness: a creature that arrived this turn may not pay a `{T}`
    // (CR 302.6), so the card offers nothing at all while it is standing there
    // untapped — the half a test that seats its creature would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == boa),
        "the Boa arrived this turn, so its {{T}} is unpayable: {:?}",
        legal.abilities
    );

    // A whole turn cycle: the untap step stands the Boa back up and the one
    // line the card prints becomes a price the seat may pay.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, boa), "the untap step stood it back up");

    activate(&mut engine, p0, boa_constrictor(), 0);
    assert!(
        is_tapped(&engine, boa),
        "{{T}} is the whole price and is paid as the ability is activated"
    );
    assert_eq!(
        pt(&engine, boa),
        (3, 3),
        "CR 601.2h is the last step: the pump has not resolved yet"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, boa),
        (6, 6),
        "+3/+3 on the creature that activated the ability"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "`Filter::This` is the source and no other creature on the board"
    );

    // "until end of turn": another cycle and the grant is gone, which is what
    // tells a duration from a static or from a counter that never expires.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, boa),
        (3, 3),
        "the pump lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, boa_constrictor()).is_some(),
        "and the creature is still standing, so the bonus left rather than the body"
    );
}
