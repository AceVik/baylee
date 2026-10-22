//! `cards/creatures/mv_4/balduvian_dead.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn balduvian_dead() -> CardIndex {
    card_index("18d6b89a-3588-43d9-a247-4747a7d784fc")
}

/// `Balduvian Dead` ({3}{B}, 2/3 Creature — Zombie, `Coverage::Partial`)
/// activates for `{2}{R}, Exile a creature card from your graveyard` to create
/// a 3/1 black and red Graveborn creature token with haste.
#[test]
fn balduvian_dead_exiles_creature_from_graveyard_to_create_hasty_graveborn_token() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
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

    cast_from_hand(&mut engine, p0, balduvian_dead());
    pass_until(&mut engine, stack_is_empty);
    let dead = on_battlefield(&engine, p0, balduvian_dead()).expect("Balduvian Dead enters");
    assert_eq!(pt(&engine, dead), (2, 3));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dead,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(corpse).unwrap().zone, Zone::Exile);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let graveborn = tokens[0];
    assert_eq!(pt(&engine, graveborn), (3, 1));
    assert!(keywords(&engine, graveborn).contains(KeywordSet::HASTE));

    // Graveborn token attacks immediately thanks to haste.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(graveborn, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 17);
}
