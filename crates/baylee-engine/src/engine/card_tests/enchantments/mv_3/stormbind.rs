//! Stormbind — `{1}{R}{G}` Enchantment (Ice Age)
//!
//! `{2}`, Discard a card at random: Stormbind deals 2 damage to any target.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stormbind() -> CardIndex {
    card_index("78f50668-36fc-4911-84f7-93667436b0c7")
}

#[test]
fn stormbind_discards_at_random_to_deal_two_damage_to_creature_or_player_and_requires_card_in_hand()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[stormbind(), mountain(), forest()])
        .battlefield(
            0,
            &[
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Stormbind ({1}{R}{G}) with 6 mana still floating.
    let sb_hand = in_hand(&engine, p0, stormbind()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: sb_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let sb_id = on_battlefield(&engine, p0, stormbind()).expect("Stormbind on battlefield");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("Grizzly Bears on battlefield");

    // First activation targeting Grizzly Bears: discards 1 card at random from the 2 cards in hand.
    assert!(ability_offered(&engine, sb_id, 0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sb_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "one card discarded at random as activation cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Graveyard,
        "Grizzly Bears takes 2 lethal damage from Stormbind"
    );

    // Second activation targeting player p1: discards the last card in hand.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sb_id,
                ability_index: 0,
            },
        )
        .unwrap();
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
        engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "second activation discarded the final card in hand"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "Stormbind dealt 2 damage to target player"
    );

    // With an empty hand, Stormbind's ability cannot be activated even with {2} floating.
    assert!(
        !ability_offered(&engine, sb_id, 0),
        "Stormbind cannot be activated with an empty hand"
    );
}
