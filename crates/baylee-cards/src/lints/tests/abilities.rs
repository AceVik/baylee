//! Continuous relations, mana abilities, divisions, modes, choices,
//! reflexives, grants and tokens.

use super::*;

/// A player relation a continuous effect carries is one the game state
/// can answer on its own.
///
/// `DrawLimitPerTurn` and `CantLoseLife` say whose draws or whose life,
/// relative to the effect's controller, and the engine reads them with
/// no resolution in hand (`GameState::draw_limit`,
/// `GameState::cant_lose_life`). `Chosen` and `ControllerOfTarget` are
/// answered only by a resolution, so there they name nobody: the effect
/// would be registered and limit no one. The match on the relation is
/// exhaustive so that a new relation has to be sorted into one side.
#[test]
fn a_continuous_player_relation_is_one_the_state_can_answer() {
    use crate::dsl::effect::PlayerRel;
    let mut wrong = Vec::new();
    let mut seen = 0_usize;
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            let printed = match ability {
                AbilityDef::Static(sa) => vec![sa.modifier],
                _ => Vec::new(),
            };
            let granted = branches(ability)
                .into_iter()
                .flat_map(|b| b.effects.iter().flat_map(declared_layers))
                .map(|(_, modifier)| modifier);
            for modifier in printed.into_iter().chain(granted) {
                let (Modifier::DrawLimitPerTurn { who, .. }
                | Modifier::CantLoseLife { who }
                | Modifier::NoLossForZeroLife { who }
                | Modifier::LifeGainDrawsInstead { who }
                | Modifier::CantBeAttackedExceptBy { who, .. }
                | Modifier::SkipUntapStep { who }
                | Modifier::UntapAtMost { who, .. }) = modifier
                else {
                    continue;
                };
                seen += 1;
                let answerable = match who {
                    PlayerRel::You
                    | PlayerRel::Opponent
                    | PlayerRel::EachOpponent
                    | PlayerRel::EachPlayer
                    | PlayerRel::ActivePlayer => true,
                    PlayerRel::Chosen
                    | PlayerRel::ControllerOfTarget
                    | PlayerRel::ControllerOfEvent
                    | PlayerRel::DamagedPlayer
                    | PlayerRel::OwnerOfSource
                    | PlayerRel::ControllerOfAttached => false,
                };
                if !answerable {
                    wrong.push(format!("{}: {modifier:?}", def.name()));
                }
            }
        }
    }
    // Spirit of the Labyrinth and Everybody Lives!, 2026-09-24: one
    // through each door, a printed static and a spell's grant.
    assert!(
        seen >= 2,
        "only {seen} player relations found on continuous effects — the \
         walker has gone blind"
    );
    assert!(
        wrong.is_empty(),
        "{} continuous effect(s) name a player only a resolution can \
         answer, so they would apply to nobody:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// A mana ability is an activated ability that **could** add mana and
/// does not target (CR 605.1a).
///
/// The flag is not decoration: an ability marked `mana_ability` does not
/// use the stack and cannot be responded to, and one that is not marked
/// does. Both directions of the mistake are silent — a mana ability that
/// forgot the flag merely feels slow, and a non-mana ability that claims
/// it skips a window the rules guarantee — and nothing else in the suite
/// reads either as a bug.
///
/// Damage/life riders do not exclude a mana ability. Library movement
/// does: Chromatic Sphere now belongs on the stack under CR 605.1a.
#[test]
fn mana_abilities_meet_all_current_activation_criteria() {
    let mut wrong = Vec::new();
    let mut seen = 0_usize;
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            if matches!(
                ability,
                AbilityDef::Activated { .. } | AbilityDef::ActivatedConditional { .. }
            ) {
                seen += 1;
            }
            if let Some(fault) = mana_ability_fault(ability) {
                wrong.push(format!("{} {fault}", def.name()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} ability/abilities disagree with CR 605.1:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // The floor: a sweep that inspected nothing is indistinguishable
    // from one that found nothing, and the pool has four hundred lands
    // in it.
    assert!(
        seen > 300,
        "only {seen} activated abilities were looked at; the sweep is not reaching the pool"
    );
}

/// Every damage division in the pool sits where the engine asks it: a
/// top-level op of a targeting triggered ability, with no more targets
/// than damage (CR 601.2d). Fury is why the sweep exists; a walk that
/// found no division has stopped reaching it.
#[test]
fn every_divided_damage_is_a_trigger_that_can_divide() {
    let (mut wrong, mut found) = (Vec::new(), 0_usize);
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            found += resolving_lists(ability)
                .iter()
                .map(|(effects, _, _)| divisions_in(effects))
                .sum::<usize>();
            if let Some(fault) = division_fault(ability) {
                wrong.push(format!("{} {fault}", def.name()));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(found >= 1, "no division found, and Fury prints one");
}

/// The sweep above passes, and this shows passing means something: each
/// shape the rule refuses is reported, and Fury's is not.
#[test]
fn a_division_fault_fires_on_each_bad_shape() {
    use crate::dsl::ability::Trigger;
    const DIVIDE: &[Effect] = &[Effect::DealDamageDivided { amount: 4 }];
    const IN_A_MAY: &[Effect] = &[Effect::MayDo { effects: DIVIDE }];
    const TWICE: &[Effect] = &[
        Effect::DealDamageDivided { amount: 2 },
        Effect::DealDamageDivided { amount: 2 },
    ];
    let targets = |max| {
        Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER),
            max,
        ))
    };
    let trigger = |effects, targets| AbilityDef::Triggered {
        trigger: Trigger::ETB,
        effects,
        targets,
        second_targets: None,
        zone: baylee_cards_dsl::TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    };
    assert_eq!(division_fault(&trigger(DIVIDE, targets(4))), None);
    for (shape, ability) in [
        ("no targets", trigger(DIVIDE, None)),
        ("more targets than damage", trigger(DIVIDE, targets(5))),
        ("inside a may", trigger(IN_A_MAY, targets(4))),
        ("twice", trigger(TWICE, targets(2))),
        (
            "on a spell",
            AbilityDef::Spell {
                effects: DIVIDE,
                targets: targets(4),
                second_targets: None,
                condition: None,
            },
        ),
    ] {
        assert!(
            division_fault(&ability).is_some(),
            "a division {shape} was not reported"
        );
    }
}

/// Every modal spell in the pool can be cast the way it prints: one
/// mode with at most its own replacement cost, or several (CR 700.2d)
/// with costs added to the card's and at most two instances of the
/// word "target" between them. The three cards that choose several
/// (Farewell, Final Showdown, Three Steps Ahead) are why the sweep
/// exists; a walk that found fewer has stopped reaching them.
#[test]
fn every_modal_spell_chooses_its_modes_the_way_the_engine_casts_them() {
    let (mut wrong, mut several) = (Vec::new(), 0_usize);
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            if let AbilityDef::ModalSpell { choose, .. } = ability
                && !choose.is_one()
            {
                several += 1;
            }
            if let Some(fault) = modes_fault(ability) {
                wrong.push(format!("{} {fault}", def.name()));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(several >= 3, "only {several} spells choose several modes");
}

/// The sweep above passes, and this shows passing means something: each
/// shape the rule refuses is reported, and Three Steps Ahead's is not.
#[test]
fn a_modes_fault_fires_on_each_bad_shape() {
    use crate::dsl::ModeCount;
    use baylee_core::mana::ManaCost;
    const CREATURE: Option<TargetReq> = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)));
    const PLAYER: Option<TargetReq> = Some(TargetReq::one(TargetSpec::AnyPlayer));
    const DRAW: &[Effect] = &[Effect::draw(1)];
    let mode = |targets, cost_override, additional_cost| SpellMode {
        effects: DRAW,
        targets,
        second_targets: None,
        cost_override,
        additional_cost,
    };
    let one = Some(const { ManaCost::parse("{1}") });
    let spell = |choose, modes: &'static [SpellMode]| AbilityDef::ModalSpell { modes, choose };
    let leak = |modes: Vec<SpellMode>| -> &'static [SpellMode] { Vec::leak(modes) };
    assert_eq!(
        modes_fault(&spell(
            ModeCount::ONE_OR_MORE,
            leak(vec![
                mode(PLAYER, None, one),
                mode(CREATURE, None, one),
                mode(None, None, one),
            ])
        )),
        None
    );
    for (shape, ability) in [
        (
            "choosing none",
            spell(
                ModeCount { min: 0, max: 2 },
                leak(vec![mode(None, None, None)]),
            ),
        ),
        (
            "choosing more than it prints",
            spell(ModeCount::TWO, leak(vec![mode(None, None, None)])),
        ),
        (
            "a mode cost on a choose-one",
            spell(ModeCount::ONE, leak(vec![mode(None, None, one)])),
        ),
        (
            "an overload among several",
            spell(
                ModeCount::ONE_OR_MORE,
                leak(vec![mode(None, one, None), mode(None, None, None)]),
            ),
        ),
        (
            "three targeting modes",
            spell(
                ModeCount::ONE_OR_MORE,
                leak(vec![mode(CREATURE, None, None); 3]),
            ),
        ),
        (
            "a player in the second instance",
            spell(
                ModeCount::ONE_OR_MORE,
                leak(vec![mode(CREATURE, None, None), mode(PLAYER, None, None)]),
            ),
        ),
        (
            "nine modes",
            spell(
                ModeCount::ONE_OR_MORE,
                leak(vec![mode(None, None, None); 9]),
            ),
        ),
        (
            "a mode that says \"target\" twice among several",
            spell(
                ModeCount::ONE_OR_MORE,
                leak(vec![
                    SpellMode {
                        second_targets: CREATURE,
                        ..mode(CREATURE, None, None)
                    },
                    mode(None, None, None),
                ]),
            ),
        ),
    ] {
        assert!(
            modes_fault(&ability).is_some(),
            "a modal spell with {shape} was not reported"
        );
    }
}

