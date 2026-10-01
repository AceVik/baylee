//! How many targets a triggered ability's question offers, and how many it
//! lets the controller name (CR 603.3d puts a trigger on the stack through
//! CR 601.2c, so its targets are announced like a spell's).
//!
//! The question carries its bounds as `u8`, and the board it counts does not
//! stop at 255. The Commander table of l29 game 1930 held 201 permanents on
//! turn 24, and a Venser trigger there was asked for one target of at most
//! none, which nothing can answer: its 256 options (those permanents, and
//! 55 abilities on the stack that "target spell or permanent" offered until
//! it was fixed) were counted into a `u8` and wrapped to zero. Only a
//! multiple of 256 wraps to zero, so the board here is built to land on
//! exactly that count, out of permanents alone.

use super::testkit::{
    Duel, RegistryLookup, SEED, card_index, in_hand, keep_mulligans, pass_until, reach_main_phase,
    stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::CardIndex;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// `{2}{U}{U}` "When Venser enters, return target spell or permanent to its
/// owner's hand." A required target among every permanent on the table.
fn venser_shaper_savant() -> CardIndex {
    card_index("0f41cefc-d6ff-4db7-ba35-502b7e081de1")
}

/// 256 legal targets ask for one of them, not for one of none.
///
/// Four Islands pay for Venser, and the opponent's 251 Swamps make the
/// table, Venser included, exactly 256 permanents, every one a legal target
/// of "target spell or permanent" while the stack is empty. The old code
/// stopped the game in `Engine::settle_question` as Venser resolved.
#[test]
fn a_trigger_with_256_legal_targets_asks_for_one_of_them() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let swamps = vec![swamp(); 251];
    let mut engine: Engine<RegistryLookup> = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[venser_shaper_savant()])
        .battlefield(1, &swamps)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let venser = in_hand(&engine, p0, venser_shaper_savant()).expect("Venser is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: venser })
        .expect("four Islands pay {2}{U}{U}");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        stack_is_empty(&engine),
        "Venser resolved; only its trigger asks"
    );
    assert_eq!(player, p0, "Venser's controller aims its trigger");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        256,
        "the board is the count that wrapped"
    );
    assert_eq!(options.len(), 256, "every permanent is a legal target");
    assert!(player_options.is_empty(), "and no player is");
    assert_eq!(
        (min, max),
        (1, 1),
        "one target, out of 256 — a count cut to a `u8` would say at most none"
    );

    // And the question is one a seat can answer: a Swamp goes home.
    let swamp = options
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == p1)
        })
        .expect("one of the 251 Swamps is on the menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![swamp],
                players: vec![],
            },
        )
        .expect("an option the question enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        1,
        "the Swamp was returned to its owner's hand"
    );
}
