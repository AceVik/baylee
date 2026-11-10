//! `cards/creatures/mv_5/ravaging_horde.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ravaging Horde's whole printed text is one enters-trigger and one target:
/// "When this creature enters, destroy target land." The board is built so the
/// menu has to be read rather than assumed — the five Mountains that pay
/// {3}{R}{R} stand beside the defender's Forest, its Sol Ring and the Horde
/// itself, and only the lands may be on it. Naming the Forest is what makes the
/// rest a control: the Mountain the trigger did not name, the artifact its
/// filter declines and the Horde all have to survive it.
#[test]
fn ravaging_horde_destroys_the_land_its_enter_trigger_names() {
    fn ravaging_horde() -> CardIndex {
        card_index("b775b83c-0c3d-437d-9d3c-1d5c9cd418a8")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ravaging_horde()])
        // The Forest is the land this test names, and the Sol Ring beside it
        // is a permanent that is no land at all, so the filter is read.
        .battlefield(1, &[forest(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let mine = on_battlefield(&engine, p0, mountain()).expect("my Mountain is out");

    // Five Mountains are exactly {3}{R}{R}, so the whole board pays for the
    // Horde — the trigger below charges no mana and is offered whatever the
    // pool holds.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, and the Horde's controller has no other source on the board"
    );
    cast_with_floating(&mut engine, p0, ravaging_horde());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{R}}{{R}} took every mana the Mountains made"
    );

    // The creature resolves and its enters-trigger asks for the land; the
    // Horde is on the battlefield by then, which is what "enters" means.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let horde = on_battlefield(&engine, p0, ravaging_horde()).expect("the Horde resolved");
    assert_eq!(pt(&engine, horde), (3, 3), "the body the card prints");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target question")
    };
    assert_eq!(
        player, p0,
        "the Horde's controller is the one that names it"
    );
    assert_eq!((min, max), (1, 1), "one land, and the trigger asks once");
    assert!(
        options.contains(&theirs) && options.contains(&mine),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "an artifact is no land, whatever its controller: {options:?}"
    );
    assert!(
        !options.contains(&horde),
        "and a creature is none either, its own trigger's target included: {options:?}"
    );
    assert_eq!(
        options.len(),
        6,
        "the five Mountains and the one Forest are every land on the battlefield: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Forest was one of the options the trigger enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"destroy target land\": the Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it left the battlefield, which is what destroy means"
    );
    assert_eq!(
        engine.state().object(mine).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Mountain the trigger did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "nor did the permanent its filter declined"
    );
    assert!(
        on_battlefield(&engine, p0, ravaging_horde()).is_some(),
        "the Horde is the trigger's source and survives it"
    );
}
