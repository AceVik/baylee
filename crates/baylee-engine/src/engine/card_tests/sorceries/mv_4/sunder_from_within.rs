//! `cards/sorceries/mv_4/sunder_from_within.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunder from Within — {2}{R}{R} Sorcery — Arcane: "Destroy target artifact
/// or land."
///
/// The `Or` is the whole card, so the board carries one of each half under the
/// opponent while a creature stands beside them: a menu that had lost the
/// filter would offer the Elf and this one must decline it. Both halves are
/// read off the same one question — the artifact is chosen and the land is
/// left standing, which is what tells a single target from a board sweep — and
/// CR 601.2c before CR 601.2h is what leaves the spell on the stack with its
/// four Mountains' mana still floating while the target question is open.
#[test]
fn sunder_from_within_destroys_the_artifact_or_land_it_names_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[sunder_from_within()])
        .battlefield(1, &[quiet_artifact(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let land = on_battlefield(&engine, p1, forest()).expect("their land is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Four Mountains are exactly {2}{R}{R}, and nothing else on this board
    // makes mana, so whatever the pool holds is the Mountains' own.
    cast_from_hand(&mut engine, p0, sunder_from_within());

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&rock),
        "the artifact half of \"target artifact or land\": {options:?}"
    );
    assert!(
        options.contains(&land),
        "and the land half, read off the same one choice: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor a land: {options:?}"
    );
    assert!(
        in_hand(&engine, p0, sunder_from_within()).is_some(),
        "the cast is still being put together, so the card has not left the hand yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "CR 601.2c names the target before CR 601.2h pays, so the four \
         Mountains' red is still floating and the artifact is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "nothing has resolved yet: the target is named, not destroyed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{R}}{{R}} was paid as the last step of the cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy\" puts the named artifact in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and off the battlefield, which is what destroying it means"
    );
    assert!(
        in_graveyard(&engine, p0, sunder_from_within()).is_some(),
        "the sorcery resolved and is in its caster's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the land was a legal target and was not the one named: one target, \
         one permanent destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the creature the filter declined never moved"
    );
}
