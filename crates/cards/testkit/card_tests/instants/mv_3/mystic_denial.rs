//! `cards/instants/mv_3/mystic_denial.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystic Denial is a {1}{U}{U} instant under `Coverage::Implemented` that counters target creature or sorcery spell.
/// When an opponent casts a creature spell, Mystic Denial targets that spell while it is on the stack.
/// Resolving Mystic Denial counters the spell, sending the creature card directly to its owner's graveyard.
/// The creature never enters the battlefield.
#[test]
fn mystic_denial_counters_a_creature_spell_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[mystic_denial()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());

    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "caster holds priority");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let Pending::Priority {
        player: responding, ..
    } = engine.pending().clone()
    else {
        panic!("expected opponent priority, got {:?}", engine.pending())
    };
    assert_eq!(responding, p1, "priority passes to opponent");

    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "three Islands produce three blue mana"
    );
    cast_with_floating(&mut engine, p1, mystic_denial());

    let Pending::ChooseTargets {
        player: targeting_player,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        targeting_player, p1,
        "opponent selects target for Mystic Denial"
    );

    let elf_spell = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .expect("targeting creature spell on stack is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered creature never reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered creature is in owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mystic_denial()).is_some(),
        "Mystic Denial went to graveyard upon resolution"
    );
}
