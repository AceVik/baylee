//! `cards/enchantments/mv_4/freyalise_s_winds.rs`, played.
//!
//! Oracle:
//! - "Whenever a permanent becomes tapped, put a wind counter on it."
//! - "If a permanent with a wind counter on it would untap during its controller's untap step, remove all wind counters from it instead."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn freyalises_winds() -> CardIndex {
    card_index("ffeb792a-6df0-46c1-badc-4deb857f7d48")
}

#[track_caller]
fn tap_for_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId, land: ObjectId) {
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source: land })
        .expect("the basic land taps for mana");
    lk::settle(engine);
}

#[test]
fn freyalises_winds_puts_wind_counters_on_each_players_tapped_permanents() {
    let mut engine = lk::game(
        SEED,
        &[freyalises_winds(), index::FOREST],
        &[index::FOREST],
        &[],
        &[],
    );
    let own_land = lk::obj(&engine, P0, index::FOREST);
    let opposing_land = lk::obj(&engine, P1, index::FOREST);

    tap_for_mana(&mut engine, P0, own_land);
    assert_eq!(counters_on(&engine, own_land, counters::WIND), 1);

    lk::pass_once(&mut engine);
    tap_for_mana(&mut engine, P1, opposing_land);
    assert_eq!(
        counters_on(&engine, opposing_land, counters::WIND),
        1,
        "the trigger is not limited to its controller's permanents"
    );
}

#[test]
fn ordinary_untap_preserves_wind_and_untap_steps_remove_all_wind_before_untapping() {
    let mut engine = lk::game(
        SEED,
        &[freyalises_winds(), index::FOREST],
        &[],
        &[index::BURST_OF_ENERGY],
        &[],
    );
    let land = lk::obj(&engine, P0, index::FOREST);

    tap_for_mana(&mut engine, P0, land);
    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut engine, P0, index::BURST_OF_ENERGY, &[land], &[]);
    assert!(!lk::tapped(&engine, land));
    assert_eq!(
        counters_on(&engine, land, counters::WIND),
        1,
        "only an untap during the controller's untap step is replaced"
    );

    tap_for_mana(&mut engine, P0, land);
    assert_eq!(counters_on(&engine, land, counters::WIND), 2);
    crate::replacement::put_counters(
        engine.dev_state_mut(P0).expect("the harness may set up"),
        land,
        CounterKind::Charge,
        2,
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        lk::tapped(&engine, land),
        "the first untap step removes counters instead of untapping"
    );
    assert_eq!(counters_on(&engine, land, counters::WIND), 0);
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        2,
        "other kinds of counters remain"
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !lk::tapped(&engine, land),
        "without a wind counter, the following untap step untaps normally"
    );
}

#[test]
fn wind_replacement_follows_the_tapped_permanents_controller() {
    let mut engine = lk::game(SEED, &[freyalises_winds()], &[index::FOREST], &[], &[]);
    let land = lk::obj(&engine, P1, index::FOREST);

    lk::pass_once(&mut engine);
    tap_for_mana(&mut engine, P1, land);
    assert_eq!(counters_on(&engine, land, counters::WIND), 1);

    reach_their_main_phase(&mut engine, P1);
    assert!(lk::tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, counters::WIND), 0);

    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    assert!(!lk::tapped(&engine, land));
}
