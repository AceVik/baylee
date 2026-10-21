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

/// "Cumulative upkeep {1}" (CR 702.24a): paid at the first upkeep in the
/// fixture, the second asks {1} for each of two age counters, and declined,
/// Energy Storm is sacrificed.
#[test]
fn energy_storm_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid() {
    let mut engine = lk::game_with(SEED, &[energy_storm()], &[], &[], &[], |e| {
        lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
        lk::yes_no(e, true);
    });
    let storm = lk::obj(&engine, P0, energy_storm());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, P0, "asked of Energy Storm's controller");
    assert_eq!(engine.state().turn.active, P0, "at its own upkeep");
    assert_eq!(
        engine
            .state()
            .object(storm)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 2 },
        "{{1}} for each of the two age counters"
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, energy_storm()).is_some(),
        "unpaid, Energy Storm is sacrificed"
    );
}
