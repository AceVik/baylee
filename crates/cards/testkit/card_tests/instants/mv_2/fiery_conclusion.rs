//! `cards/instants/mv_2/fiery_conclusion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "91af3c81-615a-4b89-acf2-34c72001ebbd"
fn fiery_conclusion() -> CardIndex {
    card_index("91af3c81-615a-4b89-acf2-34c72001ebbd")
}

/// Fiery Conclusion ({1}{R}): cannot be cast without a creature you control to
/// sacrifice; sacrifices your creature as an additional cost and deals 5 damage
/// to target creature (not players).
#[test]
fn fiery_conclusion_requires_your_creature_sacrifices_it_and_deals_five() {
    let mut no_sac = lk::game(SEED, &[], &[index::SERRA_ANGEL], &[fiery_conclusion()], &[]);
    lk::float(&mut no_sac, P0, &[(ManaColor::Red, 2)]);
    assert!(
        !lk::castable(&no_sac, P0, fiery_conclusion()),
        "cannot cast without a creature you control to sacrifice"
    );

    let mut engine = lk::game(
        SEED,
        &[index::LLANOWAR_ELVES],
        &[index::SERRA_ANGEL],
        &[fiery_conclusion()],
        &[],
    );
    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut engine, P0, fiery_conclusion());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        player_options.is_empty(),
        "targets creatures only, not players"
    );
    assert!(options.contains(&angel));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
    }
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, index::LLANOWAR_ELVES).is_some());
    assert!(
        in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some(),
        "5 damage killed 4/4 Serra Angel"
    );
}
