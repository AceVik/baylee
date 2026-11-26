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
//!
//! The last test is the same question asked of `Filter::This` inside a
//! *granted* triggered ability: "this creature" names the creature that has
//! the ability, not the object whose event set it off (#248).

use super::synthetic::{SyntheticLookup, creature, keep_mulligans, land, preset, walk_past};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, CardDef, Cost, Duration, Effect,
    Filter, Modifier, TargetSpec, Trigger,
};

const BOUNCER: u32 = 1_401;

/// `{0}: Return this land to its owner's hand.` — Oboro's sentence with the
/// price taken off, because the price is not what this is about.
static BOUNCE_SELF: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::FREE,
    effects: &[Effect::ReturnToHand {
        target: TargetSpec::ThisObject,
    }],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
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

/// The source version also protects an untargeted return with no
/// intervening-if clause to remove the old trigger (CR 400.7).
#[test]
fn a_graveyard_return_names_only_the_incarnation_that_triggered() {
    static RETURN: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
        Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::Upkeep,
            whose: baylee_cards_dsl::PlayerRel::You,
        },
        &[Effect::reanimate(TargetSpec::ThisObject)],
        zone = baylee_cards_dsl::TriggerZone::Graveyard,
    )];
    let me = PlayerId::new(0);
    for leave_and_return in [false, true] {
        let (mut engine, source) = seated(145);
        engine
            .state
            .move_object(
                source,
                ZoneLocation::Graveyard(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        engine
            .state
            .object_mut(source)
            .expect("source card")
            .own_abilities =
            Some(crate::object::AbilityList::from_static(RETURN, None, None).into_bundle());
        for step in 0..400 {
            if !engine.state.zones.list(ZoneLocation::Stack).is_empty() {
                break;
            }
            assert!(step < 399, "reached the source's upkeep trigger");
            let pending = engine.pending().clone();
            assert!(walk_past(&mut engine, &pending), "{pending:?}");
        }
        if leave_and_return {
            engine
                .state
                .move_object(
                    source,
                    ZoneLocation::Exile(me),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            engine
                .state
                .move_object(
                    source,
                    ZoneLocation::Graveyard(me),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        for step in 0..30 {
            if engine.state.zones.list(ZoneLocation::Stack).is_empty() {
                break;
            }
            assert!(step < 29, "return trigger resolved");
            let pending = engine.pending().clone();
            assert!(walk_past(&mut engine, &pending), "{pending:?}");
        }
        assert_eq!(
            engine.state.object(source).expect("card remains").zone,
            if leave_and_return {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
    }
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

const SHIELDBEARER: u32 = 1_402;

/// `{0}: Prevent the next 1 damage that would be dealt to this permanent.`
/// Rock Hydra's `{R}` sentence with the price taken off.
static SHIELD_SELF: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::FREE,
    effects: &[Effect::PreventNextDamage {
        target: TargetSpec::ThisObject,
        amount: baylee_cards_dsl::Amount::Fixed(1),
    }],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

/// The same rule on the damage side (`resolve::life::recipients`): a shield
/// on "this creature" is on the source. Reading `targets` there, as the
/// object specs do, found nothing and put the shield on nobody.
#[test]
fn a_shield_on_its_own_source_is_on_the_source() {
    let me = PlayerId::new(0);
    let lookup = SyntheticLookup::new(vec![land(SHIELDBEARER, "Shieldbearer", SHIELD_SELF)]);
    let mut engine = Engine::new(&preset(3, &[SHIELDBEARER]), lookup).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.pending(), Pending::Priority { player, .. } if *player == me) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    let source = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .first()
        .expect("the bench seated one permanent");
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source,
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
    let version = engine.state().object(source).expect("seated").version;
    assert!(
        engine
            .state()
            .shields
            .iter()
            .any(|s| s.protects == crate::prevention::Shielded::Object(source, version)),
        "no shield on the source: {:?}",
        engine.state().shields
    );
}

/// `landwalk::choose` reads its recipient through `subjects::source` for
/// `ThisObject`, and through `life::recipients` for the other specifications.
/// Its `lands` field precedes `target`, so a prefix-only spelling cannot
/// recognize it. Both pool censuses use this same complete-effect check.
fn landwalk_targets_read(text: &str, target: &str) -> usize {
    let needle = format!("target: {target}");
    text.split("ChooseLandwalk {")
        .skip(1)
        .filter(|tail| {
            tail.split_once(" }")
                .is_some_and(|(effect, _)| effect.contains(&needle))
        })
        .count()
}

/// `MustBlockIfAble` reads `attacker` through `subjects::source` for
/// `ThisObject` and `subjects::event` for `EventObject`; `blocker` precedes it.
fn must_block_attackers_read(text: &str, attacker: &str) -> usize {
    let needle = format!("attacker: {attacker},");
    text.split("MustBlockIfAble {")
        .skip(1)
        .filter(|tail| {
            tail.split_once(" }")
                .is_some_and(|(effect, _)| effect.contains(&needle))
        })
        .count()
}

/// `RedirectCombatDamageToYouThisTurn` reads `to` through `subjects::source` for
/// `DamageRedirectTo::ThisObject` (`resolve::life::ice_combat`); `by` precedes `to`.
fn redirect_combat_to_self_read(text: &str) -> usize {
    text.split("RedirectCombatDamageToYouThisTurn {")
        .skip(1)
        .filter(|tail| {
            tail.split_once(" }")
                .is_some_and(|(effect, _)| effect.contains("to: ThisObject"))
        })
        .count()
}

const THIS_OBJECT_READ: &[&str] = &[
    "ReturnToHand { target: ThisObject }",
    "ReturnToBattlefieldTapped { target: ThisObject }",
    "Destroy { target: ThisObject, no_regen: false }",
    "Destroy { target: ThisObject, no_regen: true }",
    "Exile { target: ThisObject }",
    "GraveyardToBattlefield { target: ThisObject, owner_control: false, counters: None }",
    "Regenerate { target: ThisObject }",
    "CantBeRegeneratedThisTurn { target: ThisObject }",
    // Rock Hydra's "{R}: Prevent the next 1 damage that would be dealt
    // to this creature": `resolve::life::recipients`. Left open, the
    // amount after the target is not this list's business.
    // Galleon's played regression proves the delayed source is retained.
    "AtEndOfCombat { about: ThisObject",
    // Krovikan Vampire watches its own exact incarnation for control loss
    // or departure; `control_observers::register` retains that version.
    "CreateDelayedObserver { watch: ThisObject",
    "PreventNextDamage { target: ThisObject",
    // Goblin Tinkerer's "that artifact deals damage equal to its mana value
    // to this creature": the recipient goes through
    // `resolve::life::recipients`, which reads the source.
    "EventObjectDealsDamage { amount: TargetCmc, target: ThisObject",
    "RedirectNextDamage { target: ThisObject",
    "SuppressUntap { target: ThisObject",
    "Explore { target: ThisObject }",
    "CreateTokenCopy { target: ThisObject,",
    "CopySpellOrAbility { target: ThisObject,",
    "Suspect { target: ThisObject }",
    "BecomeCopy { subject: First, subject_filter: None, original: ThisObject, original_is_second_target: true,",
    "DealDamage { amount: Fixed(1), target: ThisObject }",
    "DealDamage { amount: Fixed(2), target: ThisObject }",
    "DealDamage { amount: Fixed(3), target: ThisObject }",
    "ObjectMatches { what: ThisObject, filter:",
    "Play { source: Subject(ThisObject),",
    // Ojutai Exemplars's "Exile this creature, then return it...":
    // `resolve::reanimation::move_and_modify` reads `ThisObject` via
    // `resolve::zones::moving_objects` -> `subjects::moving_source`.
    "MoveAndModify { subject: Target(ThisObject),",
    "PutEachCounterKindFromSource { target: ThisObject }",
    "PutAllCountersFrom { donor: Event, target: ThisObject }",
    "PutAllCountersFrom { donor: Source, target: ThisObject }",
    "GainControlEx { target: ThisObject,",
    "MoveAttachment { attachment: ThisObject,",
    "AddTempModifier { target: ThisObject,",
    "subject: Spec(ThisObject),",
    "host: Spec(ThisObject)",
    "to: ThisObject,",
];

/// Which effects may be spelled with `ThisObject`, held against the pool.
///
/// The spec must reach an implicit-subject reader, such as
/// [`resolve::zones::spec_object`]. A card writing it into an effect that
/// only reads chosen targets compiles, claims
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
    let redirect_to_self = format!(
        "{:?}",
        Effect::RedirectTargetSourceDamageThisTurn {
            source: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
            only_to_you: true,
            to: baylee_cards_dsl::DamageRedirectTo::ThisObject,
        }
    );
    let sunflare_self = format!(
        "{:?}",
        Effect::DealDamage {
            amount: baylee_cards_dsl::Amount::CountOf {
                filter: &Filter::HasSubtype(baylee_core::generated::subtypes::creature::ELEMENTAL),
                zone: baylee_cards_dsl::ZoneSel::GraveyardOf(baylee_cards_dsl::PlayerRel::You),
            },
            target: TargetSpec::ThisObject,
        }
    );

    let mut unread = Vec::new();
    let mut seen = 0_usize;
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        let total = text.matches("ThisObject").count();
        if total == 0 {
            continue;
        }
        let read: usize = THIS_OBJECT_READ
            .iter()
            .map(|shape| text.matches(shape).count())
            .sum::<usize>()
            + text.matches(&redirect_to_self).count()
            + text.matches(&sunflare_self).count()
            + landwalk_targets_read(&text, "ThisObject")
            + must_block_attackers_read(&text, "ThisObject")
            + redirect_combat_to_self_read(&text);
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

const EVENT_OBJECT_READ: &[&str] = &[
    "spec: EventObject",
    "TapEventObject",
    "PumpEventObject",
    "RemoveEventObjectFromCombat",
    "SuppressUntap { target: EventObject",
    "CantAttackDuringNextControllerTurn { target: EventObject",
    "CantBeRegeneratedThisTurn { target: EventObject",
    "ReturnToHand { target: EventObject",
    "GraveyardToHand { target: EventObject }",
    "GraveyardToBattlefield { target: EventObject",
    "GraveyardToBattlefieldModified { target: EventObject",
    "GraveyardToTop { target: EventObject",
    "AttachSelf { target: EventObject",
    "PutOnBottomOfLibraryFromGraveyard { target: EventObject",
    "Destroy { target: EventObject",
    "Exile { target: EventObject",
    "IfEventObjectMatches",
    "SacrificeObject { target: EventObject",
    "AtEndOfCombat { about: EventObject",
    "CreateDelayedObserver { watch: ThisObject, when: YouLoseControl, about: EventObject",
    "EventObjectDealsDamage {",
    "EventObjectDealsDamageEach {",
    "EventObjectDealsDamageEqualToPower",
    "DamagePermanentAndController { target: EventObject,",
    "DealDamage { amount: Fixed(1), target: EventObject",
    "DealDamage { amount: Fixed(2), target: EventObject",
    "DealDamage { amount: Fixed(3), target: EventObject }",
    "DealDamage { amount: Fixed(4), target: EventObject }",
    "DealDamage { amount: SourcePower, target: EventObject }",
    "remember: EventObject,",
    "CreateTokenCopy { target: EventObject,",
    "CopySpellOrAbility { target: EventObject,",
    "BecomeCopy { subject: This, subject_filter: None, original: EventObject, original_is_second_target: false,",
    "Explore { target: EventObject }",
    "ExileWithPermission { target: EventObject,",
    "PhaseOut { target: Some(EventObject) }",
    "Blink { target: EventObject,",
    "ObjectMatches { what: EventObject, filter:",
    "source: Subject(EventObject),",
    "Suspect { target: EventObject }",
    "MoveAndModify { subject: Target(EventObject),",
    "GainControlEx { target: EventObject,",
    "AddTempModifier { target: EventObject,",
    "ExileIfDiesThisTurn { target: EventObject }",
    "MoveAttachment { attachment: EventObject,",
    "PutEachCounterKindFromSource { target: EventObject }",
    "PutAllCountersFrom { donor: Source, target: EventObject }",
    "PutAllCountersFrom { donor: Event, target: EventObject }",
    "host: Spec(EventObject)",
    "to: EventObject,",
    "DealDamage { amount: Fixed(7), target: EventObject }",
    "DoublePT { subject: EventObject,",
    "ExileLinked { target: EventObject,",
    "EventPower }, target: EventObject",
    "OwnerPutsOnTopOrBottom { target: EventObject",
];

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
    let mut unread = Vec::new();
    let mut seen = 0_usize;
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        let total = text.matches("EventObject").count();
        if total == 0 {
            continue;
        }
        let read: usize = EVENT_OBJECT_READ
            .iter()
            .map(|shape| text.matches(shape).count())
            .sum::<usize>()
            + landwalk_targets_read(&text, "EventObject")
            + must_block_attackers_read(&text, "EventObject");
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

const GRANTEE: u32 = 1_402;
const RUNNER: u32 = 1_403;

/// "This creature gets +1/+0 until end of turn."
static PUMP_THIS: &[Effect] = &[Effect::continuous(
    &Filter::This,
    Modifier::ModifyPT(1, 0),
    Duration::UntilEndOfTurn,
)];

/// `{0}: This creature gains "Whenever a creature you control attacks, this
/// creature gets +1/+0 until end of turn" until end of turn.` — Raging
/// Ravine's shape, with an event whose object is some *other* creature.
static GAIN_THE_TRIGGER: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::FREE,
    effects: &[Effect::continuous(
        &Filter::This,
        Modifier::GrantTriggered {
            trigger: Trigger::Attacks(&Filter::YOUR_CREATURE),
            effects: PUMP_THIS,
            target: None,
        },
        Duration::UntilEndOfTurn,
    )],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

/// The rule for a granted trigger: its "this" is the object that has it.
///
/// The source of a triggered ability is the object whose ability triggered
/// (CR 113.7), a granted ability is that object's own (CR 113.1a), and a
/// gained ability's word for itself names the object that gained it
/// (CR 201.5b). The Grantee stays home and the Runner attacks, so the event
/// object and the grantee are two different creatures and the pump can only
/// be right on one of them. The untargeted synthetic path used to put the
/// event object first among the trigger's targets, where `Filter::This`
/// reads it — and the Runner got +1/+0.
#[test]
fn a_granted_trigger_pumps_the_creature_that_has_it_and_not_the_one_that_set_it_off() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let grantee: &'static CardDef = Box::leak(Box::new(CardDef {
        abilities: GAIN_THE_TRIGGER,
        ..*creature(GRANTEE, "Grantee", 0, 1, &[])
    }));
    let lookup = SyntheticLookup::new(vec![grantee, creature(RUNNER, "Runner", 2, 2, &[])]);
    let mut engine = Engine::new(&preset(3, &[GRANTEE, RUNNER]), lookup).expect("the game starts");
    keep_mulligans(&mut engine);
    let object = |engine: &Engine<SyntheticLookup>, index: u32| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index.get() == index)
            })
            .expect("the bench seated it")
    };
    let (grantee, runner) = (object(&engine, GRANTEE), object(&engine, RUNNER));
    let power = |engine: &Engine<SyntheticLookup>, id: ObjectId| {
        engine
            .state()
            .object(id)
            .and_then(|o| o.characteristics().power)
    };
    let drain = |engine: &mut Engine<SyntheticLookup>| {
        while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
            let Pending::Priority { player, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending())
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
    };

    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: grantee,
                ability_index: 0,
            },
        )
        .expect("the ability activates");
    drain(&mut engine);

    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == me => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected the way to combat, got {other:?}"),
        }
    }
    engine
        .apply(
            me,
            PlayerAction::DeclareAttackers {
                attackers: vec![(runner, baylee_core::ids::Defender::Player(them))],
            },
        )
        .expect("the Runner attacks");
    assert!(
        !engine.state().zones.list(ZoneLocation::Stack).is_empty(),
        "the granted trigger is on the stack"
    );
    drain(&mut engine);

    assert_eq!(
        power(&engine, grantee),
        Some(1),
        "the Grantee got its +1/+0"
    );
    assert_eq!(power(&engine, runner), Some(2), "the Runner got nothing");
}

