//! `TargetSpec::ThisObject`: an effect that names its own source.
//!
//! Every other target spec is chosen at CR 601.2c and read back out of
//! `Resolution::targets`. `ThisObject` is the one that is **not** chosen — it
//! names the source, so no question is asked and hexproof has nothing to
//! answer. That is the difference between "return Oboro to its owner's hand"
//! and "return target land", and it is why the resolver has to read the spec
//! off the effect rather than off the answer.
//!
//! Three cards in the pool spell it and all three did nothing at all before
//! `resolve::zones::spec_object` existed (#147): Oboro, Palace in the Clouds
//! and Ghost Town through `ReturnToHand`, The Tabernacle at Pendrell Vale
//! through `Destroy`. Their own tests are in `card_tests::lands`. What is
//! here is the rule those three share, asked without a card — a printed card
//! carries four other sentences that would have to keep working for a card
//! test to mean this — plus the census that says which effects may be spelled
//! this way at all.
//!
//! Each of the three was checked against an injected defect rather than
//! trusted for passing. Reverting `spec_object` to `res.targets.first()`
//! turns the rule test and all three card tests red. Declaring
//! `target: Some(TargetSpec::ThisObject)` on the bench ability — the fix
//! that also works and is wrong — turns the second one red with
//! `ChooseTargets { options: [ObjectId(0#0)] }`, an ability asking a player
//! to pick the only thing it could have meant. And dropping the `Destroy`
//! spelling from the census's list turns the third red naming The Tabernacle
//! at Pendrell Vale, whose `ThisObject` is two levels down — inside a
//! `PlayerMayPayOr` inside a `Modifier::GrantTriggered` — which is the case
//! a walk over an ability's own effect list cannot see.

use super::synthetic::{SyntheticLookup, keep_mulligans, land, preset, walk_past};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Cost, Effect, TargetSpec,
};

const BOUNCER: u32 = 1_401;

/// `{0}: Return this land to its owner's hand.` — Oboro's sentence with the
/// price taken off, because the price is not what this is about.
static BOUNCE_SELF: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::FREE,
    effects: &[Effect::ReturnToHand {
        target: TargetSpec::ThisObject,
    }],
    target: None,
    second_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
}];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![land(BOUNCER, "Bouncer", BOUNCE_SELF)])
}

/// A game with a Bouncer on seat 0's battlefield, at that seat's priority.
fn seated(seed: u64) -> (Engine<SyntheticLookup>, ObjectId) {
    let me = PlayerId::new(0);
    let mut engine = Engine::new(&preset(seed, &[BOUNCER]), lookup()).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            let land = *engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .first()
                .expect("the bench seated one permanent");
            return (engine, land);
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    panic!("never reached seat 0's main phase");
}

/// The rule: an effect naming `ThisObject` moves the **source**, and it moves
/// it without anything being chosen.
#[test]
fn an_effect_that_names_its_own_source_moves_the_source() {
    let me = PlayerId::new(0);
    let (mut engine, land) = seated(1);
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the ability activates");
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "the source is still on the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me))
            .contains(&land),
        "the source did not reach its owner's hand"
    );
}

/// And the half that refuses the tempting wrong fix.
///
/// Declaring `targets = Some(TargetReq::one(TargetSpec::ThisObject))` also
/// makes the ability work — the choose-targets flow then fills
/// `Resolution::targets` with the one option there is — and it is wrong: a
/// card whose printed sentence does not say "target" does not target, so
/// hexproof, shroud and protection would start answering an ability that
/// never pointed at anything (CR 115.6). Nothing else in the suite reads
/// that as a bug, because the permanent moves either way.
#[test]
fn naming_your_own_source_asks_no_question() {
    let me = PlayerId::new(0);
    let (mut engine, land) = seated(2);
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the ability activates");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "activating it asked something: {:?}",
        engine.pending()
    );
}

