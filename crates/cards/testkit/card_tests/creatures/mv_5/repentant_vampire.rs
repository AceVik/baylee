//! `cards/creatures/mv_5/repentant_vampire.rs`, played.
//!
//! Oracle:
//! - "Flying"
//! - "Whenever a creature dealt damage by this creature this turn dies, put a
//!   +1/+1 counter on this creature."
//! - "Threshold — As long as seven or more cards are in your graveyard, this
//!   creature is white and has '{T}: Destroy target black creature.'"

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn repentant_vampire() -> CardIndex {
    card_index("a5dabc98-0db9-4b4f-b2d2-f74dbd022d22")
}

#[test]
fn repentant_vampire_grows_only_when_creature_it_damaged_dies_same_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), mountain(), forest(), forest()],
        )
        .battlefield(
            1,
            &[
                index::SAVANNAH_LIONS,
                index::GIANT_SPIDER,
                index::GIANT_SPIDER,
            ],
        )
        .hand(
            0,
            &[
                repentant_vampire(),
                index::SHOCK,
                index::DARK_BANISHING,
                index::SHOCK,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Leave 1 mountain + 1 swamp + 2 forests? Actually cast from hand using 5 mana.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, repentant_vampire());
    pass_until(&mut engine, stack_is_empty);
    let vamp = on_battlefield(&engine, P0, repentant_vampire()).unwrap();
    assert_eq!(pt(&engine, vamp), (3, 3));
    assert!(keywords(&engine, vamp).contains(KeywordSet::FLYING));

    // Killing a creature Vampire did NOT damage grants no counter.
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    give_mana(&mut engine, P0, ManaColor::Red, 1);
    cast_with_floating(&mut engine, P0, index::SHOCK);
    aim_at(&mut engine, P0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, vamp, CounterKind::P1P1), 0);

    // Turn 3: Giant Spider #1 blocks Vampire (takes 3 damage, survives). Do NOT finish it this turn.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let spiders = all_on_battlefield(&engine, P1, index::GIANT_SPIDER);
    attack_and_collect_blocks(&mut engine, vamp, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spiders[0], vamp)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Turn 5: kill Spider #1 next turn -> "this turn" expired, no counter!
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    give_mana(&mut engine, P0, ManaColor::Black, 3);
    cast_with_floating(&mut engine, P0, index::DARK_BANISHING);
    aim_at(&mut engine, P0, spiders[0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, vamp, CounterKind::P1P1), 0);

    // Attack into Spider #2, then Shock it in SecondMain of the SAME turn -> +1/+1 counter.
    attack_and_collect_blocks(&mut engine, vamp, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spiders[1], vamp)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    give_mana(&mut engine, P0, ManaColor::Red, 1);
    cast_with_floating(&mut engine, P0, index::SHOCK);
    aim_at(&mut engine, P0, spiders[1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, vamp, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, vamp), (4, 4));
}

#[test]
fn repentant_vampire_threshold_changes_color_and_destroys_black_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[repentant_vampire(), mountain()])
        .battlefield(1, &[index::SCATHE_ZOMBIES, index::GRIZZLY_BEARS])
        .hand(0, &[index::SHOCK, bojuka_bog()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let vamp = on_battlefield(&engine, P0, repentant_vampire()).unwrap();
    let zombies = on_battlefield(&engine, P1, index::SCATHE_ZOMBIES).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    // 6 cards in graveyard: Threshold inactive (still black, ability 1 absent).
    seed_graveyard(&mut engine, P0, 6);
    engine.refresh_offer();
    assert_eq!(
        engine
            .state()
            .object(vamp)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Black)
    );
    assert!(!ability_offered(&engine, vamp, 1));

    // Cast Shock (7th card into graveyard through real resolution -> statics sync!).
    cast_from_hand(&mut engine, P0, index::SHOCK);
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!("expected Shock target prompt");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        7
    );
    assert_eq!(
        engine
            .state()
            .object(vamp)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::White)
    );
    assert!(ability_offered(&engine, vamp, 1));

    activate(&mut engine, P0, repentant_vampire(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&zombies));
    assert!(
        !options.contains(&bears),
        "Grizzly Bears is green, not black"
    );
    assert!(
        !options.contains(&vamp),
        "Vampire is now white, cannot self-target"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![zombies],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P1, index::SCATHE_ZOMBIES).is_some());
    assert!(is_tapped(&engine, vamp));

    // Exiling own graveyard drops count to 0: Vampire reverts to Black and loses ability.
    play_land(&mut engine, P0, bojuka_bog());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected Bojuka Bog player target, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(vamp)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Black)
    );
    assert!(!ability_offered(&engine, vamp, 1));
}
