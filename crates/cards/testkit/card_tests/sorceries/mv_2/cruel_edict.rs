//! `cards/sorceries/mv_2/cruel_edict.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cruel_edict() -> CardIndex {
    card_index("10c585c4-bf5b-4d8f-94a9-e9a5036a688f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Cruel Edict (10c585c4-bf5b-4d8f-94a9-e9a5036a688f) — {1}{B} Sorcery:
/// "Target opponent sacrifices a creature of their choice." Only opponents are
/// legal player targets, and the targeted opponent chooses which creature dies.
#[test]
fn cruel_edict_targets_only_an_opponent_who_chooses_which_creature_to_sacrifice() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), aurochs()],
        &[grizzly_bears(), llanowar_elves()],
        &[cruel_edict()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());
    let their_elf = lk::obj(&engine, P1, llanowar_elves());

    cast_from_hand(&mut engine, P0, cruel_edict());
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, P0);
            assert_eq!(options, vec![P1], "only opponents may be targeted");
            engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        }
        Pending::ChooseTargets {
            player,
            player_options,
            options,
            ..
        } => {
            assert_eq!(player, P0);
            assert!(options.is_empty());
            assert_eq!(player_options, vec![P1], "only opponents may be targeted");
            lk::aim(&mut engine, &[], &[P1]);
        }
        other => panic!("expected opponent target prompt, got {other:?}"),
    }

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P1, "the targeted opponent chooses");
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&their_bears) && options.contains(&their_elf));
    assert!(!options.contains(&my_beast));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![their_bears],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, their_bears), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_elf), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, my_beast), Zone::Battlefield);
}

/// Cruel Edict — targets the opponent rather than the creature, so a lone
/// hexproof creature is still sacrificed.
#[test]
fn cruel_edict_forces_sacrifice_of_a_lone_hexproof_creature() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp()],
        &[thrun_the_last_troll()],
        &[cruel_edict()],
        &[],
    );
    let thrun = lk::obj(&engine, P1, thrun_the_last_troll());

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, cruel_edict(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![thrun],
                },
            )
            .unwrap();
        lk::settle(&mut engine);
    }

    assert_eq!(lk::zone_of(&engine, thrun), Zone::Graveyard);
}
