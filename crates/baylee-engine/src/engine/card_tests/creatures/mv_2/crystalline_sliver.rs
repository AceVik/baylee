//! `cards/creatures/mv_2/crystalline_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Crystalline Sliver` grants `KeywordSet::SHROUD` to all Slivers as a static ability under `Coverage::Implemented`.
/// On a board with `Crystalline Sliver` and a non-Sliver creature (`llanowar_elves`), the Sliver has shroud
/// while the Elf does not. When `Swords to Plowshares` is cast, `Pending::ChooseTargets` offers the Elf
/// but excludes `Crystalline Sliver` due to shroud.
#[test]
fn crystalline_sliver_grants_shroud_preventing_targeting() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1609, forest())
        .battlefield(0, &[crystalline_sliver(), llanowar_elves(), plains()])
        .hand(0, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sliver = on_battlefield(&engine, p0, crystalline_sliver()).expect("sliver is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is seated");

    assert!(keywords(&engine, sliver).contains(KeywordSet::SHROUD));
    assert!(!keywords(&engine, elf).contains(KeywordSet::SHROUD));

    cast_from_hand(&mut engine, p0, swords_to_plowshares());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Swords to Plowshares");
    };
    assert!(options.contains(&elf), "Elf can be targeted");
    assert!(
        !options.contains(&sliver),
        "Crystalline Sliver has shroud and cannot be targeted"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p0, crystalline_sliver()).is_some());
}
