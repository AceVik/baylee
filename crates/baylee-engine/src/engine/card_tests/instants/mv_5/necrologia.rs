//! `cards/instants/mv_5/necrologia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn necrologia() -> CardIndex {
    card_index("925f11c3-fd32-4aa7-a130-6554b81ad3eb")
}

#[test]
fn necrologia_pays_x_life_and_draws_x_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[necrologia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    lk::float(&mut engine, p0, &[(ManaColor::Black, 10)]);
    let spell = in_hand(&engine, p0, necrologia()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&spell) && !legal.payable.contains(&spell));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, necrologia()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before - 1 + 3
    );
}

#[test]
fn necrologia_is_not_castable_in_an_opponents_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[], &[necrologia()], &[]);
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == crate::turn::Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    lk::float(&mut e, p0, &[(ManaColor::Black, 10)]);
    let spell = in_hand(&e, p0, necrologia()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&spell) && !legal.payable.contains(&spell));
    assert!(
        e.apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
}
