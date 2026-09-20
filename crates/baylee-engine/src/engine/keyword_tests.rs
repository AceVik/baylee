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

/// Badlands — `{T}: Add {B} or {R}`, the cheapest source that *asks*.
fn badlands() -> baylee_core::ids::CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}

/// Hands `seat` priority during the *other* seat's first main phase with
/// every mana source they control tapped, and returns what they may do.
///
/// "Every land" until #159, which is what it said and what it did; the kit
/// takes both offer lists now, so an artifact or a mana creature on this
/// board is tapped too.
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
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "priority did not reach the non-active seat");
    tap_all_mana(engine, seat);
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
        // Two, so that the seat still has a mana floating when the tax is
        // asked and answers it out of the pool. The question is put either
        // way now (CR 605.3a opens a payment window against an empty pool),
        // and these three tests are about ward rather than about the window
        // — `a_taxed_seat_may_make_its_mana_after_the_question` is the one
        // that walks through it.
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

/// The same ward tax, asked of a seat that has **not** made its mana yet:
/// one land still untapped and nothing floating.
///
/// The fixture taps exactly one Plains rather than every one, which is what
/// separates it from [`ward_asks_for_its_tax`]. That helper seats two lands
/// and floats both because, until CR 605.3a was implemented, the question was
/// only ever put to a seat whose pool already covered it.
#[track_caller]
fn ward_asks_a_seat_that_has_not_made_its_mana(
    seed: u64,
) -> (Engine<RegistryLookup>, baylee_core::ids::ObjectId) {
    ward_asks_a_seat_whose_spare_land_is(seed, plains())
}

/// The same fixture with the spare land named, because *what that land is*
/// decides whether making its mana asks a question — which is the whole of
/// #167. A Plains adds its mana outright; Badlands prints `{T}: Add {B} or
/// {R}` and suspends a resolution of its own to ask which.
#[track_caller]
fn ward_asks_a_seat_whose_spare_land_is(
    seed: u64,
    spare: baylee_core::ids::CardIndex,
) -> (Engine<RegistryLookup>, baylee_core::ids::ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, plains())
        .battlefield(0, &[twining_twins()])
        .battlefield(1, &[plains(), spare])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let twins = on_battlefield(&engine, p0, twining_twins()).expect("the warded creature");
    // The Plains by name, not `mana_abilities.first()`: the spare is a
    // parameter now, so "whichever is first" would tap a different land
    // depending on what the caller seated — and the point of the fixture is
    // that the spare is the one still standing when the tax is asked.
    let one = on_battlefield(&engine, p1, plains()).expect("a Plains to pay for Path");
    assert!(
        matches!(engine.pending(), Pending::Priority { legal, .. } if legal
            .mana_abilities
            .contains(&one)),
        "the Plains is offered as the CR 305.6 shortcut it is: {:?}",
        engine.pending()
    );
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: one })
        .expect("tapping one land is legal");
    let spell = in_hand(&engine, p1, path_to_exile()).expect("Path is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: spell })
        .expect("one Plains pays for Path");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![twins],
            },
        )
        .expect("Path points at the warded creature");

    for _ in 0..20 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { .. },
                ..
            }
        ) {
            assert_eq!(
                engine.state().players[1].mana_pool.total(),
                0,
                "the fixture is only worth anything with an empty pool",
            );
            return (engine, twins);
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected before the tax: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!(
        "ward never asked a seat with an empty pool. Before CR 605.3a was \
         implemented it never did: `PlayerMayPayOr` ran its fallback outright \
         against a pool that did not already cover the tax."
    );
}

