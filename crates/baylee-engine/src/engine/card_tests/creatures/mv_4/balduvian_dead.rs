//! Balduvian Dead — `{3}{B}` 2/3 Creature — Zombie (Masters Edition II)
//!
//! `{2}{R}`, Exile a creature card from your graveyard: Create a 3/1 black and red Graveborn
//! creature token with haste. Sacrifice it at the beginning of the next end step.
//!
//! The ability is off the card: nothing can name the token it creates for the delayed sacrifice,
//! and a token that stayed would be more than printed. This test pins that, so the day the piece
//! exists it has to move (with a test that the token goes at the end step).

#[allow(clippy::wildcard_imports)]
use super::*;

fn balduvian_dead() -> CardIndex {
    card_index("18d6b89a-3588-43d9-a247-4747a7d784fc")
}

#[test]
fn balduvian_dead_is_a_two_three_without_the_token_ability_it_cannot_yet_play_correctly() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[balduvian_dead(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let corpse = in_hand(&engine, p0, grizzly_bears()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            corpse,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, balduvian_dead());
    pass_until(&mut engine, stack_is_empty);
    let dead = on_battlefield(&engine, p0, balduvian_dead()).expect("Balduvian Dead enters");
    assert_eq!(pt(&engine, dead), (2, 3));

    // {2}{R} and a creature card in the graveyard are both there, and the ability is not: a Graveborn
    // that nothing sacrifices at the end step would stay for good.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: dead,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the token ability is not on the card"
    );
    assert!(tokens_of(&engine, p0).is_empty());
    assert_eq!(
        engine.state().object(corpse).unwrap().zone,
        Zone::Graveyard,
        "nothing was exiled"
    );
}
