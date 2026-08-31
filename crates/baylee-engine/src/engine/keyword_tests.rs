//! Keyword abilities that live in the *timing* and *combat* rules rather
//! than in a card's own text, tested through the whole engine because that
//! is the only place they exist.
//!
//! Keywords whose effect is purely a state predicate (hexproof, shroud,
//! defender, double strike) are unit-tested next to the rule that reads
//! them, in `eval` and `combat`. What is left here needs a real turn to
//! happen in.

use super::testkit::*;
use super::*;

fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
/// Restoration Angel — {3}{W}, flash.
fn restoration_angel() -> baylee_core::ids::CardIndex {
    card_index("dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa")
}
/// Ondu Cleric — {1}{W}, no flash: the control for the flash tests.
fn ondu_cleric() -> baylee_core::ids::CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

/// Hands `seat` priority during the *other* seat's first main phase with
/// every land they control tapped for mana, and returns what they may do.
#[track_caller]
fn legal_on_the_opponents_turn(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
) -> crate::choice::LegalActions {
    let active = PlayerId::new(1 - seat.get());
    reach_main_phase(engine, active);
    engine
        .apply(active, PlayerAction::PassPriority)
        .expect("the active seat passes");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "priority did not reach the non-active seat");
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("lands tap for mana");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after tapping, got {:?}",
            engine.pending()
        )
    };
    *legal
}

/// Flash (CR 702.8a): a creature with flash may be cast whenever an
/// instant could be — here, in an opponent's main phase.
#[test]
fn flash_lets_a_creature_be_cast_on_the_opponents_turn() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(31, plains())
        .battlefield(1, &[plains(), plains(), plains(), plains()])
        .hand(1, &[restoration_angel()])
        .start();
    keep_mulligans(&mut engine);
    let legal = legal_on_the_opponents_turn(&mut engine, p1);
    let angel = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    assert!(
        legal.castable.contains(&angel),
        "a flash creature was not castable on the opponent's turn"
    );
}

/// The control: same colour, same zone, enough mana, no flash — and it
/// has to wait. Without this the test above would also pass if the
/// timing check had simply been deleted.
#[test]
fn without_flash_a_creature_waits_for_its_own_main_phase() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(32, plains())
        .battlefield(1, &[plains(), plains(), plains(), plains()])
        .hand(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let legal = legal_on_the_opponents_turn(&mut engine, p1);
    let cleric = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    assert!(
        !legal.castable.contains(&cleric),
        "a creature without flash was castable on the opponent's turn"
    );
}

/// Every keyword the engine actually reads somewhere, with the rule that
/// reads it. A keyword absent from this table is a bit nobody looks at.
///
/// `KeywordSet` is deliberately larger than this: it has room for the
/// keywords the pool will grow into. The danger is the gap between the two
/// — a card printing `KeywordSet::INFECT` today would compile, ship, and do
/// nothing at all, which is exactly how hexproof, shroud, defender and
/// flash sat unenforced on fifteen cards until 2026-08-31. The test below
/// closes that gap: add the keyword to a card and the build fails until
/// some rule reads it.
const ENFORCED: &[(&str, baylee_cards_dsl::KeywordSet)] = {
    use baylee_cards_dsl::KeywordSet as K;
    &[
        ("flying", K::FLYING),                 // combat::can_block
        ("first strike", K::FIRST_STRIKE),     // combat::strikes_now
        ("double strike", K::DOUBLE_STRIKE),   // combat::strikes_now
        ("deathtouch", K::DEATHTOUCH),         // combat::lethal_damage
        ("haste", K::HASTE),                   // combat::summoning_sick
        ("hexproof", K::HEXPROOF),             // eval::untargetable_by
        ("shroud", K::SHROUD),                 // eval::untargetable_by
        ("indestructible", K::INDESTRUCTIBLE), // sba::run
        ("lifelink", K::LIFELINK),             // combat::deal_combat_damage
        ("menace", K::MENACE),                 // combat::can_block
        ("reach", K::REACH),                   // combat::can_block
        ("trample", K::TRAMPLE),               // combat::assign_attacker_damage
        ("vigilance", K::VIGILANCE),           // Engine::declare_attackers
        ("defender", K::DEFENDER),             // combat::can_attack
        ("flash", K::FLASH),                   // casting::can_cast
        ("prowess", K::PROWESS),               // trigger.rs (synthetic)
        ("changeling", K::CHANGELING),         // layers::recompute_with
        ("unblockable", K::UNBLOCKABLE),       // combat::can_block
        ("uncounterable", K::UNCOUNTERABLE),   // resolve (counter effects)
        ("rebound", K::REBOUND),               // progress.rs (rider)
    ]
};

/// A card may not claim a keyword no rule reads: it would look supported
/// on the card, in the view, and in the roadmap, and change nothing at the
/// table.
#[test]
fn no_card_claims_a_keyword_the_engine_ignores() {
    let mut enforced = baylee_cards_dsl::KeywordSet::EMPTY;
    for (_, k) in ENFORCED {
        enforced = enforced.union(*k);
    }
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let unknown = def.keywords.difference(enforced);
        assert_eq!(
            unknown.bits(),
            0,
            "{} ({oracle_id}) declares a keyword no engine rule reads (bits {:#x}); \
             implement it and add it to ENFORCED, or take it off the card",
            def.faces[0].name,
            unknown.bits(),
        );
    }
}
