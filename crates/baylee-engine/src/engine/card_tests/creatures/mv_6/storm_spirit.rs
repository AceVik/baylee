//! `cards/creatures/mv_6/storm_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Storm Spirit is a 3/3 flying Elemental Spirit under `Coverage::Implemented` with an activated damage ability.
/// Tapping the creature deals 2 damage to target creature.
/// Following `CR 601.2c` and `CR 601.2h`, the target is chosen first and the tap cost is paid as activation finishes.
/// Dealing lethal damage destroys the target via state-based actions, sending it to the graveyard while other creatures survive.
#[test]
fn storm_spirit_taps_to_deal_damage_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[storm_spirit()])
        .battlefield(1, &[llanowar_elves(), rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spirit =
        on_battlefield(&engine, p0, storm_spirit()).expect("Storm Spirit is on battlefield");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");

    assert_eq!(pt(&engine, spirit), (3, 3), "printed body is 3/3");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::FLYING),
        "Storm Spirit has flying"
    );
    assert!(!is_tapped(&engine, spirit), "Storm Spirit starts untapped");

    activate(&mut engine, p0, storm_spirit(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target creature required");
    assert!(
        options.contains(&elf) && options.contains(&wurm),
        "both creatures across the table are legal targets: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("target chosen");

    assert!(
        is_tapped(&engine, spirit),
        "the {{T}} cost is paid after targeting is completed"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "2 damage is lethal to a 1/1 Elf, destroying it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the destroyed Elf left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&wurm),
        "the Wurm was not targeted and remains on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, storm_spirit()).is_some(),
        "Storm Spirit remains on the battlefield"
    );
}
