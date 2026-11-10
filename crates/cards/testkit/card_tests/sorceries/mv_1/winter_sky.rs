//! Winter Sky — `{R}` Sorcery (Homelands)
//!
//! Flip a coin. If you win the flip, Winter Sky deals 1 damage to each creature and each player.
//! If you lose the flip, each player draws a card.

#[allow(clippy::wildcard_imports)]
use super::*;

fn winter_sky() -> CardIndex {
    card_index("fb150d6a-5f81-4fd8-92b2-22a9615bfd4f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn winter_sky_deals_damage_on_won_flip_and_draws_for_each_player_on_lost_flip() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut saw_win = false;
    let mut saw_loss = false;

    for seed in 1..=20 {
        let mut engine = Duel::new(seed, mountain())
            .hand(0, &[winter_sky()])
            .battlefield(0, &[mountain(), grizzly_bears()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let sky = in_hand(&engine, p0, winter_sky()).unwrap();
        let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
        let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
        let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
        let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

        tap_all_mana(&mut engine, p0);
        engine
            .apply(p0, PlayerAction::CastSpell { card: sky })
            .unwrap();
        pass_until(&mut engine, stack_is_empty);

        if engine.state().players[0].life == 19 {
            saw_win = true;
            assert_eq!(engine.state().players[1].life, 19);
            assert_eq!(engine.state().object(bears).unwrap().damage, 1);
            assert_eq!(
                engine.state().object(ogre).unwrap().damage,
                1,
                "each creature, the opponent's too"
            );
            // A won flip draws nothing: the Sky left p0's hand and no card came in.
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                p0_hand_before - 1
            );
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
                p1_hand_before
            );
        } else {
            saw_loss = true;
            // A lost flip deals no damage to anything.
            assert_eq!(engine.state().players[0].life, 20);
            assert_eq!(engine.state().players[1].life, 20);
            assert_eq!(engine.state().object(bears).unwrap().damage, 0);
            assert_eq!(engine.state().object(ogre).unwrap().damage, 0);
            // Each player draws: the caster too (the Sky left the hand, one card came in).
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                p0_hand_before
            );
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
                p1_hand_before + 1
            );
        }
        if saw_win && saw_loss {
            break;
        }
    }

    assert!(saw_win, "tested Winter Sky won-flip branch");
    assert!(saw_loss, "tested Winter Sky lost-flip branch");
}
