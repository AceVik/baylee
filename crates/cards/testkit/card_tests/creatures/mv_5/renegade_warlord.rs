//! `cards/creatures/mv_5/renegade_warlord.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn renegade_warlord() -> CardIndex {
    card_index("ae63849d-7352-4a2b-9d64-f3005146bd22")
}

/// `Renegade Warlord` ({4}{R}, 3/3 Creature — Human Soldier, `Coverage::Implemented`):
/// `First strike`
/// `Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.`
#[test]
fn renegade_warlord_pumps_other_attacking_creatures_when_it_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[renegade_warlord(), GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let warlord = on_battlefield(&engine, p0, renegade_warlord()).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    assert!(
        engine
            .state()
            .object(warlord)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );

    // Advance to declare attackers; attack with Warlord and Bears, leave Giant back.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (warlord, Defender::Player(p1)),
                    (bears, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let w_ch = engine.state().object(warlord).unwrap().characteristics();
    let b_ch = engine.state().object(bears).unwrap().characteristics();
    let g_ch = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((w_ch.power, w_ch.toughness), (Some(3), Some(3)));
    assert_eq!((b_ch.power, b_ch.toughness), (Some(3), Some(2)));
    assert_eq!((g_ch.power, g_ch.toughness), (Some(3), Some(3)));
}