const TOKEN_CLONER: u32 = 1_404;
const ELEMENTAL_GRANTOR: u32 = 1_405;
const PLAIN_BEAR: u32 = 1_406;

static TOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::IsToken]);
static ANOTHER_YOUR_CREATURE: Filter = Filter::And(&[Filter::YOUR_CREATURE, Filter::Another]);
static IS_ELEMENTAL: Filter =
    Filter::HasSubtype(baylee_core::generated::subtypes::creature::ELEMENTAL);

static CLONER_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::activated!(
        Cost::FREE,
        &[Effect::CreateToken {
            token: &baylee_cards_dsl::WARRIOR_1_1_RED,
        }]
    ),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(Sacrifice(&TOKEN_CREATURE)),
        &[Effect::gain_life(1)]
    ),
    baylee_cards_dsl::triggered!(
        Trigger::Dies(&Filter::ANOTHER_CREATURE),
        &[Effect::CreateTokenCopy {
            target: TargetSpec::EventObject,
            controller: baylee_cards_dsl::PlayerRel::You,
            amount: baylee_cards_dsl::Amount::Fixed(1),
            mods: &[],
            tapped: false,
            attacking: false,
            haste_until_eot: false,
            cleanup: baylee_cards_dsl::CopyCleanup::None,
        }]
    ),
];

