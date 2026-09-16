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
        ("daybound", K::DAYBOUND),             // progress::day_night_statics
        ("nightbound", K::NIGHTBOUND),         // progress::day_night_statics
    ]
};

/// [`ENFORCED`] as one set.
fn enforced() -> baylee_cards_dsl::KeywordSet {
    let mut enforced = baylee_cards_dsl::KeywordSet::EMPTY;
    for (_, k) in ENFORCED {
        enforced = enforced.union(*k);
    }
    enforced
}

/// A card may not claim a keyword no rule reads: it would look supported
/// on the card, in the view, and in the roadmap, and change nothing at the
/// table.
#[test]
fn no_card_claims_a_keyword_the_engine_ignores() {
    let enforced = enforced();
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let unknown = def.all_keywords().difference(enforced);
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

/// The same claim, for the permanents no card prints.
///
/// A `TokenDef` carries a `KeywordSet` the layer system reads exactly as it
/// reads a face's — which is what a 1/1 white Bird with flying *is* — so a
/// token claiming a keyword no rule reads is inert in the same way a card
/// would be, and `generated::ALL` above cannot see it. `tokens.rs` sits
/// beside `cards/` rather than inside it, and that is the door a sweep keeps
/// walking past: it is how the Blood token kept an ability the engine will
/// never offer through a whole commit written to find exactly that
/// (`offer_tests::no_token_carries_an_ability_the_engine_will_never_offer`).
///
/// Four of the fourteen tokens claim anything at all — flying on the Bird
/// and the Angel, changeling on both Shapeshifters — so this passes today
/// and is here for the fifteenth.
#[test]
fn no_token_claims_a_keyword_the_engine_ignores() {
    let enforced = enforced();
    for token in baylee_cards::tokens::ALL {
        let unknown = token.keywords.difference(enforced);
        assert_eq!(
            unknown.bits(),
            0,
            "the {} token declares a keyword no engine rule reads (bits {:#x}); \
             implement it and add it to ENFORCED, or take it off the token",
            token.name,
            unknown.bits(),
        );
    }
}

/// Twining Twins — {2}{U}{U}, flying, vigilance, **ward {1}**.
fn twining_twins() -> baylee_core::ids::CardIndex {
    card_index("105aea98-8eb9-4fb2-a0cb-7c7513317c5b")
}
/// Path to Exile — {W}, "exile target creature": the cheapest spell in the
/// pool that points at a creature and nothing else.
fn path_to_exile() -> baylee_core::ids::CardIndex {
    card_index("d683d985-9888-4d21-8b5f-69e69ce4a03b")
}

/// Drives a duel to the moment Twining Twins' ward asks p1 for its tax, and
/// hands the engine back standing on that question.
///
/// Three tests share it because the three things worth proving about ward
/// are one question and two answers — and until the resolution seam was
/// fixed, *nothing reached the answers at all*. `AwaitingOp::PlayerMayPay`
/// was never suspended, so `resume_tax_choice`, the pool debit and the
/// counter-the-spell fallback had never executed in any game this engine
/// has played.
#[track_caller]
fn ward_asks_for_its_tax(seed: u64) -> (Engine<RegistryLookup>, baylee_core::ids::ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, plains())
        .battlefield(0, &[twining_twins()])
        // Two, because the tax is only ever *asked* of a seat that could pay
        // it: `PlayerMayPayOr` runs its fallback outright off an empty pool,
        // and a board with one Plains on it would prove the rule by countering
        // the spell for the wrong reason.
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let twins = on_battlefield(&engine, p0, twining_twins()).expect("the warded creature");
    cast_from_hand(&mut engine, p1, path_to_exile());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Path asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "their spell, their choice");
    assert!(
        options.contains(&twins),
        "ward does not make a creature untargetable (CR 702.21a): {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![twins],
            },
        )
        .expect("Path points at the warded creature");

    // The trigger goes on the stack above the spell and asks before it.
    for _ in 0..20 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { .. },
                ..
            }
        ) {
            return (engine, twins);
        }
        // Anything that is not priority means the tax was never asked and
        // Path has already resolved. Naming the finding here matters: the
        // fault this test was written for showed up as `ChooseAttackers`,
        // which reads like a harness mistake rather than a missing question.
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "ward asked for nothing and the spell resolved — got {:?}. \
                 The trigger is collected and does reach the stack; it dies \
                 in resolution if `PlayerMayPayOr` resolves its `PlayerRel` \
                 through `eval::players`, which has no answer for \
                 `ControllerOfTarget`.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("ward never asked for its tax");
}

/// Ward {1} (CR 702.21a): an opponent's spell that targets the permanent is
/// countered unless **that opponent** pays the tax.
///
/// The tax is asked of the caster, not of the ward's controller, which is
/// the half of the rule a `PlayerRel` can get backwards without anything
/// noticing — so the assertion names the seat as well as the number.
#[test]
fn ward_taxes_the_opponent_who_targeted_it() {
    let p1 = PlayerId::new(1);
    let (engine, _twins) = ward_asks_for_its_tax(17);
    let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("the helper returns standing on the tax question")
    };
    assert_eq!(*mana, 1, "Twining Twins prints ward {{1}}");
    assert_eq!(
        *player, p1,
        "the tax is paid by the spell's controller, not by the creature's \
         (CR 702.21a)"
    );
}

/// The declined half: "countered unless that player pays" is a *counter*,
/// not a fizzle. Path goes to its owner's graveyard and the creature it
/// pointed at is still on the battlefield.
#[test]
fn ward_declined_counters_the_spell_that_targeted_it() {
    let p1 = PlayerId::new(1);
    let (mut engine, twins) = ward_asks_for_its_tax(19);
    let pool_before = engine.state().players[1].mana_pool.total();

    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is an answer");
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty()
    });

    assert_eq!(
        engine
            .state()
            .object(twins)
            .map(|o| o.zone)
            .expect("the creature still exists"),
        crate::zone::Zone::Battlefield,
        "the spell was countered, so it never exiled anything",
    );
    assert!(
        in_graveyard(&engine, p1, path_to_exile()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)",
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before,
        "declining spends nothing",
    );
}

/// The paid half: the mana leaves the pool and the spell resolves.
#[test]
fn ward_paid_lets_the_spell_through_and_costs_the_mana() {
    let p1 = PlayerId::new(1);
    let (mut engine, twins) = ward_asks_for_its_tax(23);
    let pool_before = engine.state().players[1].mana_pool.total();
    assert!(
        pool_before >= 1,
        "the second Plains is what makes the tax payable"
    );

    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("paying is an answer");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before - 1,
        "ward {{1}} costs one mana, taken from the floating pool",
    );

    // Path resolves, exiles the creature — and then offers its *controller*
    // the basic-land search, which is the ramp half of the same card and the
    // reason this cannot simply pass priority to the end. The search is
    // declined here; that it is asked at all is entry 30's other victim
    // proving itself alive.
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards { player, .. } => engine
                .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                .expect("the search may be declined"),
            Pending::Priority { player, .. } => {
                if engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Stack)
                    .is_empty()
                {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the spell resolves: {other:?}"),
        }
    }
    assert_eq!(
        engine
            .state()
            .object(twins)
            .map(|o| o.zone)
            .expect("the creature still exists as a card"),
        crate::zone::Zone::Exile,
        "the tax was paid, so Path resolved and exiled its target",
    );
}
