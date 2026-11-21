//! `cards/creatures/mv_6/dromar_the_banisher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dromar_the_banisher() -> CardIndex {
    card_index("35ad7f82-a1b1-4aa7-a8b0-3f99ca2cff52")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Dromar, the Banisher (35ad7f82-a1b1-4aa7-a8b0-3f99ca2cff52) — `{3}{W}{U}{B}` 6/6
/// Legendary Creature — Dragon:
/// - Flying
/// - Whenever Dromar deals combat damage to a player, you may pay `{2}{U}`. If you do,
///   choose a color, then return all creatures of that color to their owners' hands.
#[test]
fn dromar_the_banisher_pays_two_blue_on_combat_damage_to_bounce_all_creatures_of_chosen_color() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                plains(),
                island(),
                swamp(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .hand(0, &[dromar_the_banisher()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, dromar_the_banisher());
    pass_until(&mut engine, stack_is_empty);

    let dromar = on_battlefield(&engine, P0, dromar_the_banisher()).expect("Dromar entered");
    assert_eq!(pt(&engine, dromar), (6, 6));
    assert!(keywords(&engine, dromar).contains(KeywordSet::FLYING));

    // Combat 1: choose Green -> bounces green creatures on BOTH sides, leaves Dromar and Hill Giant.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, dromar, P1);

    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_hand(&engine, P0, llanowar_elves()).is_some());
    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P0, dromar_the_banisher()).is_some());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());

    // Combat 2: choosing Blue bounces multicolored WUB Dromar itself!
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, dromar, P1);
    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_hand(&engine, P0, dromar_the_banisher()).is_some());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());
}
