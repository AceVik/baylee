//! `cards/lands/utility/detection_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Targeting permission leaves hexproof intact.
#[test]
fn detection_tower_targets_opponent_creatures_without_removing_hexproof() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[detection_tower(), forest()])
        .battlefield(1, &[sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, detection_tower()).expect("tower on battlefield");
    let caryatid = on_battlefield(&engine, p1, sylvan_caryatid()).expect("caryatid on battlefield");
    assert!(keywords(&engine, caryatid).contains(KeywordSet::HEXPROOF));

    // Float {1} green mana from the Forest while keeping Detection Tower untapped.
    tap_mana_except(&mut engine, p0, tower);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(!is_tapped(&engine, tower));

    activate(&mut engine, p0, detection_tower(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, caryatid).contains(KeywordSet::HEXPROOF));
    assert!(!crate::eval::untargetable_by(engine.state(), caryatid, p0));
    assert!(is_tapped(&engine, tower));
}

#[test]
fn detection_tower_permission_is_personal_dynamic_and_ends_at_cleanup() {
    use baylee_cards_dsl::TargetSpec;
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[detection_tower(), forest()])
        .battlefield(1, &[sylvan_caryatid(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tower = on_battlefield(&e, p0, detection_tower()).unwrap();
    let creature = on_battlefield(&e, p1, sylvan_caryatid()).unwrap();
    let land = on_battlefield(&e, p1, forest()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(land).unwrap().base_mut().keywords = KeywordSet::HEXPROOF;
    state.refresh_characteristics();
    protect_player(&mut e, p1);
    assert!(
        !crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p0).contains(&p1)
    );
    assert!(
        crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p1).contains(&p1)
    );
    tap_mana_except(&mut e, p0, tower);
    activate(&mut e, p0, detection_tower(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(!crate::eval::untargetable_by(e.state(), creature, p0));
    assert!(crate::eval::untargetable_by(e.state(), creature, p2));
    assert!(
        crate::eval::untargetable_by(e.state(), land, p0),
        "noncreature hexproof is unchanged"
    );
    assert!(
        crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p0).contains(&p1)
    );
    assert!(
        !crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p2).contains(&p1)
    );
    let state = e.dev_state_mut(p0).unwrap();
    let name = state.names.intern("later creature");
    let later = state.create_bare(
        p1,
        crate::object::ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let base = state.object_mut(later).unwrap().base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(1);
    base.toughness = Some(1);
    base.keywords = KeywordSet::HEXPROOF;
    state.refresh_characteristics();
    assert!(!crate::eval::untargetable_by(e.state(), later, p0));
    bury(&mut e, &[tower]);
    assert!(!crate::eval::untargetable_by(e.state(), later, p0));
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(creature)
        .unwrap()
        .base_mut()
        .keywords = KeywordSet::HEXPROOF.union(KeywordSet::SHROUD);
    e.dev_state_mut(p0).unwrap().refresh_characteristics();
    assert!(
        crate::eval::untargetable_by(e.state(), creature, p0),
        "shroud still applies"
    );
    reach_their_main_phase(&mut e, p1);
    assert!(crate::eval::untargetable_by(e.state(), later, p0));
    assert!(
        !crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p0).contains(&p1)
    );
}

fn protect_player(e: &mut Engine<RegistryLookup>, controller: PlayerId) {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
    use baylee_core::ids::EffectId;
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller,
        origin: EffectOrigin::Resolution,
        layer: Layer::Text,
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(&Filter::Any),
        modifier: Modifier::PlayerHexproof {
            who: baylee_cards_dsl::PlayerRel::You,
        },
    });
}

#[test]
fn player_hexproof_allows_its_players_and_teammates_spells() {
    use baylee_cards_dsl::TargetSpec;
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3).team(0, 7).team(1, 7).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    protect_player(&mut e, p1);
    for caster in [p0, p1] {
        assert!(
            crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, caster)
                .contains(&p1)
        );
        assert!(
            !crate::eval::target_player_options(e.state(), &TargetSpec::AnyOpponent, caster)
                .contains(&p1)
        );
    }
    assert!(
        !crate::eval::target_player_options(e.state(), &TargetSpec::AnyPlayer, p2).contains(&p1)
    );
}
