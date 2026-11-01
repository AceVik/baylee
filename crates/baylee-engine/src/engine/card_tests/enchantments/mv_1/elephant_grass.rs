//! `cards/enchantments/mv_1/elephant_grass.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elephant_grass() -> CardIndex {
    card_index("53cc4b93-538d-4193-8bbb-5b73d3ce9d79")
}

fn scathe_zombies() -> CardIndex {
    baylee_core::generated::index::SCATHE_ZOMBIES
}

/// Walks to the next cumulative-upkeep question and returns its price.
#[track_caller]
fn walk_to_price(engine: &mut Engine<RegistryLookup>) -> baylee_core::mana::ManaCost {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, PlayerId::new(0), "\"your upkeep\"");
    cost
}

/// "Black creatures can't attack you": the black Zombies is not offered as an
/// attacker, a nonblack Bears is (the filter is the color and nothing wider).
#[test]
fn elephant_grass_stops_black_creatures_from_attacking_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[scathe_zombies(), grizzly_bears()])
        .hand(0, &[elephant_grass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let opp_zombies = on_battlefield(&engine, p1, scathe_zombies()).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, elephant_grass());
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        !attackers.contains(&opp_zombies),
        "black: cannot attack you"
    );
    assert!(attackers.contains(&opp_bear), "nonblack: may attack");
}

/// Cumulative upkeep {1}: the first upkeep adds an age counter and asks {1},
/// paid; the second asks {2} (one {1} per age counter), refused, and the
/// Grass is sacrificed.
#[test]
fn elephant_grass_is_sacrificed_when_its_growing_upkeep_goes_unpaid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[elephant_grass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, elephant_grass());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(walk_to_price(&mut engine), baylee_core::mana!("{1}"));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, elephant_grass()).is_some());

    assert_eq!(walk_to_price(&mut engine), baylee_core::mana!("{2}"));
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, elephant_grass()).is_none());
    assert!(in_graveyard(&engine, p0, elephant_grass()).is_some());
}
