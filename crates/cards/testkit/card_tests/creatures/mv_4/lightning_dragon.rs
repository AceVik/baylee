//! `cards/creatures/mv_4/lightning_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lightning_dragon() -> CardIndex {
    card_index("be0fc708-b205-4307-85f3-f8fd1efd4cf6")
}

/// Paying echo `{2}{R}{R}` on the next upkeep keeps the flying 4/4, `{R}` pumps
/// it +1/+0 until end of turn, and echo does not ask again on later upkeeps.
#[test]
fn lightning_dragon_pays_echo_once_flies_over_ground_blockers_and_pumps_power() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lightning_dragon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let lands: Vec<_> = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| lands[..4].contains(&id));
    let spell = in_hand(&engine, p0, lightning_dragon()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, lightning_dragon()).expect("resolved");
    assert_eq!(pt(&engine, dragon), (4, 4));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));

    // Advance to p0's next upkeep, stopping at the echo payment question.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, p0, |id| lands[..4].contains(&id));
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(on_battlefield(&engine, p0, lightning_dragon()).is_some());

    // Spend the two remaining untapped Mountains on `{R}: +1/+0 until end of turn` twice.
    tap_mana_where(&mut engine, p0, |id| lands[4..].contains(&id));
    for _ in 0..2 {
        activate(&mut engine, p0, lightning_dragon(), 1);
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(pt(&engine, dragon), (6, 4));

    let blocks = attack_and_collect_blocks(&mut engine, dragon, p1);
    assert!(
        blocks.iter().all(|opt| !opt.attackers.contains(&dragon)),
        "ground Grizzly Bears cannot block a flying Dragon"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[1].life, 14, "6 flying damage dealt");
    assert_eq!(pt(&engine, dragon), (4, 4), "+1/+0 expires at end of turn");

    // Next p0 upkeep: controlled continuously since before last upkeep, so echo does not fire.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, lightning_dragon()).is_some(),
        "echo does not sacrifice it on subsequent upkeeps"
    );
}

/// Declining echo sacrifices Lightning Dragon during controller's upkeep.
#[test]
fn lightning_dragon_is_sacrificed_when_echo_is_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[lightning_dragon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, lightning_dragon());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, lightning_dragon()).is_some());

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(on_battlefield(&engine, p0, lightning_dragon()).is_none());
    assert!(in_graveyard(&engine, p0, lightning_dragon()).is_some());
}
