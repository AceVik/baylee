//! Hone is a rule of the counter, independent of either card's abilities.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn dwalin_hones_your_equipment_on_entry_and_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::BONESPLITTER, plains(), mountain()])
        .battlefield(1, &[index::BONESPLITTER])
        .hand(0, &[index::DWALIN_WEAPONMASTER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, index::DWALIN_WEAPONMASTER);
    pass_until(&mut e, stack_is_empty);
    let equipment = on_battlefield(&e, p0, index::BONESPLITTER).unwrap();
    let theirs = on_battlefield(&e, p1, index::BONESPLITTER).unwrap();
    let dwalin = on_battlefield(&e, p0, index::DWALIN_WEAPONMASTER).unwrap();
    assert_eq!(counters_on(&e, equipment, CounterKind::Hone), 1);
    assert_eq!(counters_on(&e, theirs, CounterKind::Hone), 0);
    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(dwalin, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(counters_on(&e, equipment, CounterKind::Hone), 2);
    assert_eq!(counters_on(&e, theirs, CounterKind::Hone), 0);
}

#[test]
fn hone_follows_attachments_survives_ability_loss_and_requires_equipment() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[index::BONESPLITTER, index::GRIZZLY_BEARS, llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let equipment = on_battlefield(&e, p0, index::BONESPLITTER).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::put_counters(state, equipment, CounterKind::Hone, 3);
    state.attach(equipment, bear);
    state.refresh_characteristics();
    assert_eq!(pt(&e, bear), (7, 2));
    let state = e.dev_state_mut(p0).unwrap();
    super::keyword_counters::remove_abilities(state, equipment);
    state.refresh_characteristics();
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority")
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        pt(&e, bear),
        (5, 2),
        "the Equipment's +2/+0 is lost; its counter rule remains"
    );
    let state = e.dev_state_mut(p0).unwrap();
    state.attach(equipment, elf);
    state.refresh_characteristics();
    assert_eq!(pt(&e, bear), (2, 2));
    assert_eq!(pt(&e, elf), (4, 1));
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(equipment).unwrap().base_mut().subtypes =
        baylee_core::types::SubtypeSet::EMPTY;
    state.refresh_characteristics();
    assert_eq!(pt(&e, elf), (1, 1));
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::remove_counters(state, equipment, CounterKind::Hone, 3);
    assert!(
        !state
            .effects
            .iter()
            .any(|fx| fx.origin == crate::effects::EffectOrigin::Counter(CounterKind::Hone))
    );
}