/// Every "choose a permanent you control, then …" in the pool does
/// something to it without stopping to ask, which is what keeps the
/// choice in hand while it happens. Final Showdown is why the sweep
/// exists.
#[test]
fn every_chosen_permanent_keeps_its_choice() {
    let (mut wrong, mut found) = (Vec::new(), 0_usize);
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            for (effects, _, _) in resolving_lists(ability) {
                let mut seen = 0_usize;
                Effect::walk(effects, &mut seen, &mut |effect| {
                    if matches!(effect, Effect::ChooseYoursThen { .. }) {
                        found += 1;
                    }
                });
                if let Some(fault) = chosen_then_fault(effects) {
                    wrong.push(format!("{} {fault}", def.name()));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(
        found >= 1,
        "no chosen permanent found, and Final Showdown prints one"
    );
}

/// And the rule refuses what it says it refuses.
#[test]
fn a_chosen_then_fault_fires_on_each_bad_shape() {
    const GAINS: &[Effect] = &[Effect::CreateContinuousEffect {
        layer: Layer::Ability,
        filter: &Filter::This,
        modifier: Modifier::LoseAllAbilities,
        duration: Duration::UntilEndOfTurn,
    }];
    const FINE: &[Effect] = &[Effect::ChooseYoursThen {
        filter: &Filter::CREATURE,
        then: GAINS,
    }];
    const EMPTY: &[Effect] = &[Effect::ChooseYoursThen {
        filter: &Filter::CREATURE,
        then: &[],
    }];
    const ASKS: &[Effect] = &[Effect::ChooseYoursThen {
        filter: &Filter::CREATURE,
        then: &[Effect::MayDo { effects: GAINS }],
    }];
    const NESTED: &[Effect] = &[Effect::IfKicked {
        then: ASKS,
        otherwise: &[],
    }];
    assert_eq!(chosen_then_fault(FINE), None);
    for (shape, effects) in [("empty", EMPTY), ("asking", ASKS), ("nested", NESTED)] {
        assert!(
            chosen_then_fault(effects).is_some(),
            "a chosen permanent's {shape} list was not reported"
        );
    }
}

/// Every reflexive trigger in the pool sits where the engine can read its
/// action back (CR 603.12). That place is the last op of a list that
/// resolves off the stack, directly after the sacrifice it names, with
/// nothing before it that could move the source.
///
/// Every door is read, for the reason
/// `no_granted_ability_breaks_the_rule_a_printed_one_is_held_to` gives:
/// printed, granted by a static, granted by a resolving effect, granted
/// by a copy clause, and printed on a token. The floor counts the lists
/// read through each door rather than the reflexives found, because a
/// count of found reflexives clears itself as soon as the pool holds
/// enough of one shape. The reflexive floor sits beside it. The six cards
/// that print "when you do" are why the sweep exists, and a walk that
/// found none of them has stopped reaching them.
#[test]
fn every_reflexive_sits_where_it_can_trigger() {
    let mut wrong = Vec::new();
    let mut doors = [0_usize; 4];
    let (mut token_lists, mut reflexives) = (0_usize, 0_usize);
    let mut read = |name: &str, ability: &AbilityDef, counts: &mut dyn FnMut(Door)| {
        for (effects, on_the_stack, door) in resolving_lists(ability) {
            counts(door);
            reflexives += reflexives_in(effects);
            if let Some(fault) = reflexive_fault_in(effects, on_the_stack) {
                wrong.push(format!("{name} {fault}"));
            }
        }
    };
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            read(def.name(), ability, &mut |door| doors[door as usize] += 1);
        }
    }
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            read(token.name, ability, &mut |_| token_lists += 1);
        }
    }
    assert!(
        wrong.is_empty(),
        "{} reflexive trigger(s) the engine would read wrongly:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(
        reflexives >= 6,
        "only {reflexives} reflexive trigger(s) found, and six cards print one"
    );
    assert!(
        doors.iter().all(|n| *n >= 1) && token_lists >= 1,
        "lists read per door: printed {}, static grant {}, effect grant {}, \
         copy grant {}, token {token_lists}. A nought is a door the sweep \
         stopped reaching",
        doors[Door::Printed as usize],
        doors[Door::Static as usize],
        doors[Door::Effect as usize],
        doors[Door::Copy as usize],
    );
}

