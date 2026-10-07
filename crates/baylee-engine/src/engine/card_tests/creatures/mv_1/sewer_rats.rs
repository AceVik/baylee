//! `cards/creatures/mv_1/sewer_rats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sewer Rats prints one ability and one restriction on it: "{B}, Pay 1
/// life: This creature gets +1/+0 until end of turn. Activate no more than
/// three times each turn." Three activations off four Swamps is the whole of
/// the card in one main phase — the pool never empties (CR 500.5), so the
/// pump, the mana and the life are all read off one board — and the fourth
/// activation is then read off the offer while a `{B}` and seventeen life
/// are still sitting there. That leftover is what makes the refusal the
/// printed cap rather than `can_afford` declining a price nobody can pay.
#[test]
fn sewer_rats_grows_three_times_a_turn_and_then_the_offer_is_gone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), sewer_rats()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rats = on_battlefield(&engine, p0, sewer_rats()).expect("the Rats are on the table");
    assert_eq!(
        pt(&engine, rats),
        (1, 1),
        "a printed 1/1 before anything is paid"
    );

    // Four Swamps and not three: three activations then leave one black in
    // the pool, which is what the assertion at the foot of this test needs.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4,
        "four Swamps are four {{B}}, and the Rats make none of their own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(rats, 0)),
        "the one line the card prints is offered with the {{B}} already floating: {:?}",
        legal.abilities
    );

    for pump in 1i16..=3 {
        activate(&mut engine, p0, sewer_rats(), 0);
        pass_until(&mut engine, |e| at_rest(e, p0));
        assert_eq!(
            pt(&engine, rats),
            (1 + pump, 1),
            "the {pump}. activation is +1/+0, and the toughness it prints is untouched"
        );
    }

    assert_eq!(
        engine.state().players[0].life,
        17,
        "1 life paid for each of the three activations, and no more"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "and one of the four {{B}} is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == rats),
        "\"activate no more than three times each turn\" — the fourth \
         activation is not offered even though the {{B}} is in the pool and \
         the life is there to pay it. A refusal for want of mana would read \
         the same on a board that had spent everything"
    );
}
