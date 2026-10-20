//! `cards/enchantments/mv_2/energy_storm.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "Prevent all damage that would be dealt by instant and sorcery spells."
//! - "Creatures with flying don't untap during their controllers' untap steps."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn energy_storm() -> CardIndex {
    card_index("567d3de7-8d56-4b6d-a59a-f8674172f595")
}

#[test]
fn energy_storm_stops_creatures_with_flying_from_untapping() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS],
        &[energy_storm()],
        &[],
    );
    let flyer = lk::obj(&engine, P1, index::AIR_ELEMENTAL);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, energy_storm());
    lk::settle(&mut engine);

    engine
        .dev_state_mut(P1)
        .unwrap()
        .object_mut(flyer)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine
        .dev_state_mut(P1)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(
        lk::tapped(&engine, flyer),
        "Air Elemental has flying and does not untap during its controller's untap step"
    );
    assert!(
        !lk::tapped(&engine, bears),
        "Grizzly Bears without flying untaps normally"
    );
}
