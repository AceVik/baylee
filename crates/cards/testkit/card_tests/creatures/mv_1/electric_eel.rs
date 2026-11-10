//! `cards/creatures/mv_1/electric_eel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Electric Eel is a {U} 1/1 whose whole rules text is two shocks aimed at
/// **its own controller**: an enters trigger, and an activated {R}{R} that
/// also pumps it by +2/+0 until end of turn. One first main phase reads both
/// halves off one board: one Island and two Mountains pay for the Eel and
/// leave exactly {R}{R} in the pool (CR 500.5 keeps it there for the rest of
/// the phase), so the activated ability is a real payment rather than a
/// label, the body proves the pump landed, and the opponent's untouched 20 is
/// the control — `PlayerRel::You` on both sentences must not cross the table,
/// which a card that had read `Opponent` would satisfy just as well if only
/// the caster's life were checked.
#[test]
fn electric_eel_shocks_its_controller_on_entry_and_again_when_it_pumps_itself() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4517, forest())
        .battlefield(0, &[island(), mountain(), mountain()])
        .hand(0, &[electric_eel()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `cast_from_hand` takes every mana ability whose whole price is its own
    // {T}, so the Island and both Mountains go: {U} pays for a 1/1 and the
    // two red are left floating, which is exactly the {R}{R} below.
    cast_from_hand(&mut engine, p0, electric_eel());
    pass_until(&mut engine, stack_is_empty);
    let eel = on_battlefield(&engine, p0, electric_eel()).expect("the Eel resolved");
    assert_eq!(pt(&engine, eel), (1, 1), "a printed 1/1");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"When this creature enters, it deals 1 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing at all to the opponent"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "the two Mountains are still floating, in the same main phase"
    );

    // The offer is read off the pool rather than off the untapped lands, and
    // the index comes out of the offer rather than being guessed: {R}{R} is
    // the only ability the card can activate at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == eel)
        .expect("{{R}}{{R}} is affordable, so the one activated ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the offer was made off the pool the Mountains filled");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, eel),
        (3, 1),
        "{{R}}{{R}}: +2/+0 until end of turn, on the creature that prints it"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "and the second sentence bites its own controller a second time"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "both the pump and the shock say \"you\", and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{R}}{{R}} was the whole cost, and nothing was left floating"
    );
}
