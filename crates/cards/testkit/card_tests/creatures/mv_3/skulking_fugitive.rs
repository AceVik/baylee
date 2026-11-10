//! `cards/creatures/mv_3/skulking_fugitive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "4178ae56-2449-4617-aff8-f59d8fb96c66"
fn skulking_fugitive() -> CardIndex {
    card_index("4178ae56-2449-4617-aff8-f59d8fb96c66")
}

/// `Skulking Fugitive` (`Coverage::Generated`):
/// "When this creature becomes the target of a spell or ability, sacrifice it."
///
/// Verifies that targeting `Skulking Fugitive` with a spell triggers its
/// sacrifice ability, putting it into its owner's graveyard before the spell
/// resolves.
#[test]
fn skulking_fugitive_sacrifices_itself_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9610, forest())
        .battlefield(0, &[skulking_fugitive(), forest()])
        .hand(0, &[index::GIANT_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let fugitive = on_battlefield(&engine, p0, skulking_fugitive()).expect("P0 Skulking Fugitive");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GIANT_GROWTH);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![fugitive],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, skulking_fugitive()).is_none(),
        "Skulking Fugitive sacrificed itself when targeted"
    );
}
