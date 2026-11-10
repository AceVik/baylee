//! `cards/creatures/mv_4/ravenous_baloth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ravenous Baloth is a 4/4 Beast under `Coverage::Implemented` with an activated sacrifice ability.
/// Sacrificing a Beast gains its controller 4 life.
/// The sacrifice prompt is flagged as `ChoicePrompt::CostSacrifice`, offering Beasts while non-Beast creatures are excluded.
/// Sacrificing the Baloth to its own ability sends it to the graveyard and increases life from 16 to 20.
#[test]
fn ravenous_baloth_sacrifices_a_beast_to_gain_four_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ravenous_baloth(), llanowar_elves()])
        .life(0, 16)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let baloth =
        on_battlefield(&engine, p0, ravenous_baloth()).expect("Ravenous Baloth is on battlefield");
    let elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is on battlefield");

    assert_eq!(pt(&engine, baloth), (4, 4), "printed body is 4/4");

    activate(&mut engine, p0, ravenous_baloth(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "controller selects which permanent to sacrifice"
    );
    assert_eq!((min, max), (1, 1), "exactly one permanent sacrificed");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "prompt is flagged as a sacrifice cost"
    );
    assert!(
        options.contains(&baloth),
        "Ravenous Baloth is a Beast and can be sacrificed: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "Elf is not a Beast and cannot be sacrificed: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![baloth],
            },
        )
        .expect("sacrificing the Baloth is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, ravenous_baloth()).is_some(),
        "sacrificed Baloth is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, ravenous_baloth()).is_none(),
        "sacrificed Baloth left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&elf),
        "unrelated Elf remains on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "controller gained 4 life, moving from 16 to 20"
    );
}
