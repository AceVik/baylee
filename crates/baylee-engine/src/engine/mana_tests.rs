//! Mana production, now that one effect covers all of it.
//!
//! Seven `Effect` variants used to each fix all three of the independent
//! questions a mana line asks — which colors, how much, and what it may be
//! spent on — and none of the cards below had an engine test. They do now,
//! one per answer the unified effect has to get right.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, reach_main_phase};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}
fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}
fn command_tower() -> CardIndex {
    card_index("0895c9b7-ae7d-4bb3-af17-3b75deb50a25")
}
fn cavern_of_souls() -> CardIndex {
    card_index("89ca686a-7c72-4d8f-9290-e89635624a83")
}
fn reflecting_pool() -> CardIndex {
    card_index("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
}
fn badlands() -> CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}

/// Activates printed ability `index` of `card`.
#[track_caller]
fn activate(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex, index: u32) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, ai)| {
            *ai == index
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the ability is offered");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability activates");
}

/// "Add X mana in any combination of colors, where X is the number of
/// Allies you control." Two Allies is two mana and two picks — the old
/// non-combination path added the whole amount per pick and then asked
/// again, which paid X² mana for a card that promises X.
#[test]
fn any_combination_adds_one_mana_per_pick() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[harabaz_druid(), halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, harabaz_druid(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any of the five colours");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("first mana");
    assert!(
        matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the second mana is a second pick, not a repeat of the first"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("second mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 2, "two Allies, two mana");
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
}

/// "Add one mana of any color in your commander's color identity." There is
/// no commander in a duel, so there is no color to choose and the ability
/// still has to resolve — colorless is what is left of it.
#[test]
fn command_tower_without_a_commander_makes_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[command_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, command_tower(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one option is no choice: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
}

/// "Spend this mana only to cast a creature spell of the chosen type, and
/// that spell can't be countered." The rider rides on the mana, so it has
/// to survive the color choice: the pool keeps it as restricted mana, not
/// as an ordinary red.
#[test]
fn cavern_of_souls_mana_stays_restricted_after_the_colour_choice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls()])
        .start();
    // "As this land enters, choose a creature type" — the preset puts it on
    // the battlefield, so that choice lands in the middle of the mulligans.
    for _ in 0..3 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::ChooseSubtype { player, options } => {
                engine
                    .apply(player, PlayerAction::ChooseSubtype(options[0]))
                    .expect("a creature type is chosen");
            }
            other => panic!("expected a mulligan or the type choice, got {other:?}"),
        }
    }
    reach_main_phase(&mut engine, p0);

    // Ability 0 is the plain {C}; ability 1 is the restricted any-colour.
    activate(&mut engine, p0, cavern_of_souls(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("colour chosen");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "restricted mana is not free red"
    );
    let restricted = pool.restricted();
    assert_eq!(restricted.len(), 1);
    assert_eq!(restricted[0].color, ManaColor::Red);
    assert_eq!(restricted[0].amount, 1);
    assert!(
        engine
            .state()
            .restriction_info
            .contains_key(&restricted[0].restriction.0),
        "the spend restriction is registered, or nothing can check it"
    );
}

/// "Add one mana of any type that a land you control could produce." A
/// Reflecting Pool reads that off the *other* lands — it produces nothing
/// by itself, so it must not read its own promise back. Next to a Badlands
/// it makes black or red, and nothing else.
#[test]
fn reflecting_pool_offers_what_the_other_lands_produce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(14, forest())
        .battlefield(0, &[reflecting_pool(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, reflecting_pool(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the Badlands' two colours, not the rainbow the Pool would promise itself"
    );
}

/// With no other land, the Pool has nothing to reflect and produces no mana
/// at all — the ability still resolves (CR 106.6a), it just adds nothing.
#[test]
fn a_lone_reflecting_pool_produces_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(15, forest())
        .battlefield(0, &[reflecting_pool()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, reflecting_pool(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "nothing to choose from: {:?}",
        engine.pending()
    );
    assert!(
        engine.state().players[0].mana_pool.is_empty(),
        "a Pool reflecting only itself is not a rainbow land"
    );
}