/// The sweep above passes, and this test shows that passing means
/// something. Every misplacement the rule forbids is written once and
/// has to be reported. Both correct shapes, the fetch land's and Eden's,
/// have to pass.
#[test]
fn a_reflexive_fault_fires_on_each_bad_shape() {
    use crate::dsl::effect::{Amount, PlayerRel};
    const BODY: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    const REFLEX: Effect = Effect::Reflexive {
        when: ReflexiveEvent::SacrificedThis,
        effects: BODY,
        target: None,
    };
    const SACRIFICE: &[Effect] = &[Effect::SacrificeSelf];
    const DRAW: Effect = Effect::DrawCards {
        amount: Amount::Fixed(1),
    };
    const MILL: Effect = Effect::Mill {
        amount: Amount::Fixed(2),
        target: PlayerRel::You,
    };
    static FETCH: [Effect; 2] = [Effect::SacrificeSelf, REFLEX];
    static EDEN: [Effect; 3] = [MILL, Effect::MayDo { effects: SACRIFICE }, REFLEX];
    static NOT_LAST: [Effect; 3] = [Effect::SacrificeSelf, REFLEX, DRAW];
    static BETWEEN: [Effect; 3] = [Effect::SacrificeSelf, DRAW, REFLEX];
    static IN_A_MAY: [Effect; 1] = [Effect::MayDo { effects: &FETCH }];
    static IN_A_REFLEX: [Effect; 2] = [
        Effect::SacrificeSelf,
        Effect::Reflexive {
            when: ReflexiveEvent::SacrificedThis,
            effects: &FETCH,
            target: None,
        },
    ];
    static PREFIX: [Effect; 3] = [DRAW, Effect::SacrificeSelf, REFLEX];
    const EXILED: Effect = Effect::Reflexive {
        when: ReflexiveEvent::ExiledThis,
        effects: BODY,
        target: None,
    };
    static BALROG: [Effect; 2] = [
        Effect::MayDo {
            effects: &[Effect::ExileSource],
        },
        EXILED,
    ];
    static EXILED_AFTER_A_SACRIFICE: [Effect; 2] = [Effect::SacrificeSelf, EXILED];
    static ALONE: [Effect; 1] = [REFLEX];
    static GRANT: [Effect; 1] = [Effect::CreateContinuousEffect {
        layer: Layer::Ability,
        filter: &Filter::This,
        modifier: Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: &FETCH,
            mana_ability: true,
        },
        duration: Duration::UntilEndOfTurn,
    }];

    assert_eq!(reflexive_fault_in(&FETCH, true), None);
    assert_eq!(reflexive_fault_in(&EDEN, true), None);
    assert_eq!(reflexive_fault_in(&BALROG, true), None);

    for (shape, list) in [
        ("not last", &NOT_LAST[..]),
        ("an op between the action and it", &BETWEEN[..]),
        ("inside a may", &IN_A_MAY[..]),
        ("inside another reflexive", &IN_A_REFLEX[..]),
        ("an unlisted op before the action", &PREFIX[..]),
        ("no action at all", &ALONE[..]),
        (
            "waiting for an exile after a sacrifice",
            &EXILED_AFTER_A_SACRIFICE[..],
        ),
    ] {
        assert!(
            reflexive_fault_in(list, true).is_some(),
            "a reflexive trigger {shape} was not reported"
        );
    }
    assert!(
        reflexive_fault_in(&FETCH, false).is_some(),
        "a reflexive trigger in a mana ability was not reported"
    );

    // Through a door: a granted mana ability, written inside the effect
    // list of an ordinary trigger.
    let granting = AbilityDef::Triggered {
        trigger: crate::dsl::ability::Trigger::ETB,
        effects: &GRANT,
        targets: None,
        second_targets: None,
        zone: baylee_cards_dsl::TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    };
    let faults: Vec<_> = resolving_lists(&granting)
        .into_iter()
        .filter_map(|(effects, on_the_stack, _)| reflexive_fault_in(effects, on_the_stack))
        .collect();
    assert_eq!(
        faults.len(),
        1,
        "the granted mana ability is read: {faults:?}"
    );
}

