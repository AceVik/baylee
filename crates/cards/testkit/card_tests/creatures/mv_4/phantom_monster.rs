//! `cards/creatures/mv_4/phantom_monster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Phantom Monster` is a 3/3 creature under `Coverage::Implemented` with flying and no activated abilities.
/// When cast from hand off four Islands, it resolves onto the battlefield with its printed 3/3 stats.
/// Its continuous characteristics grant flying to itself while grounded bystanders remain unchanged.
#[test]
fn phantom_monster_resolves_as_a_flying_three_three_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[phantom_monster()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf deployed");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "Elf is a grounded creature"
    );

    let card = in_hand(&engine, p0, phantom_monster()).expect("Phantom Monster is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&card),
        "cannot cast {{3}}{{U}} with an empty mana pool"
    );

    // Keep the Elf untapped so its mana ability does not inflate the pool.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands produce four blue mana"
    );

    cast_with_floating(&mut engine, p0, phantom_monster());
    pass_until(&mut engine, stack_is_empty);

    let monster = on_battlefield(&engine, p0, phantom_monster()).expect("Phantom Monster resolved");
    assert_eq!(
        pt(&engine, monster),
        (3, 3),
        "printed power and toughness is 3/3"
    );
    assert!(
        keywords(&engine, monster).contains(KeywordSet::FLYING),
        "Phantom Monster has flying"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "grounded Elf did not gain flying"
    );
}
