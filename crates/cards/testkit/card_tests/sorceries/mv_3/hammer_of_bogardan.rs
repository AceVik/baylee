//! `cards/sorceries/mv_3/hammer_of_bogardan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hammer_of_bogardan() -> CardIndex {
    card_index("3089bdd7-87e6-4d0e-ad98-c32e6f861c30")
}

/// Hammer of Bogardan (3089bdd7-87e6-4d0e-ad98-c32e6f861c30) — {1}{R}{R} Sorcery:
/// "Hammer of Bogardan deals 3 damage to any target. {2}{R}{R}{R}: Return
/// Hammer of Bogardan from your graveyard to your hand. Activate only during
/// your upkeep."
#[test]
fn hammer_of_bogardan_deals_three_damage_to_any_target() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[hammer_of_bogardan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, hammer_of_bogardan());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

/// "{2}{R}{R}{R}: Return this card from your graveyard to your hand. Activate
/// only during your upkeep.": not offered in the main phase it was cast in nor
/// in the opponent's upkeep, offered in its owner's own upkeep, and it then
/// returns to the hand.
#[test]
fn hammer_of_bogardan_returns_from_the_graveyard_in_its_owners_upkeep_only() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[hammer_of_bogardan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, hammer_of_bogardan());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let hammer = in_graveyard(&engine, P0, hammer_of_bogardan()).unwrap();
    assert!(!ability_offered(&engine, hammer, 1), "main phase");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(
        !ability_offered(&engine, hammer, 1),
        "the opponent's upkeep"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    assert!(ability_offered(&engine, hammer, 1), "its owner's upkeep");
    activate(&mut engine, P0, hammer_of_bogardan(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, hammer_of_bogardan()).is_some());
    assert!(in_graveyard(&engine, P0, hammer_of_bogardan()).is_none());
}