/// A payment window decides what the engine does next, so an engine holding
/// one must not hash the same as an engine that is not.
///
/// [`Engine::snapshot_hash`] is the determinism handle a host compares — a
/// replay against its recording, one machine against another — so a field
/// that decides a continuation and is missing from it is a divergence that
/// compares equal. It is **not** what either loop detector reads: the engine
/// finds loops with Brent over `GameState::loop_signature`, and gamehost's
/// harness keys on `state().snapshot_hash()` beside a pending fingerprint.
/// Neither of those calls this method, and nothing in this tree does — which
/// is the reason the omission survived and not a reason it is harmless.
/// Inside a
/// window the legal actions are narrowed to mana abilities and passing closes
/// the window instead of counting toward the round; outside one the same seat
/// with the same board is being asked a yes-or-no. Two engines one step apart
/// there hashed identically.
///
/// Two halves, because the realistic pair alone does not name the field. The
/// pair is how a game reaches the defect — and it differs in `pending` too,
/// which nothing hashes either (#86), so on its own it would be satisfied by
/// a fix to something else and would stop meaning what it says the day that
/// arrives. The second half changes **only** this field and is what pins it.
///
/// It sets the window to **seat 0** deliberately. An `Option<PlayerId>` folded
/// in as "the seat number, or zero for none" collides exactly there, and a
/// test written on seat 1 would pass over it. See
/// [`automation_is_part_of_the_engine_snapshot`](super::automation_tests) for
/// the sibling this is modelled on.
#[test]
fn a_payment_window_is_part_of_the_engine_snapshot() {
    let p1 = PlayerId::new(1);
    let (asked, _) = ward_asks_a_seat_that_has_not_made_its_mana(31);
    let (mut open, _) = ward_asks_a_seat_that_has_not_made_its_mana(31);
    open.apply(p1, PlayerAction::YesNo(true))
        .expect("saying they will pay opens the window");

    assert!(asked.payment_window().is_none(), "one is still being asked");
    assert_eq!(
        open.payment_window(),
        Some((p1, 1)),
        "and the other is inside a window for the tax it just agreed to"
    );
    assert_eq!(
        asked.state().snapshot_hash(),
        open.state().snapshot_hash(),
        "nothing moved on the board between them, which is the point: the \
         difference is entirely in what the engine will do next"
    );
    assert_ne!(
        asked.snapshot_hash(),
        open.snapshot_hash(),
        "a seat inside a payment window is a different engine state"
    );

    // The same claim with nothing else moving, so this cannot be satisfied by
    // a fix to a neighbouring field. Seat 0 because that is where a careless
    // fold collides with `None`.
    let (before, mut after) = (
        ward_asks_a_seat_that_has_not_made_its_mana(31).0,
        ward_asks_a_seat_that_has_not_made_its_mana(31).0,
    );
    // The window carries the resolution it was opened over (#167), so this
    // half borrows the one the tax question already suspended rather than
    // inventing a `Resolution` that no game would produce.
    let suspended = Box::new(
        after
            .resolution
            .clone()
            .expect("standing on the tax question, which suspended one"),
    );
    after.mana_window = Some(PaymentWindow {
        player: PlayerId::new(0),
        suspended,
    });
    assert_ne!(
        before.snapshot_hash(),
        after.snapshot_hash(),
        "an open window on seat 0 hashes as no window at all"
    );
}

/// And the resolution that window is *holding* is part of the state too.
///
/// #167 is what makes this need saying rather than assuming. That resolution
/// used to live in [`Engine::resolution`], which `snapshot_hash` already
/// folds in; the fix moved it into the window, so a fold written only for
/// the old home would have stopped hashing it on the day it was repaired —
/// two engines inside a window over different taxes comparing equal. That is
/// the failure with the longest fuse in this crate: it breaks a replay weeks
/// later rather than a test now.
///
/// Changes only that one thing, the way its neighbour above changes only the
/// seat. Advancing `pc` by one is the smallest difference a suspended
/// resolution can have and still be a different continuation.
#[test]
fn the_resolution_a_payment_window_holds_is_part_of_the_engine_snapshot() {
    let p1 = PlayerId::new(1);
    let mut before = ward_asks_a_seat_that_has_not_made_its_mana(31).0;
    let mut after = ward_asks_a_seat_that_has_not_made_its_mana(31).0;
    for engine in [&mut before, &mut after] {
        engine
            .apply(p1, PlayerAction::YesNo(true))
            .expect("saying they will pay opens the window");
    }
    assert_eq!(
        before.snapshot_hash(),
        after.snapshot_hash(),
        "the same fixture at the same step, which is what makes the line \
         below a statement about one field"
    );

    after
        .mana_window
        .as_mut()
        .expect("the window is open on both")
        .suspended
        .pc += 1;
    assert_ne!(
        before.snapshot_hash(),
        after.snapshot_hash(),
        "a window holding a resolution one operation further along is a \
         different engine, and hashing it the same is a divergence that \
         compares equal"
    );
}