#[test]
fn copiable_exception_walk_reports_invalid_mana_abilities() {
    static BAD: AbilityDef = AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::FREE,
        effects: &[
            Effect::SacrificeSelf,
            Effect::Reflexive {
                when: ReflexiveEvent::SacrificedThis,
                effects: &[Effect::GainLife {
                    amount: crate::dsl::effect::Amount::Fixed(1),
                }],
                target: None,
            },
        ],
        targets: None,
        second_targets: None,
        mana_ability: true,
        zone: crate::dsl::ability::ActivationZone::Battlefield,
        timing: crate::dsl::ability::ActivationTiming::InstantSpeed,
        limit: crate::dsl::ability::ActivationLimit::Unlimited,
        cost_reduction: None,
    };
    static MODS: [crate::dsl::ability::CopyMod; 1] =
        [crate::dsl::ability::CopyMod::GrantAbility(&BAD)];
    let copy = AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &MODS,
    };
    let granted = copiable_grants(&copy);
    assert_eq!(granted.len(), 1);
    assert!(mana_ability_fault(granted[0]).is_some());
    assert!(
        resolving_lists(&copy)
            .into_iter()
            .any(|(effects, stack, _)| reflexive_fault_in(effects, stack).is_some())
    );
    assert!(
        resolving_lists(&copy)
            .iter()
            .any(|(_, stack, door)| !stack && matches!(door, Door::Copy))
    );
}

