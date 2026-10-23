//! `cards/creatures/mv_3/haunting_apparition.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn haunting_apparition() -> CardIndex {
    card_index("ced9130c-2865-49df-abdd-84069aa1b180")
}

/// Haunting Apparition (ced9130c-2865-49df-abdd-84069aa1b180) — {1}{U}{B} 1+*/2:
/// "Flying. As Haunting Apparition enters, choose an opponent."
#[test]
fn haunting_apparition_enters_with_flying_and_chooses_an_opponent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), swamp(), swamp()])
        .hand(0, &[haunting_apparition()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, haunting_apparition());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let apparition = on_battlefield(&engine, P0, haunting_apparition()).unwrap();
    let obj = engine.state().object(apparition).unwrap();
    assert!(obj.characteristics().keywords.contains(KeywordSet::FLYING));
    assert_eq!(obj.chosen_opponent(), Some(P1));
    assert_eq!(
        pt(&engine, apparition),
        (1, 2),
        "1 plus no green creature cards"
    );
}
