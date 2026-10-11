//! Small fixtures shared by card regressions; all effects use engine rules.
#[allow(clippy::wildcard_imports)]
use super::*;
pub(super) fn give_mana(e: &mut Engine<RegistryLookup>, who: PlayerId, color: ManaColor, n: u32) {
    e.dev_state_mut(who).unwrap().players[who.get() as usize]
        .mana_pool
        .add(color, n);
    e.refresh_offer();
}
pub(super) fn attach_test_aura(e: &mut Engine<RegistryLookup>, who: PlayerId, target: ObjectId) {
    let aura = baylee_core::generated::index::HOLY_STRENGTH;
    give_mana(e, who, ManaColor::White, 1);
    let card = in_hand(e, who, aura).unwrap();
    e.apply(who, PlayerAction::CastSpell { card }).unwrap();
    aim_at(e, who, target);
    pass_until(e, stack_is_empty);
}
pub(super) fn deal_test_damage(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    target: ObjectId,
    combat: bool,
) -> u32 {
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let source_version = Some(state.object(source).unwrap().version);
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version,
            recipient: crate::event::DamageTarget::Object(target),
            amount: 1,
            is_combat: combat,
        }],
    );
    assert!(
        work.advance(state).is_none(),
        "at most one applicable prevention effect"
    );
    work.dealt()
}

/// Three reachable blockers, one rampage trigger, with the count frozen on resolution.
pub(super) fn assert_printed_rampage(card: CardIndex, n: i16) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for count in [1, 3] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[card])
            .battlefield(1, &[giant_spider(), giant_spider(), giant_spider()])
            .start();
        keep_mulligans(&mut e);
        if card == baylee_core::generated::index::CHROMIUM {
            for c in [ManaColor::White, ManaColor::Blue, ManaColor::Black] {
                give_mana(&mut e, p0, c, 1);
            }
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            e.apply(p0, PlayerAction::YesNo(true)).unwrap();
        }
        reach_main_phase(&mut e, p0);
        let attacker = on_battlefield(&e, p0, card).unwrap();
        let base = pt(&e, attacker);
        let blockers: Vec<_> = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| e.state().object(*id).unwrap().controller == p1)
            .take(count)
            .collect();
        attack_and_collect_blocks(&mut e, attacker, p1);
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: blockers.iter().map(|b| (*b, attacker)).collect(),
            },
        )
        .unwrap();
        assert_eq!(
            e.state().zones.list(ZoneLocation::Stack).len(),
            1,
            "one transition, not one trigger per blocker"
        );
        if count == 3 {
            bury(&mut e, &[blockers[0]]);
        }
        pass_until(&mut e, stack_is_empty);
        let bonus = if count == 3 { n } else { 0 };
        assert_eq!(pt(&e, attacker), (base.0 + bonus, base.1 + bonus));
        if count == 3 {
            bury(&mut e, &[blockers[1]]);
            assert_eq!(
                pt(&e, attacker),
                (base.0 + bonus, base.1 + bonus),
                "no ongoing recalculation"
            );
        }
    }
}
