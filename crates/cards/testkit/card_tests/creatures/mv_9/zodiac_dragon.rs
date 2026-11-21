//! `cards/creatures/mv_9/zodiac_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zodiac_dragon() -> CardIndex {
    card_index("e5c2cf0f-c1c0-4fd6-9b5f-7d4aa25246a7")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

fn act_of_treason() -> CardIndex {
    card_index("9d08af23-9f4a-4097-9abc-3b17475ab744")
}

/// Zodiac Dragon (e5c2cf0f-c1c0-4fd6-9b5f-7d4aa25246a7) — `{7}{R}{R}` 8/8
/// Creature — Dragon:
/// "When this creature is put into your graveyard from the battlefield, you may
/// return it to your hand."
#[test]
fn zodiac_dragon_may_return_to_hand_when_put_into_your_graveyard_from_battlefield() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[zodiac_dragon()])
        .hand(1, &[murder(), murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zodiac_dragon());
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, P0, zodiac_dragon()).expect("Zodiac Dragon entered");
    assert_eq!(pt(&engine, dragon), (8, 8));

    // First Murder: accept `MayDo` -> returns to P0's hand.
    reach_their_main_phase(&mut engine, P1);
    let spare_swamp = on_battlefield(&engine, P1, swamp()).unwrap();
    let _ = spare_swamp;
    cast_from_hand(&mut engine, P1, murder());
    aim_at(&mut engine, P1, dragon);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, zodiac_dragon()).is_some());
    assert!(in_graveyard(&engine, P0, zodiac_dragon()).is_none());

    // Recast and decline `MayDo` -> stays in P0's graveyard.
    reach_their_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, zodiac_dragon());
    pass_until(&mut engine, stack_is_empty);
    let dragon2 = on_battlefield(&engine, P0, zodiac_dragon()).unwrap();

    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, murder());
    aim_at(&mut engine, P1, dragon2);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, zodiac_dragon()).is_some());
    assert!(in_hand(&engine, P0, zodiac_dragon()).is_none());
}

#[test]
fn zodiac_dragon_dying_under_opponent_control_goes_to_owner_graveyard_so_does_not_trigger() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[zodiac_dragon()])
        .battlefield(
            1,
            &[
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(1, &[act_of_treason(), murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let dragon = on_battlefield(&engine, P0, zodiac_dragon()).unwrap();
    tap_all_mana_but(&mut engine, P1, Some(swamp()));
    cast_with_floating(&mut engine, P1, act_of_treason());
    aim_at(&mut engine, P1, dragon);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(dragon).unwrap().controller, P1);

    // Under P1's control, P1 controls the dies ability, so "your graveyard" means P1's
    // graveyard (CR 109.5); the card goes to P0's graveyard and does not trigger.
    cast_from_hand(&mut engine, P1, murder());
    aim_at(&mut engine, P1, dragon);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, zodiac_dragon()).is_some());
    assert!(in_hand(&engine, P0, zodiac_dragon()).is_none());
}
