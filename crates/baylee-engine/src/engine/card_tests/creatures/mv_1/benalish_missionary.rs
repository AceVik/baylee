//! `cards/creatures/mv_1/benalish_missionary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn benalish_missionary() -> CardIndex {
    card_index("84fdcfd3-2b22-4570-af34-7e3f55f97466")
}

/// `Benalish Missionary` ({W}, 1/1 Creature — Human Cleric, `Coverage::Partial`)
/// activates for `{1}{W}, {T}` to prevent all combat damage that would be dealt
/// by target blocked creature this turn.
#[test]
fn benalish_missionary_prevents_combat_damage_by_target_blocked_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[HILL_GIANT])
        .battlefield(
            1,
            &[plains(), plains(), GRIZZLY_BEARS, benalish_missionary()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, benalish_missionary(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![attacker],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, HILL_GIANT).is_none());
}
