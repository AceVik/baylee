//! `cards/creatures/mv_4/balduvian_horde.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn balduvian_horde() -> CardIndex {
    card_index("fd2a94d3-901d-4b46-aa9b-a30ed6dea6bd")
}

/// `Balduvian Horde` ({2}{R}{R}, 5/5 Creature — Human Barbarian,
/// `Coverage::Partial`) sacrifices itself when it enters unless you discard a
/// card from your hand.
#[test]
fn balduvian_horde_stays_when_discard_is_paid_and_sacrifices_when_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 8])
        .hand(
            0,
            &[
                balduvian_horde(),
                balduvian_horde(),
                ironclaw_orcs(),
                ironclaw_orcs(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = in_hand(&engine, p0, ironclaw_orcs()).unwrap();
    let first = in_hand(&engine, p0, balduvian_horde()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(first).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(pt(&engine, first), (5, 5));
    assert_eq!(engine.state().object(fodder).unwrap().zone, Zone::Graveyard);

    // Second Balduvian Horde: decline the discard, so it is sacrificed.
    let second = in_hand(&engine, p0, balduvian_horde()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(second).unwrap().zone, Zone::Graveyard);
}
