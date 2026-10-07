//! `cards/creatures/mv_4/fighting_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fighting Drake` is a 2/4 creature under `Coverage::Implemented` with flying and no activated abilities.
/// When cast from hand off four Islands, it resolves onto the battlefield with its printed 2/4 stats.
/// Its continuous characteristics grant flying to itself while grounded bystanders remain unchanged.
#[test]
fn fighting_drake_resolves_as_a_flying_two_four_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[fighting_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf deployed");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "Elf is a grounded creature"
    );

    let card = in_hand(&engine, p0, fighting_drake()).expect("Fighting Drake is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&card),
        "cannot cast {{2}}{{U}}{{U}} with an empty mana pool"
    );

    // Keep the Elf untapped so its mana ability does not inflate the pool.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands produce four blue mana"
    );

    cast_with_floating(&mut engine, p0, fighting_drake());
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, p0, fighting_drake()).expect("Fighting Drake resolved");
    assert_eq!(
        pt(&engine, drake),
        (2, 4),
        "printed power and toughness is 2/4"
    );
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "Fighting Drake has flying"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "grounded Elf did not gain flying"
    );
}
