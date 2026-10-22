//! `cards/instants/mv_1/ertai_s_meddling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ertai_s_meddling() -> CardIndex {
    card_index("609c529a-b3b3-46d5-b9d0-ae0117507d6e")
}

/// Ertai's Meddling (`609c529a-b3b3-46d5-b9d0-ae0117507d6e`) — {X}{U} Instant (Partial):
/// "X can't be 0. Target spell's controller exiles it with X delay counters on it..."
#[test]
fn ertai_s_meddling_exiles_target_spell_when_x_is_at_least_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[ertai_s_meddling()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    let spell_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P1);
    let meddling = in_hand(&engine, P1, ertai_s_meddling()).unwrap();
    engine
        .apply(P1, PlayerAction::CastSpell { card: meddling })
        .unwrap();
    engine.apply(P1, PlayerAction::ChooseNumber(1)).unwrap();
    aim_at(&mut engine, P1, spell_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 1);
}
