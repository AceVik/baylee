//! `cards/creatures/mv_4/phantasmal_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phantasmal_dragon() -> CardIndex {
    card_index("7f32c581-4d4a-4873-8e4b-b95e13790481")
}

/// Targeting another creature leaves Phantasmal Dragon alone to fly over ground
/// blockers for 5, whereas targeting Phantasmal Dragon itself sacrifices it on top
/// of the targeting spell.
#[test]
fn phantasmal_dragon_flies_until_a_spell_targets_it_then_sacrifices_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), forest(), forest()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[phantasmal_dragon(), giant_growth(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    let spell = in_hand(&engine, p0, phantasmal_dragon()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, phantasmal_dragon()).expect("resolved");
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));

    // Targeting Grizzly Bears with the first Giant Growth does NOT trigger Phantasmal Dragon.
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    let growth1 = in_hand(&engine, p0, giant_growth()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: growth1 })
        .unwrap();
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
    assert!(on_battlefield(&engine, p0, phantasmal_dragon()).is_some());

    // Next turn, Phantasmal Dragon flies over Grizzly Bears for 5 damage.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let blocks = attack_and_collect_blocks(&mut engine, dragon, p1);
    assert!(blocks.iter().all(|opt| !opt.attackers.contains(&dragon)));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 15);

    // Targeting Phantasmal Dragon with the second Giant Growth triggers SacrificeSelf.
    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    let growth2 = in_hand(&engine, p0, giant_growth()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: growth2 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dragon],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, phantasmal_dragon()).is_none());
    assert!(in_graveyard(&engine, p0, phantasmal_dragon()).is_some());
}
