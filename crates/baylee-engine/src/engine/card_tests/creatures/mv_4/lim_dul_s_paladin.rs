//! Lim-Dûl's Paladin — `{2}{B}{R}` 0/3 Creature — Human Knight (Alliances)
//!
//! Trample
//! At the beginning of your upkeep, you may discard a card. If you don't,
//! sacrifice this creature and draw a card.
//! Whenever this creature becomes blocked, it gets +6/+3 until end of turn.
//! Whenever this creature attacks and isn't blocked, it assigns no combat
//! damage this turn and defending player loses 4 life.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lim_dul_s_paladin() -> CardIndex {
    card_index("c4161335-1150-4f0d-bd2b-9211efb3bea6")
}

#[test]
fn lim_dul_s_paladin_unblocked_loses_4_life_and_upkeep_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[swamp(), swamp()])
        .battlefield(0, &[lim_dul_s_paladin()])
        .start();
    keep_mulligans(&mut engine);

    // On turn 1 upkeep, pay the discard cost to keep Lim-Dûl's Paladin.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard choice for Lim-Dul's Paladin upkeep");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    let paladin = on_battlefield(&engine, p0, lim_dul_s_paladin()).unwrap();

    // Attack with Lim-Dûl's Paladin; p1 does not block -> defending player loses 4 life and takes 0 combat damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(paladin, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
    });

    assert_eq!(engine.state().players[1].life, 16);

    // On p0's next turn upkeep, decline the discard (choose 0 cards) -> sacrifice Lim-Dûl's Paladin and draw a card.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, lim_dul_s_paladin()).is_none());
    assert!(in_graveyard(&engine, p0, lim_dul_s_paladin()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

#[test]
fn lim_dul_s_paladin_gets_plus_6_plus_3_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[swamp()])
        .battlefield(0, &[lim_dul_s_paladin()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    // Pay the discard on turn 1 upkeep.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    let paladin = on_battlefield(&engine, p0, lim_dul_s_paladin()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(paladin, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, paladin)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
    });

    assert_eq!(
        pt(&engine, paladin),
        (6, 6),
        "0/3 gets +6/+3 until end of turn when blocked"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "Grizzly Bears died in combat to 6/6 trample Lim-Dul's Paladin"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "4 trample damage assigned to defending player (6 power - 2 toughness)"
    );
}
