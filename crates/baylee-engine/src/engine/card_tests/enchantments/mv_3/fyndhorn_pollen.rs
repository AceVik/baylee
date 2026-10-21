//! `cards/enchantments/mv_3/fyndhorn_pollen.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "All creatures get -1/-0."
//! - "{1}{G}: All creatures get -1/-0 until end of turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fyndhorn_pollen() -> CardIndex {
    card_index("2f18967d-c2cd-449d-a288-df6343672743")
}

fn power(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<i16> {
    engine.state().object(id).unwrap().characteristics().power
}

/// "All creatures": the opponent's Bears as well as its controller's, both
/// by the static ability and by the activation, which ends with the turn.
#[test]
fn fyndhorn_pollen_debuffs_all_creatures_statically_and_when_activated() {
    let mut engine = lk::game_with(
        SEED,
        &[fyndhorn_pollen(), index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
            lk::yes_no(e, true);
        },
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    assert_eq!(
        power(&engine, bears),
        Some(1),
        "static ability gives all creatures -1/-0"
    );
    assert_eq!(
        power(&engine, their_bears),
        Some(1),
        "the opponent's creatures included"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    activate(&mut engine, P0, fyndhorn_pollen(), 2);
    lk::settle(&mut engine);

    assert_eq!(
        power(&engine, bears),
        Some(0),
        "activated ability gives all creatures an additional -1/-0 until end of turn"
    );
    assert_eq!(power(&engine, their_bears), Some(0));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        power(&engine, bears),
        Some(1),
        "the activation ended with the turn; the static ability did not"
    );
    assert_eq!(power(&engine, their_bears), Some(1));
}

/// "Cumulative upkeep {1}" (CR 702.24a): paid at the first upkeep in the
/// fixture, the second asks {1} for each of two age counters, and declined,
/// the Pollen is sacrificed.
#[test]
fn fyndhorn_pollen_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid() {
    let mut engine = lk::game_with(SEED, &[fyndhorn_pollen()], &[], &[], &[], |e| {
        lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
        lk::yes_no(e, true);
    });
    let pollen = lk::obj(&engine, P0, fyndhorn_pollen());

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
    assert_eq!(player, P0, "asked of the Pollen's controller");
    assert_eq!(engine.state().turn.active, P0, "at its own upkeep");
    assert_eq!(
        engine
            .state()
            .object(pollen)
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
        in_graveyard(&engine, P0, fyndhorn_pollen()).is_some(),
        "unpaid, Fyndhorn Pollen is sacrificed"
    );
}
