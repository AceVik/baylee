//! `cards/creatures/mv_5/lu_su_wu_advisor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lu_su_wu_advisor() -> CardIndex {
    card_index("59238ba1-fde4-4e43-9f24-b24125e59de0")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Lu Su, Wu Advisor (59238ba1-fde4-4e43-9f24-b24125e59de0) — `{3}{U}{U}` 1/2
/// Legendary Creature — Human Advisor:
/// "`{T}`: Draw a card. Activate only during your turn, before attackers are declared."
#[test]
fn lu_su_wu_advisor_taps_to_draw_a_card_only_on_your_turn_before_attackers() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[lu_su_wu_advisor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lu_su_wu_advisor());
    pass_until(&mut engine, stack_is_empty);
    let lu_su = on_battlefield(&engine, P0, lu_su_wu_advisor()).expect("Lu Su entered");
    assert_eq!(pt(&engine, lu_su), (1, 2));

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !ability_offered(&engine, lu_su, 0),
        "cannot activate during opponent's turn"
    );

    reach_their_main_phase(&mut engine, P0);
    let before = hand_size(&engine, P0);
    let lib_before = library_size(&engine, P0);
    assert!(ability_offered(&engine, lu_su, 0));

    activate(&mut engine, P0, lu_su_wu_advisor(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(hand_size(&engine, P0), before + 1);
    assert_eq!(library_size(&engine, P0), lib_before - 1);
}

#[test]
fn lu_su_wu_advisor_cannot_activate_in_second_main_phase() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[lu_su_wu_advisor()])
        .start();
    keep_mulligans(&mut engine);
    let lu_su = on_battlefield(&engine, P0, lu_su_wu_advisor()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(!ability_offered(&engine, lu_su, 0));
}
