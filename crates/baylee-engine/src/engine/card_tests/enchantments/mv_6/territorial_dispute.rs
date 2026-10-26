//! `cards/enchantments/mv_6/territorial_dispute.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "a1785817-f17b-471b-a63b-866e7972df1f"
fn territorial_dispute() -> CardIndex {
    card_index("a1785817-f17b-471b-a63b-866e7972df1f")
}

/// `Territorial Dispute` (`Coverage::Implemented`):
/// "At the beginning of your upkeep, sacrifice this enchantment unless you sacrifice a land.
/// Players can't play lands."
#[test]
fn territorial_dispute_prevents_playing_lands_and_has_land_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[index::MOUNTAIN],
        &[],
        &[territorial_dispute(), index::MOUNTAIN],
        &[],
    );
    let sac_mountain = lk::obj(&engine, P0, index::MOUNTAIN);
    let hand_mountain = in_hand(&engine, P0, index::MOUNTAIN).expect("Mountain in hand");

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 4), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, territorial_dispute());
    pass_until(&mut engine, stack_is_empty);

    // While Territorial Dispute is on the battlefield, players cannot play lands.
    assert!(
        !priority_offer(&engine).lands.contains(&hand_mountain),
        "Players cannot play lands while Territorial Dispute is on the battlefield"
    );

    // At P0's next upkeep, sacrifice the Mountain on the battlefield to keep Territorial Dispute.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![sac_mountain],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::MOUNTAIN).is_some());
    assert!(on_battlefield(&engine, P0, territorial_dispute()).is_some());

    // At P0's following upkeep with no lands on the battlefield, Territorial Dispute is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, territorial_dispute()).is_some());

    // Once Territorial Dispute leaves the battlefield, P0 can play lands again in main phase.
    reach_main_phase(&mut engine, P0);
    assert!(
        priority_offer(&engine).lands.contains(&hand_mountain),
        "Players can play lands again once Territorial Dispute leaves the battlefield"
    );
}
