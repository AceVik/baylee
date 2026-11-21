//! `cards/artifacts/mv_3/mizzium_transreliquat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn mizzium_transreliquat() -> CardIndex {
    card_index("a1f73421-cda5-4029-9bcf-f2baee179e5c")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

/// Mizzium Transreliquat (a1f73421-cda5-4029-9bcf-f2baee179e5c) — {3} Artifact:
/// "{3}: This artifact becomes a copy of target artifact until end of turn."
/// "{1}{U}{R}: This artifact becomes a copy of target artifact, except it has
///  this ability."
///
/// Ability 0 copies Steel Wall (0/4 Defender) until end of turn and reverts on
/// the next turn.
#[test]
fn mizzium_transreliquat_temporary_copy_reverts_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                mizzium_transreliquat(),
                steel_wall(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let reliquat = on_battlefield(&engine, p0, mizzium_transreliquat()).unwrap();
    let wall = on_battlefield(&engine, p0, steel_wall()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mizzium_transreliquat(), 0);
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, reliquat), (0, 4), "copied Steel Wall's 0/4");
    assert!(keywords(&engine, reliquat).contains(KeywordSet::DEFENDER));

    // Next turn: temporary copy effect ended and Transreliquat is no longer a creature.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(reliquat)
            .unwrap()
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE),
        "reverted to noncreature Mizzium Transreliquat"
    );
}

/// Ability 1 ({1}{U}{R}) permanently copies Sol Ring while retaining the
/// `{1}{U}{R}` copy ability — persisting across turns, producing `{C}{C}`, and
/// later using the retained `{1}{U}{R}` ability to copy Steel Wall.
#[test]
fn mizzium_transreliquat_permanent_copy_keeps_copy_ability_across_turns() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                mizzium_transreliquat(),
                sol_ring(),
                steel_wall(),
                island(),
                mountain(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let reliquat = on_battlefield(&engine, p0, mizzium_transreliquat()).unwrap();
    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let wall = on_battlefield(&engine, p0, steel_wall()).unwrap();

    // Activate {1}{U}{R} (ability 1) copying Sol Ring.
    tap_all_mana_but(&mut engine, p0, Some(sol_ring()));
    activate(&mut engine, p0, mizzium_transreliquat(), 1);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    // Cross into p0's next turn: still a Sol Ring copy!
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);

    // Activate the copied Sol Ring's {T}: Add {C}{C} (ability 0) + Island + Mountain -> {2}{U}{R}.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: reliquat,
                ability_index: 0,
            },
        )
        .expect("copied Sol Ring's {T}: Add {C}{C} is ability 0");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "permanently copied Sol Ring's {{T}}: Add {{C}}{{C}}"
    );
    let isl = on_battlefield(&engine, p0, island()).unwrap();
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == isl || id == mtn);

    // Activate the retained {1}{U}{R} copy ability on the Sol Ring copy to become Steel Wall.
    let copy_idx = priority_offer(&engine)
        .abilities
        .iter()
        .find_map(|&(src, idx)| (src == reliquat).then_some(idx))
        .expect("retained {1}{U}{R} copy ability is still available on the copy");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: reliquat,
                ability_index: copy_idx,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, reliquat),
        (0, 4),
        "retained {{1}}{{U}}{{R}} turned the Sol Ring copy into Steel Wall"
    );
    assert!(keywords(&engine, reliquat).contains(KeywordSet::DEFENDER));
}
