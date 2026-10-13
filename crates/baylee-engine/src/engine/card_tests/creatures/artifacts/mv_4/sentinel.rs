//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn sentinel_sets_only_its_own_toughness_and_keeps_a_negative_targets_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for negative in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SENTINEL])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let sentinel = on_battlefield(&e, p0, index::SENTINEL).unwrap();
        let blocker = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(sentinel)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 1);
        attack_and_collect_blocks(&mut e, sentinel, p1);
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, sentinel)],
            },
        )
        .unwrap();
        activate(&mut e, p0, index::SENTINEL, 0);
        aim_at(&mut e, p0, blocker);
        if negative {
            let state = e.dev_state_mut(p0).unwrap();
            let timestamp = state.next_timestamp();
            let filter = crate::effects::EffectFilter::object(state, blocker);
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: None,
                controller: p0,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: baylee_cards_dsl::Layer::PtModify,
                timestamp,
                duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                filter,
                modifier: baylee_cards_dsl::Modifier::ModifyPT(-4, 0),
            });
            state.refresh_characteristics();
            assert_eq!(pt(&e, blocker), (-2, 2));
        }
        pass_until(&mut e, stack_is_empty);
        if negative {
            assert_eq!(
                e.state().object(sentinel).unwrap().zone,
                Zone::Graveyard,
                "-2 + 1 base + 1 counter = 0"
            );
        } else {
            assert_eq!(pt(&e, sentinel), (2, 4));
            assert_eq!(pt(&e, blocker), (2, 2));
            reach_their_main_phase(&mut e, p1);
            assert_eq!(pt(&e, sentinel), (2, 4), "setting persists");
        }
    }
}

/// Whether `source`'s printed ability `index` is on the offer to the seat
/// holding priority right now.
fn sentinel_offered(e: &Engine<RegistryLookup>, source: ObjectId) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(source, 0)))
}

/// "target creature blocking **or blocked by** this creature": the second
/// half, with Sentinel as the blocker. The attacking Bears is the one creature
/// on the menu; the opposing Ogre and the Elves beside Sentinel took no part
/// in this combat and are not, and neither is Sentinel itself.
#[test]
fn sentinel_blocking_an_attacker_may_target_exactly_that_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SENTINEL, llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    let sentinel = on_battlefield(&e, p0, index::SENTINEL).unwrap();
    let attacker = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    attack_and_collect_blocks(&mut e, attacker, p0);
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(sentinel, attacker)],
        },
    )
    .unwrap();
    // The active player (p1) holds priority first; p0 gets it after one pass.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut e, p0, index::SENTINEL, 0);
    let options = aim_at(&mut e, p0, attacker);
    assert_eq!(
        options,
        vec![attacker],
        "\"blocked by this creature\": only the Bears Sentinel blocks; an uninvolved Ogre, \
         an uninvolved Elves and Sentinel itself are not blocking or blocked by it"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, sentinel),
        (1, 3),
        "base toughness becomes 1 plus the Bears' power (2)"
    );
}

/// The ability needs a creature in this combat to point at. With no combat at
/// all, and with Sentinel attacking unblocked while a Bears and an Ogre sit
/// untapped across the table, nothing is "blocking or blocked by" it and the
/// ability is not offered; the blocked case above is the control on the same
/// board.
#[test]
fn sentinel_ability_is_not_offered_without_a_creature_in_its_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SENTINEL, llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own main phase"
    );
    let sentinel = on_battlefield(&e, p0, index::SENTINEL).unwrap();
    assert!(
        !sentinel_offered(&e, sentinel),
        "no combat yet: creatures stand on both sides and none of them blocks or is blocked"
    );
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sentinel,
                ability_index: 0
            }
        )
        .is_err()
    );
    assert_eq!(
        e.state().snapshot_hash(),
        hash,
        "the refusal changed nothing"
    );

    // Sentinel attacks and nobody blocks it.
    attack_and_collect_blocks(&mut e, sentinel, p1);
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        !sentinel_offered(&e, sentinel),
        "an unblocked Sentinel has no creature blocking it, and the idle Bears and Ogre are not"
    );
}
