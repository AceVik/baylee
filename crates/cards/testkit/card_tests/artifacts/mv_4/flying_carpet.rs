//! `cards/artifacts/mv_4/flying_carpet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flying Carpet` is an artifact under `Coverage::Implemented` with an activated ability costing `{2}` and tapping to grant flying to target creature.
/// Tapping two Forests floats the required mana while keeping the Carpet untapped.
/// Targeting a grounded creature grants it flying until end of turn, which expires on the following turn.
#[test]
fn flying_carpet_taps_to_grant_flying_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), flying_carpet(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let carpet = on_battlefield(&engine, p0, flying_carpet()).expect("Carpet deployed");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf deployed");

    assert!(!is_tapped(&engine, carpet), "Carpet starts untapped");
    assert!(
        !keywords_of(&engine, elf).contains(KeywordSet::FLYING),
        "Elf is grounded initially"
    );

    // Float {2} from Forests while keeping Flying Carpet and the Elf untapped.
    tap_mana_where(&mut engine, p0, |id| id != carpet && id != elf);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests produce two mana"
    );

    activate(&mut engine, p0, flying_carpet(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(options.contains(&elf), "creature is an offered target");
    assert!(
        !options.contains(&carpet),
        "the artifact is not a creature: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("targeting Elf is legal");

    assert!(is_tapped(&engine, carpet), "Flying Carpet tapped as cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} spent from mana pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords_of(&engine, elf).contains(KeywordSet::FLYING),
        "Elf gained flying until end of turn"
    );

    // Duration expires on the next turn.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords_of(&engine, elf).contains(KeywordSet::FLYING),
        "flying expired after turn ended"
    );
}