/// CR 605.3a: a player asked for a mana payment may activate mana abilities
/// to make it, "even if it is in the middle of ... resolving an ability".
///
/// This is the rule the six taxing cards in this pool live on, and it was
/// missing: the question was skipped entirely against an empty pool, which
/// is the pool an opponent has right after casting the spell being taxed.
/// Against the old code this test reaches its first `panic!` in the fixture,
/// because the tax is never asked at all.
#[test]
fn a_taxed_seat_may_make_its_mana_after_the_question() {
    let p1 = PlayerId::new(1);
    let (mut engine, twins) = ward_asks_a_seat_that_has_not_made_its_mana(31);

    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("saying they will pay is an answer even with an empty pool");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "saying yes with nothing floating opens a payment window, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the window belongs to the seat being taxed");
    assert!(
        legal.lands.is_empty() && legal.castable.is_empty() && legal.suspendable.is_empty(),
        "a payment window offers mana and nothing else: {legal:?}",
    );
    assert!(
        legal.has_mana_source(),
        "the second Plains is still untapped and has to be offered",
    );

    let source = *legal
        .mana_abilities
        .first()
        .expect("the untapped Plains is the mana on offer");
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source })
        .expect("making mana is what the window is for");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("passing says the mana is made");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "closing the window spends the mana on the tax it was opened for",
    );

    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards { player, .. } => engine
                .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                .expect("Path offers its controller a basic-land search"),
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
    assert_ne!(
        engine.state().object(twins).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the tax was paid in the window, so the spell was not countered and \
         Path exiled the creature",
    );
}

/// The same window, paid with a land that asks which colour it makes: the
/// engine has one suspended-resolution slot and a CR 605.3a window needs two
/// (#167).
///
/// A mana ability that resolves outright never touches that slot, which is
/// why [`a_taxed_seat_may_make_its_mana_after_the_question`] is green on
/// either side of this fix and is the control for it — same seats, same
/// spell, same tax, and the only difference is that its spare land is a
/// Plains. Badlands prints `{T}: Add {B} or {R}`, so making its mana
/// suspends a resolution of its own to ask; that resolution went into the
/// slot the *ward* was waiting in, completed, and left the slot empty. The
/// next pass reached `close_mana_window`, whose `expect` is the only thing
/// that noticed — without it the ward's payment is silently lost, because
/// the mana is in the pool and the resolution that was going to spend it is
/// gone.
///
/// The card in the wild was not this one. ai-ec found it in a self-play
/// game at seed 1337 as **Storm of Saruman's ward {3}**, reached through
/// `AbilityRef::SYNTHETIC` rather than any printed ability, and the three
/// `Effect::PlayerMayPayOr` cards in those decks opened no window at all —
/// they were declined ten times out of eleven, because declining Rhystic
/// Study costs a card and declining a ward costs the spell. The mechanism
/// is the tax, not the card that charges it, and Twining Twins is the
/// cheapest way to reach it from a fixture that already exists.
#[test]
fn a_colour_asking_source_pays_the_tax_it_was_tapped_for() {
    let p1 = PlayerId::new(1);
    let (mut engine, twins) = ward_asks_a_seat_whose_spare_land_is(31, badlands());

    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("saying they will pay opens the window");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "saying yes with an empty pool opens a window, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the window belongs to the seat being taxed");
    let land = on_battlefield(&engine, p1, badlands()).expect("the dual is still untapped");
    // A nonbasic that prints its own `{T}: Add …` is an ordinary entry in
    // `abilities` and not the CR 305.6 shortcut, which is what
    // `narrow_to_mana_window` keeps rather than clears.
    let (source, ability_index) = *legal
        .abilities
        .iter()
        .find(|(s, _)| *s == land)
        .expect("the window offers the dual's own mana ability");
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("making mana is what the window is for");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "a dual asks which of its two colours, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "their land, their choice");
    assert_eq!(
        options.len(),
        2,
        "Badlands prints two colours and nothing has granted it a third: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Black),
        )
        .expect("black is one of the two on offer");

    engine.apply(p1, PlayerAction::PassPriority).expect(
        "passing says the mana is made — and used to panic here, \
                 because the colour question had taken the ward's slot",
    );

    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "closing the window spends the mana on the tax it was opened for",
    );

    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards { player, .. } => engine
                .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                .expect("Path offers its controller a basic-land search"),
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
    assert_ne!(
        engine.state().object(twins).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the tax was paid out of a land that asked a question first, so the \
         spell was not countered",
    );
}

