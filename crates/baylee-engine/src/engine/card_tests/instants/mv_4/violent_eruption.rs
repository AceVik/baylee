//! `cards/instants/mv_4/violent_eruption.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::{NumberPrompt, TargetRef};
use baylee_core::generated::index;

fn violent_eruption() -> CardIndex {
    card_index("842578ca-0a86-4e96-bfd5-45931488f7c1")
}

/// "Violent Eruption deals 4 damage divided as you choose among any number of
/// targets." An ordinary {1}{R}{R}{R} cast can divide all four across creatures
/// and a player, every announced target receiving at least one.
#[test]
fn violent_eruption_divides_four_across_creatures_and_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[violent_eruption()])
        .battlefield(0, &[mountain(); 4])
        .battlefield(1, &[grizzly_bears(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, violent_eruption());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![p1],
            },
        )
        .unwrap();

    let Pending::ChooseNumber {
        min,
        max,
        reason:
            NumberPrompt::DivideDamage {
                target: TargetRef::Object(target),
                left,
                ..
            },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the first damage share, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(target.object, bears);
    assert_eq!((min, max, left), (1, 2, 4));
    assert!(engine.apply(p0, PlayerAction::ChooseNumber(0)).is_err());
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 1,
            max: 2,
            reason: NumberPrompt::DivideDamage { left: 3, .. },
            ..
        }
    ));
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the last target automatically takes the remaining one"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, p0, violent_eruption()).is_some());
}

/// Damage is divided as the spell is cast, not redistributed on resolution
/// (CR 601.2d, 608.2b). A target removed by a real Lightning Bolt loses its
/// announced three; the surviving player target still takes only one.
#[test]
fn violent_eruption_does_not_reassign_an_illegal_targets_share() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[violent_eruption(), lightning_bolt()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let own_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let target = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, violent_eruption());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![p1],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();

    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, grizzly_bears()).is_some()
    });
    assert_eq!(
        life_of(&engine, p1),
        20,
        "Violent Eruption has not resolved"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "only the announced one remains");
    assert_eq!(engine.state().object(own_bears).unwrap().damage, 0);
    assert!(in_graveyard(&engine, p0, violent_eruption()).is_some());
}

/// Madness {1}{R}{R} exiles the discarded card, then casts that incarnation
/// through the ordinary target and division questions. Prying Eyes supplies a
/// real discard; exactly three red mana remain after paying its blue cost.
#[test]
fn violent_eruption_madness_casts_the_exiled_card_for_three_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[violent_eruption(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 9);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let eruption = in_hand(&engine, p0, violent_eruption()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != eruption)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![eruption, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == eruption
        )
    });
    assert_eq!(engine.state().object(eruption).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        3
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![p1],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(
        engine.state().object(eruption).unwrap().zone,
        Zone::Graveyard,
        "resolving the madness spell does not exile it again"
    );
}
