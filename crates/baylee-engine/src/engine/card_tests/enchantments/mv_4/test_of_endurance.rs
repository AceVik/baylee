//! `cards/enchantments/mv_4/test_of_endurance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn test_of_endurance_wins_the_game_at_your_upkeep_with_fifty_or_more_life() {
    let test_card = card_index("ffdad8fb-69ee-40d3-b2eb-b0aee0a5becc");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[test_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, test_card).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, test_card).is_some());

    engine.state.players[0].life = 50;

    pass_until(&mut engine, |e| {
        e.state().game_end
            == Some(crate::win::GameResult {
                winner: Some(crate::win::Victor::Player(p0)),
                reason: crate::win::EndReason::EffectWin,
            })
    });
}