/// CR 707.2, CR 608.2h: when a token creature dies, SBA 704.5d removes it
/// from the graveyard before the death trigger resolves, and
/// `CreateTokenCopy { target: EventObject }` still copies its departed LKI.
#[test]
fn token_copy_of_event_object_copies_a_token_that_ceased_to_exist() {
    let me = PlayerId::new(0);
    let cloner_def: &'static CardDef = Box::leak(Box::new(CardDef {
        abilities: CLONER_ABILITIES,
        ..*creature(TOKEN_CLONER, "Token Cloner", 2, 2, &[])
    }));
    let lookup = SyntheticLookup::new(vec![cloner_def]);
    let mut engine = Engine::new(&preset(4, &[TOKEN_CLONER]), lookup).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    let cloner = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .first()
        .expect("cloner seated");
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: cloner,
                ability_index: 0,
            },
        )
        .expect("creates token");
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "{pending:?}");
    }
    let first_token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| id != cloner)
        .expect("warrior token on battlefield");
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: cloner,
                ability_index: 1,
            },
        )
        .expect("sacrifices token");
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![first_token],
            },
        )
        .expect("pays token sacrifice cost");
    assert!(
        engine.state().object(first_token).is_none(),
        "SBA 704.5d removed the sacrificed token before the trigger resolved"
    );
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "{pending:?}");
    }
    let copied_token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| id != cloner)
        .expect("CreateTokenCopy { target: EventObject } copied the departed token");
    assert_ne!(copied_token, first_token);
    assert_eq!(
        engine
            .state()
            .object(copied_token)
            .and_then(|o| o.characteristics().power),
        Some(1)
    );
}

