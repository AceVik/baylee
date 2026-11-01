//! `cards/creatures/mv_7/minion_of_leshrac.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn minion_of_leshrac() -> CardIndex {
    card_index("8af3b981-6eda-46b0-a7b8-226bc719d137")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Minion of Leshrac (8af3b981-6eda-46b0-a7b8-226bc719d137) — {4}{B}{B}{B} 5/5 Creature — Demon Minion:
/// - "Protection from black"
/// - "At the beginning of your upkeep, this creature deals 5 damage to you unless you sacrifice a creature other than this creature. If this creature deals damage to you this way, tap it."
/// - "{T}: Destroy target creature or land."
#[test]
fn minion_of_leshrac_upkeep_sacrifice_or_damage_and_tap_plus_tap_destroy_creature_or_land() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[minion_of_leshrac(), grizzly_bears(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears(), forest(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);

    let minion = on_battlefield(&engine, P0, minion_of_leshrac()).unwrap();
    let p0_bears: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P0 && o.card.is_some_and(|c| c.index == grizzly_bears())
            })
        })
        .collect();
    assert_eq!(p0_bears.len(), 2);

    // Upkeep 1 (paid branch): P0 sacrifices one of the Grizzly Bears.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        !options.contains(&minion),
        "Minion of Leshrac itself is excluded ('a creature other than this creature')"
    );
    assert!(options.contains(&p0_bears[0]) && options.contains(&p0_bears[1]));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "sacrificing another creature avoids the 5 damage"
    );
    assert!(
        !entered_tapped(&engine, minion),
        "Minion of Leshrac stays untapped when it deals no damage"
    );

    // In P0's main phase, activate "{T}: Destroy target creature or land."
    reach_main_phase(&mut engine, P0);
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_land = on_battlefield(&engine, P1, forest()).unwrap();
    let p1_ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    activate(&mut engine, P0, minion_of_leshrac(), 2);
    let menu = aim_at(&mut engine, P0, p1_land);
    assert!(
        menu.contains(&p1_bears) && menu.contains(&p1_land) && !menu.contains(&p1_ring),
        "{{T}} ability targets creatures and lands, not noncreature artifacts"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P1, forest()).is_none(),
        "Minion of Leshrac's {{T}} ability destroyed target land"
    );

    // Advance to P0's next upkeep (turn 3): Minion untaps in untap step, then triggers in upkeep.
    // Branch 2 (unpaid): P0 declines to sacrifice p0_bears[1] (passes empty objects).
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().players[0].life,
        15,
        "declining to sacrifice another creature deals 5 damage to P0"
    );
    assert!(
        entered_tapped(&engine, minion),
        "Minion of Leshrac taps itself when it deals damage to you this way"
    );
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "the surviving Grizzly Bears was not sacrificed"
    );
}

#[test]
fn minion_of_leshrac_has_protection_from_black_so_black_creatures_cannot_block_it() {
    let mut engine = lk::game_with(
        SEED,
        &[minion_of_leshrac(), grizzly_bears()],
        &[index::DRUDGE_SKELETONS, grizzly_bears()],
        &[],
        &[],
        |e| {
            // Upkeep: sacrifice the other creature rather than take 5 and tap the Minion.
            let Pending::ChooseCards {
                player, options, ..
            } = e.pending().clone()
            else {
                panic!("expected the sacrifice question, got {:?}", e.pending());
            };
            e.apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(1).collect(),
                },
            )
            .unwrap();
        },
    );
    let minion = lk::obj(&engine, P0, minion_of_leshrac());
    let black_blocker = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);
    let green_blocker = lk::obj(&engine, P1, grizzly_bears());

    let offer = lk::attack(&mut engine, &[minion]);
    assert!(
        !lk::may_block(&offer, black_blocker, minion),
        "protection from black: a black creature cannot block Minion of Leshrac"
    );
    assert!(
        lk::may_block(&offer, green_blocker, minion),
        "a green creature can still block it"
    );
}
