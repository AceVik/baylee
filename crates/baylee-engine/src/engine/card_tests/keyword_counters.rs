//! Keyword counters participate in layer 6 and keep their own timestamps.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_cards_dsl::{Duration, Modifier};
use baylee_core::generated::index;
use baylee_core::ids::EffectId;

pub(super) fn remove_abilities(state: &mut crate::state::GameState, object: ObjectId) -> EffectId {
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: PlayerId::new(0),
        origin: EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::object(state, object),
        modifier: Modifier::LoseAllAbilities,
    })
}

#[test]
fn counters_use_timestamp_order_and_new_placement_refreshes_the_whole_kind() {
    for (kind, keyword) in [
        (CounterKind::Lifelink, KeywordSet::LIFELINK),
        (CounterKind::Flying, KeywordSet::FLYING),
        (CounterKind::Shadow, KeywordSet::SHADOW),
        (CounterKind::Indestructible, KeywordSet::INDESTRUCTIBLE),
    ] {
        let p0 = PlayerId::new(0);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        crate::replacement::put_counters(state, bear, kind, 1);
        state.refresh_characteristics();
        assert!(
            state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        let first_loss = remove_abilities(state, bear);
        state.refresh_characteristics();
        assert!(
            !state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        crate::replacement::put_counters(state, bear, kind, 1);
        state.refresh_characteristics();
        assert!(
            state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        assert_eq!(
            state
                .effects
                .iter()
                .filter(|fx| fx.origin == EffectOrigin::Counter(kind))
                .count(),
            1
        );
        let second_loss = remove_abilities(state, bear);
        crate::replacement::remove_counters(state, bear, kind, 1);
        state.refresh_characteristics();
        assert!(
            !state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword),
            "removal must not refresh timestamps"
        );
        state
            .effects
            .remove_where(|fx| fx.id == first_loss || fx.id == second_loss);
        state.refresh_characteristics();
        assert!(
            state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        crate::replacement::remove_counters(state, bear, kind, 1);
        state.refresh_characteristics();
        assert!(
            !state
                .object(bear)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        assert!(
            !state
                .effects
                .iter()
                .any(|fx| fx.origin == EffectOrigin::Counter(kind))
        );
    }
}

#[test]
fn keyword_counter_rules_end_on_departure_and_do_not_affect_another_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS, llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::put_counters(state, bear, CounterKind::Flying, 1);
    state.refresh_characteristics();
    assert!(
        !state
            .object(elf)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
    state
        .move_object(
            bear,
            crate::zone::ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Setup,
        )
        .unwrap();
    state
        .move_object(
            bear,
            crate::zone::ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Setup,
        )
        .unwrap();
    state.refresh_characteristics();
    assert!(
        !state
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
    assert!(
        !state
            .effects
            .iter()
            .any(|fx| matches!(fx.origin, EffectOrigin::Counter(_)))
    );
}

#[test]
fn minas_morgul_shadow_and_wraith_outlive_the_land_but_end_when_the_counter_is_gone() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                minas_morgul_dark_fortress(),
                index::GRIZZLY_BEARS,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, minas_morgul_dark_fortress()).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().set_tapped(land, false);
    tap_all_mana_but(&mut e, p0, Some(minas_morgul_dark_fortress()));
    activate(&mut e, p0, minas_morgul_dark_fortress(), 1);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    kill(&mut e, land);
    assert!(keywords(&e, bear).contains(KeywordSet::SHADOW));
    assert!(
        e.state()
            .object(bear)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WRAITH)
    );
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::remove_counters(state, bear, CounterKind::Shadow, 1);
    // Removing and replacing the last counter in one resolution must not
    // restart the land's expired type-changing effect.
    crate::replacement::put_counters(state, bear, CounterKind::Shadow, 1);
    state.refresh_characteristics();
    assert!(
        !state
            .object(bear)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WRAITH)
    );
    assert!(keywords(&e, bear).contains(KeywordSet::SHADOW));
}

#[test]
fn tyrite_sanctum_places_real_indestructible_counters_after_its_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::TYRITE_SANCTUM,
                index::GRIZZLY_BEARS,
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::TYRITE_SANCTUM).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state
        .object_mut(bear)
        .unwrap()
        .base_mut()
        .subtypes
        .insert(baylee_core::generated::subtypes::creature::GOD);
    state.refresh_characteristics();
    tap_all_mana_but(&mut e, p0, Some(index::TYRITE_SANCTUM));
    activate(&mut e, p0, index::TYRITE_SANCTUM, 2);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(land).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(counters_on(&e, bear, CounterKind::Indestructible), 1);
    assert!(keywords(&e, bear).contains(KeywordSet::INDESTRUCTIBLE));
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::remove_counters(state, bear, CounterKind::Indestructible, 1);
    state.refresh_characteristics();
    assert!(!keywords(&e, bear).contains(KeywordSet::INDESTRUCTIBLE));
}
