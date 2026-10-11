//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rust_counters_the_activated_ability_of_an_artifact_sacrificed_as_its_cost() {
    let p0 = PlayerId::new(0);
    let (mut e, ability) =
        crate::engine::card_tests::counter_helpers::artifact_activation(index::RUST);
    let before = e
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(1)))
        .len();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::RUST);
    aim_at(&mut e, p0, ability);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(1)))
            .len(),
        before,
        "countered activation draws nothing"
    );
}
#[test]
fn rust_excludes_triggered_abilities_and_nonartifact_sources_but_reads_a_changed_source() {
    use baylee_cards_dsl::Filter;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::RUST])
        .battlefield(
            1,
            &[
                index::MIND_STONE,
                index::SHIVAN_DRAGON,
                index::SOLEMN_SIMULACRUM,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let stone = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
    let dragon = on_battlefield(&e, p1, index::SHIVAN_DRAGON).unwrap();
    let solemn = on_battlefield(&e, p1, index::SOLEMN_SIMULACRUM).unwrap();
    let artifact = e.push_ability_to_stack(p1, stone, 1, smallvec::SmallVec::new());
    let creature = e.push_ability_to_stack(p1, dragon, 0, smallvec::SmallVec::new());
    let trigger = e.push_ability_to_stack(p1, solemn, 0, smallvec::SmallVec::new());
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::RUST);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert_eq!(options, &vec![artifact]);
    assert!(!options.contains(&creature));
    assert!(!options.contains(&trigger));
    aim_at(&mut e, p0, artifact);
    // Source characteristics are current, rather than frozen at activation.
    let s = e.dev_state_mut(p0).unwrap();
    s.object_mut(stone).unwrap().base_mut().types = baylee_core::types::TypeSet::CREATURE;
    let source = in_hand(&e, p0, index::RUST).unwrap_or(stone);
    assert!(!crate::eval::matches(
        &Filter::ActivatedAbilityFrom(&Filter::ARTIFACT),
        e.state(),
        e.state().object(artifact).unwrap(),
        p0,
        source
    ));
}
