//! `cards/instants/mv_3/teferi_s_protection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Teferi's Protection — {2}{W} instant: "Until your next turn, your life
/// total can't change and you gain protection from everything. All
/// permanents you control phase out. Exile Teferi's Protection."
///
/// One of those four sentences is expressible and three are the
/// `Coverage::Partial` gap, so the test is the exile plus the shape of what
/// is missing. The exile is read in its own right because it is the clause
/// that was *built and then undone*: `Effect::ExileSource` moved the card
/// off the stack and `finalize_spell` fetched it back into the graveyard,
/// which the rules test beside it
/// ([`super::rules`]) now holds shut.
///
/// The three missing clauses are read as one absence rather than three,
/// and deliberately: "your life total can't change" and "you gain
/// protection from everything" would both be continuous effects, and the
/// spell registers none at all. The phase-out is read on the permanent
/// itself, because `Status::PHASED_OUT` exists and nothing set it — an
/// Elf that is still an ordinary untapped creature after the spell
/// resolved is the printed sentence not happening.
#[test]
fn teferis_protection_exiles_itself_and_leaves_everything_else_exactly_as_it_was() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[teferis_protection()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    let effects_before = engine.state().effects.iter().count();
    let life_before = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, teferis_protection());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == teferis_protection()))
            }),
        "\"Exile Teferi's Protection\" is the one clause the DSL can say"
    );

    assert_eq!(
        engine.state().effects.iter().count(),
        effects_before,
        "\"your life total can't change\" and \"you gain protection from \
         everything\" are both continuous effects, and the spell registered \
         neither"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "and nothing touched the life total the first of them is about"
    );
    assert!(
        engine
            .state()
            .object(elf)
            .is_some_and(|o| !o.status.contains(Status::PHASED_OUT)),
        "\"All permanents you control phase out\" — the status exists and \
         nothing set it, so the Elf is an ordinary creature still"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and it is still on the battlefield, where a phased-out permanent \
         would also be"
    );
}
