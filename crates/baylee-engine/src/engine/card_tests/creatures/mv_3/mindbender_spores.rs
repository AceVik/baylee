//! `cards/creatures/mv_3/mindbender_spores.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0e3b7ec1-3760-4ac4-81ef-1ab389392f1b"
fn mindbender_spores() -> CardIndex {
    card_index("0e3b7ec1-3760-4ac4-81ef-1ab389392f1b")
}

#[test]
fn mindbender_spores_blocking_puts_4_fungus_counters_and_prevents_untap_while_removing_at_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[mindbender_spores()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let spores = lk::obj(&engine, P0, mindbender_spores());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spores, bears)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(counters::FUNGUS),
        4
    );
    assert!(is_tapped(&engine, bears));

    // Advance to P0's turn and then back to P1's main phase.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);

    assert!(
        is_tapped(&engine, bears),
        "Grizzly Bears did not untap during P1's untap step while it had a fungus counter"
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(counters::FUNGUS),
        3,
        "One fungus counter was removed at the beginning of P1's upkeep"
    );
}