/// The counter-test, and the one that keeps the window honest: a seat that
/// opens the window and then makes no mana pays nothing.
///
/// It is the same outcome as declining, which is the point — a window is an
/// opportunity, not a promise. It also pins the seam that would otherwise
/// fail only in debug: `resume_tax_choice` asserts that a payment it was
/// told about was payable, so the engine has to ask the pool again when the
/// window closes rather than pass the player's earlier "yes" straight
/// through.
#[test]
fn a_seat_that_makes_no_mana_in_the_window_pays_nothing() {
    let p1 = PlayerId::new(1);
    let (mut engine, twins) = ward_asks_a_seat_that_has_not_made_its_mana(37);

    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("saying they will pay is an answer");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the window opened",
    );
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("leaving the window empty-handed is allowed");

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
        "nothing was paid, so ward countered the spell (CR 702.21a)",
    );
    assert!(
        in_graveyard(&engine, p1, path_to_exile()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)",
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

/// Every `KeywordSet` a card *mentions*, and not only the ones on its faces.
///
/// The sweep above reads `CardDef::all_keywords`, which is the faces, and a
/// keyword is grantable from at least four other places: `Modifier::AddKeyword`
/// in a static, the `keywords` field of `Effect::PumpFilter` and of
/// `Effect::PumpTarget`, and `CopyMod::AddKeyword`. Mikaeus, the Unhallowed
/// granted undying through the first of them and the face sweep saw nothing,
/// while the intimidate on his own face failed loudly — the same card, the
/// same inert bit, one of them caught.
///
/// So this walks the card's whole **`Debug` rendering** rather than a list of
/// places to look. `KeywordSet` is a newtype over `u128` and renders as
/// `KeywordSet(4194304)` wherever it sits, however deep — inside a `MayDo`
/// inside a mode inside a chapter — so a nesting shape nobody thought of is
/// read for free, and a field added tomorrow needs no edit here. That is the
/// whole reason for a spelling nobody would choose for a getter: a positive
/// list of grant sites goes silent on the fifth one, and this cannot.
///
/// It counts *mentions*, which is deliberately wider than "grants". A card
/// that removes a keyword no rule reads, or filters for one, is saying
/// something the table cannot hear either.
#[test]
fn no_card_mentions_a_keyword_the_engine_ignores() {
    let enforced = enforced().bits();
    let (mut read, mut mention_one) = (0, 0);
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let (mut mentioned, mut sets) = (0u128, 0);
        let rendering = format!("{def:?}");
        for tail in rendering.split("KeywordSet(").skip(1) {
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            mentioned |= digits
                .parse::<u128>()
                .expect("a KeywordSet renders its bits");
            sets += 1;
        }
        read += usize::from(sets > 0);
        mention_one += usize::from(mentioned != 0);
        let unknown = mentioned & !enforced;
        assert_eq!(
            unknown, 0,
            "{} ({oracle_id}) names a keyword no engine rule reads (bits {:#x}); \
             a face, a static's `AddKeyword`, a pump's `keywords` or a `CopyMod` \
             — implement it and add it to ENFORCED, or take it off the card",
            def.faces[0].name, unknown,
        );
    }
    // The two bounds this reader owes, because a rendering that stopped
    // carrying the newtype's name would find nothing and pass over the whole
    // pool in silence. Every card has a face and every face has the field,
    // so the first is exact; the second is the claim that the sweep reaches
    // past the faces at all — 115 cards name a keyword against the 77 whose
    // own faces print one.
    assert_eq!(
        read,
        baylee_cards::generated::ALL.len(),
        "a card whose rendering carries no `KeywordSet` at all: every face has \
         the field, so the spelling this sweep matches on has changed"
    );
    let on_faces = baylee_cards::generated::ALL
        .iter()
        .filter(|(_, def)| def.all_keywords().bits() != 0)
        .count();
    assert!(
        mention_one > on_faces,
        "{mention_one} cards name a keyword and {on_faces} print one, so the \
         sweep is reading no further than `all_keywords` already did"
    );
}
