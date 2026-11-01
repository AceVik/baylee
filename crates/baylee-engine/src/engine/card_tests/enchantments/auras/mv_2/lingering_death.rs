//! Lingering Death — `{1}{B}` Enchantment — Aura (SCG).
//! Oracle: "Enchant creature. At the beginning of the end step of enchanted creature's controller, that player sacrifices that creature."

use super::*;

fn lingering_death() -> CardIndex {
    card_index("53463d3d-fa97-46fb-a73b-35bb0e0dac72")
}

#[test]
fn lingering_death_makes_enchanted_creatures_controller_sacrifice_it_at_their_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let aura = lingering_death();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[aura])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, aura);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // At p0's end step, p1's enchanted creature is still on the battlefield.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    // Advance to p1's end step; p1 must sacrifice the enchanted Grizzly Bears.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected sacrifice choice");
    };
    assert_eq!(options, vec![bears]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, p0, aura).is_some());
}