/// Which effects may be spelled with `ThisObject`, held against the pool.
///
/// The spec is read by [`resolve::zones::spec_object`] and by nothing else,
/// so a card writing it into any other effect compiles, claims
/// `Coverage::Implemented`, resolves — and does nothing. That is the shape
/// `xtask validate` and the pool lints cannot see, because such a card says
/// exactly the right thing; it took playing the ability to find out that
/// saying it did nothing.
///
/// Read off `Debug` rather than a match table on purpose. A table listing
/// which variants carry a `TargetSpec` would have to be kept in step with
/// the enum, and a reader that has gone blind on a variant reports zero and
/// passes. `Debug` prints every field of every nested effect, which is what
/// this has to reach: the Tabernacle's `Destroy` is inside a
/// `PlayerMayPayOr` inside a `Modifier::GrantTriggered`, so a walk over an
/// ability's own effect list finds two of the three and reports a census it
/// never took. The floor below is the other half of that guard.
#[test]
fn every_this_object_in_the_pool_is_one_the_resolver_reads() {
    // Exactly the spellings `spec_object` is reached from. Written as the
    // `Debug` of the effect, so adding an arm there and forgetting this list
    // fails loudly on the next card rather than quietly on the next player.
    //
    // `Destroy` is spelled twice because its `no_regen` rider is part of the
    // `Debug` and a prefix match would stop being a reading of the whole
    // effect — which is the property this list is here for.
    const READ: &[&str] = &[
        "ReturnToHand { target: ThisObject }",
        "Destroy { target: ThisObject, no_regen: false }",
        "Destroy { target: ThisObject, no_regen: true }",
        "GraveyardToBattlefield { target: ThisObject }",
        "Regenerate { target: ThisObject }",
    ];

    let mut unread = Vec::new();
    let mut seen = 0_usize;
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        let total = text.matches("ThisObject").count();
        if total == 0 {
            continue;
        }
        let read: usize = READ.iter().map(|shape| text.matches(shape).count()).sum();
        seen += total;
        if read < total {
            unread.push(format!(
                "{}: {total} `ThisObject`, {read} of them in an effect the resolver reads",
                def.name()
            ));
        }
    }
    assert!(
        seen >= 3,
        "only {seen} `ThisObject` found in the whole pool — three cards spell \
         it, so this reader has gone blind and an empty sweep proves nothing"
    );
    assert!(
        unread.is_empty(),
        "{} card(s) name their own source in an effect that throws the spec \
         away and resolves to nothing. Either read the spec through \
         `resolve::zones::spec_object` and add the spelling above, or write \
         the effect that acts on the source (`SacrificeSelf`, `ExileSource`, \
         `PutSourceOnTopOfLibrary`).\n{}",
        unread.len(),
        unread.join("\n")
    );
}

/// The same census for `EventObject`, which is the **second** implicit spec
/// and was found the same way.
///
/// A trigger fills [`Resolution::targets`] from the event object when its
/// own *target requirement* says `EventObject` — five cards in the pool are
/// written that way and all of them work. Writing it on an **effect's**
/// field instead is the other half, and it is read by
/// [`resolve::zones::spec_object`] and by nothing else: Journey to Eternity
/// spelled `GraveyardToBattlefield { target: EventObject }`, resolved, and
/// left the creature in the graveyard while returning its own back face
/// transformed — `Coverage::Implemented`, half of what it prints, and
/// invisible to `xtask validate` because the card says the right thing.
///
/// Read off `Debug` for the reason the sibling gives: a match table over the
/// variants carrying a `TargetSpec` goes blind on a new one and reports
/// zero. Both spellings are counted here, because the point is that every
/// `EventObject` in the pool is in *one* of the two places that reads it.
#[test]
fn every_event_object_in_the_pool_is_one_the_engine_reads() {
    // A declared target requirement (filled by `progress::stack_triggers`)
    // and the effect fields `spec_object` reads. Written as `Debug`, so an
    // arm added to the resolver and forgotten here fails on the next card
    // rather than on the next player.
    //
    // The struct spelling is left **open** — no closing brace — because the
    // fields after `target` are not this test's business and a variant that
    // grows one goes silently unread otherwise. It did: undying and persist
    // put `owner_control` and `counters` on `GraveyardToBattlefield`, and
    // the closed spelling stopped matching Journey to Eternity the same day,
    // reporting a card the resolver reads perfectly well.
    const READ: &[&str] = &[
        "spec: EventObject",
        "GraveyardToBattlefield { target: EventObject",
    ];

    let mut unread = Vec::new();
    let mut seen = 0_usize;
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        let total = text.matches("EventObject").count();
        if total == 0 {
            continue;
        }
        let read: usize = READ.iter().map(|shape| text.matches(shape).count()).sum();
        seen += total;
        if read < total {
            unread.push(format!(
                "{}: {total} `EventObject`, {read} of them somewhere the engine reads",
                def.name()
            ));
        }
    }
    assert!(
        seen >= 6,
        "only {seen} `EventObject` found in the whole pool — six cards spell \
         it, so this reader has gone blind and an empty sweep proves nothing"
    );
    assert!(
        unread.is_empty(),
        "{} card(s) name the triggering object where nothing reads it, so the \
         effect resolves and does nothing:\n  {}",
        unread.len(),
        unread.join("\n  ")
    );
}
