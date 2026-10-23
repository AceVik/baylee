#[allow(clippy::wildcard_imports)]
use super::*;

fn cloak_of_mists() -> CardIndex {
    card_index("13358b1d-0a35-48ed-a7fb-70359fa3998f")
}

#[test]
fn cloak_of_mists_makes_enchanted_creature_unblockable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[cloak_of_mists()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, cloak_of_mists());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::UNBLOCKABLE));
}