/// A reflexive trigger's target belongs to the reflexive trigger.
///
/// [`branches`] cuts a trailing reflexive off the list it ends and makes
/// its body a branch with its own target. Without that cut the body sat
/// in a branch that targets nothing, so "destroy all creatures" behind
/// "target creature" was invisible to [`target_reuse`], which is Karn's
/// fault in another place.
#[test]
fn a_reflexive_target_is_read_as_its_own_branch() {
    const SWEEP_ALL: &[Effect] = &[Effect::DestroyAll {
        filter: &Filter::CREATURE,
        no_regen: false,
    }];
    static LIST: [Effect; 2] = [
        Effect::SacrificeSelf,
        Effect::Reflexive {
            when: ReflexiveEvent::SacrificedThis,
            effects: SWEEP_ALL,
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
        },
    ];
    let ability = AbilityDef::Triggered {
        trigger: crate::dsl::ability::Trigger::ETB,
        effects: &LIST,
        targets: None,
        second_targets: None,
        zone: baylee_cards_dsl::TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    };
    assert_eq!(target_reuse(&ability), Some(&Filter::CREATURE));
}

/// CR 605.1 over the activated abilities no card *prints*: the ones a
/// permanent is **granted**.
///
/// [`mana_ability_fault`] opens on a match over `Activated |
/// ActivatedConditional` and returns [`None`] for everything else, so a
/// [`Modifier::GrantActivated`] — which carries a `mana_ability` flag of
/// its own — is outside it, and outside both sweeps that call it: those
/// walk `AbilityDef`s, and a grant is not one.
///
/// The flag means the same thing there. A granted mana ability is
/// offered in the engine's `legal.mana_abilities` and skips the stack
/// like any other (CR 605.1), and it is what `PublicObject::granted_mana`
/// projects to a client through `effects::granted_activated` and
/// [`baylee_cards_dsl::simple_mana`]. So a wrong flag here is the failure
/// `docs/protocol.md` §"Granted mana" names from the other side — a land
/// the mana planner counts on and the engine then refuses — and the next
/// person to touch it will be arriving from the client.
///
/// Two of the three faults reach a grant and the third **cannot**:
/// `GrantActivated` has no `target` field, so "claims a mana ability that
/// targets" is structurally impossible rather than unchecked. The [`None`]
/// passed below is that sentence, not an omission.
///
/// # The floor is the three doors, not the grants
///
/// A grant is written one of three ways — an [`AbilityDef::Static`], an
/// [`Effect::CreateContinuousEffect`] inside an effect list, or a
/// [`CopyMod::Grant`] inside a copy clause. Chromatic Lantern and Great
/// Divide Guide are the first; both of Urza's Saga's are the second;
/// Machine God's Effigy's "except it has '{T}: Add {U}'" is the third,
/// and arrived after the first two had been counted, which is the whole
/// argument for counting doors. A count over the pool would clear a floor
/// of four the moment four grants of *one* shape existed, so it stops
/// separating anything as soon as the population grows past it. Each door
/// is counted on its own instead: that is anchored on the structure the
/// walk has to reach, which cannot drift with the pool.
///
/// [`CopyMod::Grant`]: crate::dsl::ability::CopyMod::Grant
#[test]
fn no_granted_ability_breaks_the_rule_a_printed_one_is_held_to() {
    use crate::lines::GrantDoor;
    let mut wrong = Vec::new();
    let (mut by_static, mut by_effect, mut by_copy) = (0_usize, 0_usize, 0_usize);
    // What `Effect::walk` counts, kept because its own doc says why: a
    // door reporting nought is only news once the walk says how much it
    // read to get there.
    let mut effects_read = 0_usize;
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            // Copiable exceptions are real abilities, not modifier grants.
            for granted in copiable_grants(ability) {
                by_copy += 1;
                if let Some(fault) = mana_ability_fault(granted) {
                    wrong.push(format!("{} grants an ability that {fault}", def.name()));
                }
            }
            // The three doors are `lines::grants_in`'s, the walk the view
            // finds a grant's sentence with, so a grant this holds to
            // CR 605.1 is one a player can be shown the source of.
            for (door, modifier) in crate::lines::grants_in(ability, &mut effects_read) {
                match door {
                    GrantDoor::Static => by_static += 1,
                    GrantDoor::Effect => by_effect += 1,
                    GrantDoor::Copy => by_copy += 1,
                }
                let Some((claimed, cost, effects)) = as_granted_activated(modifier) else {
                    continue;
                };
                if let Some(fault) = mana_ability_fault_of(claimed, &cost, effects, None) {
                    wrong.push(format!("{} grants an ability that {fault}", def.name()));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} granted ability/abilities disagree with CR 605.1:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // See the doc comment: doors, not a population.
    assert!(
        by_static >= 1 && by_effect >= 1 && by_copy >= 1,
        "the walk found {by_static} grant(s) written as a static, \
         {by_effect} written inside an effect list and {by_copy} inside a \
         copy clause, over {effects_read} effects read, and this pool has \
         all three shapes. A nought is a door the sweep stopped descending \
         into, which is how a clean report comes to be written over half a \
         population"
    );
}

/// The lints above that read an ability alone, over the permanents no
/// card prints.
///
/// A `TokenDef` carries `abilities: &[AbilityDef]` that the engine reads
/// through the very path a card face's are read by, so both shapes are
/// decidable there and neither sweep above can see one: they start at
/// [`crate::all`], the card registry, and `tokens.rs` sits beside
/// `cards/`. It is a door that has already swallowed a defect —
/// `offer_tests::no_token_carries_an_ability_the_engine_will_never_offer`
/// exists because a pool-wide grep scoped to `cards/` missed the Blood
/// token entirely.
///
/// CR 605.1 is the half that is not hypothetical here. The Treasure's
/// `{T}, Sacrifice this artifact: Add one mana of any color` really is a
/// mana ability, and every token's flag is written out by hand in
/// `tokens.rs` — a `false` there is a Treasure that cannot be cracked
/// while paying for a spell, which is the whole of what a Treasure is
/// for.
#[test]
fn no_token_breaks_a_lint_the_cards_are_held_to() {
    let mut wrong = Vec::new();
    let mut seen = 0_usize;
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            seen += 1;
            if let Some(filter) = target_reuse(ability) {
                wrong.push(format!("{} sweeps with {filter:?}", token.name));
            }
            if let Some(fault) = mana_ability_fault(ability) {
                wrong.push(format!("{} {fault}", token.name));
            }
            if let Some(spec) = controller_of_a_player_target(ability) {
                wrong.push(format!("{} reads the controller of {spec:?}", token.name));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} token ability/abilities are built in a shape that cannot be \
         right:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // The floor, as in the sweeps above: the four artifact tokens each
    // carry one ability, and a walk that inspected none of them would
    // pass exactly as loudly as this one.
    assert!(
        seen >= 4,
        "only {seen} token abilities were looked at; the sweep is not \
         reaching `tokens::ALL`"
    );
}

/// And the mana lint bites in both directions.
#[test]
fn the_mana_lint_catches_both_halves_of_cr_605_1() {
    use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
    use crate::dsl::effect::Amount;
    static MANA: [Effect; 1] = [Effect::mana(baylee_core::mana::ManaColor::Green, 1)];
    static DRAW: [Effect; 1] = [Effect::DrawCards {
        amount: Amount::Fixed(1),
    }];
    static MANA_AND_DRAW: [Effect; 2] = [
        Effect::mana(baylee_core::mana::ManaColor::Green, 1),
        Effect::DrawCards {
            amount: Amount::Fixed(1),
        },
    ];
    static A_CREATURE: crate::dsl::effect::TargetReq =
        crate::dsl::effect::TargetReq::one(TargetSpec::Object(&Filter::CREATURE));

    let unmarked = AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &MANA,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    };
    assert!(
        mana_ability_fault(&unmarked).is_some(),
        "an ability that only adds mana and is not marked one slipped through"
    );

    // The case the `all` reading could not see: mana **and** a rider,
    // while library movement now changes the answer under CR 605.1a.
    let with_a_rider = AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &MANA_AND_DRAW,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    };
    assert!(
        mana_ability_fault(&with_a_rider).is_none(),
        "mana with a draw uses the stack under current CR 605.1a"
    );

    let lying = AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &DRAW,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    };
    assert!(
        mana_ability_fault(&lying).is_some(),
        "a mana ability that makes no mana slipped through"
    );

    // CR 605.1a through the *second* instance of the word "target": a
    // requirement there is a target like the first, and a lint that read
    // only `target` would call this a mana ability that targets nothing.
    let second_only = AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &MANA,
        targets: None,
        second_targets: Some(A_CREATURE),
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    };
    assert_eq!(
        mana_ability_fault(&second_only),
        Some("claims a mana ability that targets (CR 605.1a)"),
        "a mana ability targeting through its second instance slipped through"
    );
}
