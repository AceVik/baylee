//! A spell cast before its mana is made, and paid for while it is cast
//! (CR 601.2g): `LegalActions::payable`.
//!
//! The owner's report (08.10.2026): a land whose mana sets off a trigger
//! (City of Brass, or a painland beside "whenever you're dealt damage")
//! tapped *before* the cast put its trigger on the stack first, and a
//! creature could no longer be cast. Made inside the cast, the mana's
//! triggers wait until the spell has been cast (CR 601.2i, 603.3) and go on
//! the stack above it.

use super::testkit::*;
use super::*;
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn city_of_brass() -> CardIndex {
    card_index("f25351e3-539b-4bbc-b92d-6480acf4d722")
}
fn adarkar_wastes() -> CardIndex {
    card_index("d5ad26cc-2bdb-46b7-b8bf-dd099d5fa09b")
}
fn living_artifact() -> CardIndex {
    card_index("4ff9af56-ac18-4966-9e48-183e1ca1c2d0")
}
fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}
/// Crystalline Sliver, `{W}{U}`: a creature, so cast only with an empty
/// stack, which is what a trigger stacked before it takes away.
fn sliver() -> CardIndex {
    card_index("ba3aa1eb-722a-47d3-83be-96daddb50265")
}

const P0: PlayerId = PlayerId::new(0);

fn table(seed: u64, board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn legal(engine: &Engine<RegistryLookup>) -> crate::choice::LegalActions {
    match engine.pending() {
        Pending::Priority { legal, .. } => (**legal).clone(),
        other => panic!("expected priority, got {other:?}"),
    }
}

fn on_board(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P0, card).expect("on the battlefield")
}

/// The stack, bottom first, as (is the spell, the source of an ability).
fn stack(engine: &Engine<RegistryLookup>) -> Vec<(bool, Option<ObjectId>)> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter_map(|&id| engine.state().object(id))
        .map(|o| (o.ability.is_none(), o.ability.map(|a| a.source)))
        .collect()
}

fn tap(engine: &mut Engine<RegistryLookup>, source: ObjectId, ability: u32, color: ManaColor) {
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: ability,
            },
        )
        .expect("a mana ability inside the window");
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseColor(color))
            .expect("the colour was offered");
    }
}

fn tap_intrinsic(engine: &mut Engine<RegistryLookup>, source: ObjectId) {
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source })
        .expect("a basic land's mana");
}

/// Island and City of Brass for a `{W}{U}` creature, nothing floating. The
/// creature is not castable (the pool is empty) and is payable; cast, it
/// opens a window owing `{W}{U}`; the City's trigger waits through it and
/// goes on the stack above the cast creature.
#[test]
fn a_city_of_brass_tapped_inside_the_cast_triggers_above_the_spell() {
    let mut engine = table(6012, &[island(), city_of_brass()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    let offer = legal(&engine);
    assert!(!offer.castable.contains(&card), "nothing is floating");
    assert!(offer.payable.contains(&card), "{offer:?}");

    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("a payable card is cast");
    let window = legal(&engine);
    assert!(window.payable.is_empty() && window.castable.is_empty());
    assert!(window.has_mana_source(), "the window offers the mana");

    let (island_id, city) = (
        on_board(&engine, island()),
        on_board(&engine, city_of_brass()),
    );
    tap_intrinsic(&mut engine, island_id);
    tap(&mut engine, city, 1, ManaColor::White);
    assert!(
        stack(&engine).is_empty(),
        "the City's trigger waits while the cast is being paid for"
    );
    let life = engine.state().players[0].life;
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("passing pays");
    assert_eq!(
        stack(&engine),
        [(true, None), (false, Some(city))],
        "the creature, and the City's trigger above it"
    );
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Stack)
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life - 1);
    assert!(on_battlefield(&engine, P0, sliver()).is_some());
}

/// A permanent that triggers on its controller being dealt damage, beside a
/// painland: Living Artifact on a Sol Ring, Plains and Adarkar Wastes for
/// the creature. The Wastes' damage is dealt inside the cast, and the
/// Aura's trigger goes on the stack above the creature.
#[test]
fn a_damage_trigger_set_off_by_a_painland_waits_until_the_spell_is_cast() {
    let mut engine = table(
        6013,
        &[forest(), sol_ring(), plains(), adarkar_wastes()],
        &[living_artifact(), sliver()],
    );
    let forest_id = on_board(&engine, forest());
    let ring = on_board(&engine, sol_ring());
    tap_intrinsic(&mut engine, forest_id);
    cast_with_floating(&mut engine, P0, living_artifact());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_board(&engine, living_artifact());

    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    assert!(legal(&engine).payable.contains(&card));
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("a payable card is cast");
    let (plains_id, wastes) = (
        on_board(&engine, plains()),
        on_board(&engine, adarkar_wastes()),
    );
    tap_intrinsic(&mut engine, plains_id);
    tap(&mut engine, wastes, 1, ManaColor::Blue);
    assert!(stack(&engine).is_empty(), "the Aura's trigger waits");
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("passing pays");
    assert_eq!(stack(&engine), [(true, None), (false, Some(aura))]);
}

/// Passing a window the pool does not cover reverses the cast (CR 601.2h,
/// 732.1): the card goes back to the hand and the player has priority
/// again (CR 732.2). The Wastes' tap is not taken back: it dealt damage,
/// and the engine reverses only taps (`give_back_window`), which CR 732.1
/// allows ("may"). The blue stays floating.
#[test]
fn a_cast_its_window_cannot_pay_goes_back_to_the_hand() {
    let mut engine = table(6014, &[adarkar_wastes()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    assert!(legal(&engine).payable.contains(&card));
    let life = engine.state().players[0].life;
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("a payable card is cast");
    let wastes = on_board(&engine, adarkar_wastes());
    tap(&mut engine, wastes, 1, ManaColor::Blue);
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("passing short reverses the cast");
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Hand)
    );
    assert!(stack(&engine).is_empty());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "the player who cast keeps priority: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().players[0].life, life - 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}

/// What the pool already pays is `castable` and never `payable`, and a seat
/// with no mana ability left has nothing payable.
#[test]
fn payable_is_only_what_the_pool_cannot_pay_and_the_seat_can_still_make() {
    let mut engine = table(6015, &[island(), plains()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    assert!(legal(&engine).payable.contains(&card));
    tap_all_mana(&mut engine, P0);
    let offer = legal(&engine);
    assert!(offer.castable.contains(&card));
    assert!(!offer.payable.contains(&card));

    let mut engine = table(6016, &[island()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    tap_all_mana(&mut engine, P0);
    let offer = legal(&engine);
    assert!(!offer.castable.contains(&card) && offer.payable.is_empty());
}
