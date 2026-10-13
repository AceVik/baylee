//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rapid_fire_grants_one_instance_only_when_rampage_is_absent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (card, n) in [(grizzly_bears(), 2), (index::AERATHI_BERSERKER, 3)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[card, plains(), plains(), plains(), plains()])
            .battlefield(1, &[giant_spider(), giant_spider()])
            .hand(0, &[index::RAPID_FIRE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let a = on_battlefield(&e, p0, card).unwrap();
        let base = pt(&e, a);
        cast_from_hand(&mut e, p0, index::RAPID_FIRE);
        aim_at(&mut e, p0, a);
        pass_until(&mut e, stack_is_empty);
        assert!(keywords(&e, a).contains(KeywordSet::FIRST_STRIKE));
        let blockers: Vec<_> = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| e.state().object(*id).unwrap().controller == p1)
            .collect();
        attack_and_collect_blocks(&mut e, a, p1);
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: blockers.iter().map(|b| (*b, a)).collect(),
            },
        )
        .unwrap();
        assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, a), (base.0 + n, base.1 + n));
    }
}

/// "Cast this spell only before blockers are declared": offered in the main
/// phase and the declare attackers step, refused once blockers are declared.
#[test]
fn rapid_fire_is_castable_before_blockers_and_refused_after() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[giant_spider()])
        .hand(0, &[index::RAPID_FIRE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    let spell = in_hand(&e, p0, index::RAPID_FIRE).unwrap();
    let float_white = |e: &mut Engine<RegistryLookup>| {
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::White, 4);
        e.refresh_offer();
    };
    let castable = |e: &Engine<RegistryLookup>| matches!(e.pending(), Pending::Priority { player, legal } if *player == p0 && legal.castable.contains(&spell));

    float_white(&mut e);
    assert!(castable(&e), "before combat the spell is on the offer");

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert_eq!(e.state().turn.step, crate::turn::Step::DeclareAttackers);
    float_white(&mut e);
    assert!(
        castable(&e),
        "with an attacker declared and no blockers, it is still on the offer"
    );

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, bear)],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(e.state().turn.step, crate::turn::Step::DeclareBlockers);
    float_white(&mut e);
    assert!(
        e.state().players[0].mana_pool.available(ManaColor::White) >= 4,
        "{{3}}{{W}} is floating, so only the timing stands in the way"
    );
    assert!(
        !castable(&e),
        "\"only before blockers are declared\": not offered after the blocks"
    );
    let before = e.snapshot_hash();
    assert!(
        e.apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "and casting it then is refused"
    );
    assert_eq!(e.snapshot_hash(), before, "the refusal changed nothing");
}
