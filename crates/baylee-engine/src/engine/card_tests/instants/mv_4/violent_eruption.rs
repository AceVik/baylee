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

/// The spell cast off its {1}{R}{R}{R} and stopped at its target question,
/// three creatures on the other side: with both players, five legal targets.
#[track_caller]
fn cast_at_a_full_board() -> (Engine<RegistryLookup>, [ObjectId; 3]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[violent_eruption()])
        .battlefield(0, &[mountain(); 4])
        .battlefield(1, &[grizzly_bears(), index::HILL_GIANT, gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creatures = [
        on_battlefield(&engine, p1, grizzly_bears()).unwrap(),
        on_battlefield(&engine, p1, index::HILL_GIANT).unwrap(),
        on_battlefield(&engine, p1, gray_ogre()).unwrap(),
    ];
    cast_from_hand(&mut engine, p0, violent_eruption());
    (engine, creatures)
}

/// "Any number" stops at one: a single target takes all four, and there is
/// nothing to divide, so no share is asked for.
#[test]
fn violent_eruption_puts_all_four_on_a_single_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, _) = cast_at_a_full_board();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one target takes the whole amount, got {:?}",
        engine.pending()
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 16);
    assert_eq!(life_of(&engine, p0), 20);
    assert!(in_graveyard(&engine, p0, violent_eruption()).is_some());
}

/// Each target must receive at least one of the four (CR 601.2d), so four is
/// the most the spell can have. The offer says so, a fifth is refused with
/// nothing chosen, and four targets take exactly one each.
#[test]
fn violent_eruption_takes_four_targets_and_refuses_a_fifth() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, [bears, giant, ogre]) = cast_at_a_full_board();
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the target question, got {:?}", engine.pending());
    };
    assert_eq!((options.len(), player_options.len()), (3, 2));
    assert_eq!((min, max), (0, 4));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears, giant, ogre],
                    players: vec![p0, p1],
                },
            )
            .is_err(),
        "a fifth target cannot receive any of the four"
    );
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant, ogre],
                players: vec![p1],
            },
        )
        .unwrap();
    while let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() {
        assert_eq!((min, max), (1, 1), "four targets leave one apiece");
        engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    }

    pass_until(&mut engine, stack_is_empty);
    for creature in [bears, giant, ogre] {
        assert_eq!(engine.state().object(creature).unwrap().damage, 1);
    }
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(life_of(&engine, p0), 20);
    assert!(in_graveyard(&engine, p0, violent_eruption()).is_some());
}

/// "Any number" also includes none (CR 115.6). The spell is paid for, has no
/// target to lose, resolves, deals nothing and goes to the graveyard.
#[test]
fn violent_eruption_with_no_targets_resolves_and_deals_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, creatures) = cast_at_a_full_board();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no target leaves nothing to divide, got {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!stack_is_empty(&engine), "the spell was cast");

    pass_until(&mut engine, stack_is_empty);
    for creature in creatures {
        assert_eq!(engine.state().object(creature).unwrap().damage, 0);
    }
    assert_eq!(life_of(&engine, p0), 20);
    assert_eq!(life_of(&engine, p1), 20);
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
