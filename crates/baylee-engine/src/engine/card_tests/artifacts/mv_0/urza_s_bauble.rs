//! Urza's Bauble — `{0}` Artifact (Ice Age)
//!
//! `{T}`, Sacrifice Urza's Bauble: Look at a card at random in target player's
//! hand. You draw a card at the beginning of the next turn's upkeep.
//! (`Coverage::Partial`: reveals a random card from target player's hand to all
//! players via `Effect::RevealRandomFromHand`.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_bauble() -> CardIndex {
    card_index("17dbbca3-ac1c-4d4e-9618-3e66ac3ccd24")
}

#[test]
fn urza_s_bauble_sacrifices_to_reveal_random_card_and_draw_at_next_turns_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[urza_s_bauble()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bauble_hand = in_hand(&engine, p0, urza_s_bauble()).unwrap();
    let opp_card = in_hand(&engine, p1, llanowar_elves()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bauble_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let bauble = on_battlefield(&engine, p0, urza_s_bauble()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bauble,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(bauble).unwrap().zone,
        Zone::Graveyard,
        "Urza's Bauble is sacrificed as cost"
    );
    pass_until(&mut engine, stack_is_empty);

    // Opponent's card remains in hand after being revealed.
    assert_eq!(engine.state().object(opp_card).unwrap().zone, Zone::Hand);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Advance to p1's upkeep: delayed trigger draws 1 card for p0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Urza's Bauble draws a card at the next turn's upkeep"
    );
}