static ELEMENTAL_WATCHER_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::AddSubtype(baylee_core::generated::subtypes::creature::ELEMENTAL)
    ),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(Sacrifice(&ANOTHER_YOUR_CREATURE)),
        &[]
    ),
    baylee_cards_dsl::triggered!(
        Trigger::Dies(&Filter::ANOTHER_CREATURE),
        &[Effect::IfCondition {
            condition: baylee_cards_dsl::Condition::ObjectMatches {
                what: &TargetSpec::EventObject,
                filter: &IS_ELEMENTAL,
            },
            then: &[Effect::gain_life(4)],
            otherwise: &[],
        }]
    ),
];

/// CR 608.2h: `Condition::ObjectMatches { what: EventObject }` on a death
/// trigger reads the creature's battlefield LKI (including granted subtypes)
/// rather than only its bare graveyard card.
#[test]
fn object_matches_event_object_reads_battlefield_lki_after_death() {
    let me = PlayerId::new(0);
    let watcher_def: &'static CardDef = Box::leak(Box::new(CardDef {
        abilities: ELEMENTAL_WATCHER_ABILITIES,
        ..*creature(ELEMENTAL_GRANTOR, "Elemental Watcher", 2, 3, &[])
    }));
    let lookup = SyntheticLookup::new(vec![
        watcher_def,
        creature(PLAIN_BEAR, "Plain Bear", 2, 2, &[]),
    ]);
    let mut engine =
        Engine::new(&preset(5, &[ELEMENTAL_GRANTOR, PLAIN_BEAR]), lookup).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    let watcher = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index.get() == ELEMENTAL_GRANTOR)
        })
        .expect("watcher seated");
    let bear = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| id != watcher)
        .expect("bear seated");
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: watcher,
                ability_index: 1,
            },
        )
        .expect("sacrifices plain bear");
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .expect("pays bear sacrifice cost");
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "{pending:?}");
    }
    assert_eq!(
        engine.state().players[usize::from(me.get())].life,
        24,
        "Plain Bear had Elemental on the battlefield when it died (CR 608.2h)"
    );
}
