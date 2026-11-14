//! `cards/creatures/mv_4/chainflinger.rs`, played.
//!
//! Oracle:
//! - "{1}{R}, {T}: This creature deals 1 damage to any target."
//! - "Threshold — {2}{R}, {T}: This creature deals 2 damage to any target.
//!   Activate only if seven or more cards are in your graveyard."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chainflinger() -> CardIndex {
    card_index("cfaf4871-2157-495d-bca8-d169bd1f7ae3")
}

fn ping(
    engine: &mut Engine<RegistryLookup>,
    source: ObjectId,
    ability_index: u32,
    target: ObjectId,
) {
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    aim_at(engine, P0, target);
    pass_until(engine, stack_is_empty);
}

#[test]
fn chainflinger_pings_one_damage_and_two_at_threshold() {
    let mut engine = Duel::new(SEED, forest())
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
                chainflinger(),
                chainflinger(),
            ],
        )
        .battlefield(1, &[index::HILL_GIANT, index::GRIZZLY_BEARS])
        .hand(0, &[index::SHOCK, lightning_bolt()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let flingers = all_on_battlefield(&engine, P0, chainflinger());
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, flingers[0]), (2, 2));

    // 5 seeded + Shock at P1 (20 -> 18) = 6 cards (< 7).
    seed_graveyard(&mut engine, P0, 5);
    cast_from_hand(&mut engine, P0, index::SHOCK);
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

    tap_all_mana(&mut engine, P0);
    assert!(ability_offered(&engine, flingers[0], 0));
    assert!(!ability_offered(&engine, flingers[0], 1));

    // Ability 0 ({1}{R}, {T}): 1 damage to Hill Giant (3/3 -> 1 damage marked).
    ping(&mut engine, flingers[0], 0, giant);
    assert_eq!(engine.state().object(giant).unwrap().damage, 1);

    // Lightning Bolt at P1 (18 -> 15) puts 7th card into graveyard -> Threshold on.
    cast_with_floating(&mut engine, P0, lightning_bolt());
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
    assert!(ability_offered(&engine, flingers[1], 1));

    // Ability 1 ({2}{R}, {T}): 2 damage kills Grizzly Bears (2/2) and taps flinger #2.
    ping(&mut engine, flingers[1], 1, bears);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(is_tapped(&engine, flingers[1]));
    assert!(!ability_offered(&engine, flingers[1], 0));
}
