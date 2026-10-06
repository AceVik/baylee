use super::*;

/// What the view cannot see, the reader says it cannot see.
///
/// Each of the three refusals is a `Filter` whose answer lives in a
/// `GameState` field the projection has no counterpart for, and the
/// reason each one is a refusal rather than a gap is in this module's
/// documentation. `Some(false)` here would be the fault the three-valued
/// answer exists to prevent: a caller cannot tell a read "no" from an
/// unread one, so it would act on a guess wearing a reading's clothes.
///
/// `This` and `Another` join them when the caller has no source object,
/// which is a different kind of unknown with the same honest answer.
#[test]
fn a_filter_the_view_cannot_answer_is_not_answered_no() {
    use baylee_cards_dsl::{Filter, ZoneRef};
    let me = PlayerId::new(0);
    let a = agent();
    let mine = permanent(obj(1), me, 2);
    let v = view(0, &[20, 20], vec![mine.clone()]);
    let read = |f: &Filter, this| a.filter_matches(f, &v, &mine, ZoneRef::Battlefield, this);

    for filter in [
        Filter::MatchesChosenTypeOfSource,
        Filter::AttachedToBySource,
        Filter::SharesSubtypeWithCommander,
    ] {
        assert_eq!(
            read(&filter, Some(obj(1))),
            None,
            "{filter:?} reads a field no view carries, and saying `false` \
             would look exactly like a read answer"
        );
    }
    // `IsToken` is the fourth, and only for the object that carries
    // neither handle: a face-down permanent and a token copying a card
    // are one shape in a view and opposite answers in the rules.
    assert_eq!(read(&Filter::IsToken, Some(obj(1))), None);
    let mut registry = mine.clone();
    registry.token = Some(1);
    assert_eq!(
        a.filter_matches(&Filter::IsToken, &v, &registry, ZoneRef::Battlefield, None),
        Some(true)
    );
    let seen = carded(mine.clone(), "Baleful Strix", TypeSet::CREATURE);
    assert_eq!(
        a.filter_matches(&Filter::IsToken, &v, &seen, ZoneRef::Battlefield, None),
        Some(false)
    );

    assert_eq!(read(&Filter::This, None), None);
    assert_eq!(read(&Filter::Another, None), None);
    assert_eq!(read(&Filter::This, Some(obj(1))), Some(true));
    assert_eq!(read(&Filter::Another, Some(obj(1))), Some(false));
    assert_eq!(read(&Filter::CREATURE, None), Some(true));
}

/// An unknown part does not settle a question the rest of it settles.
///
/// Kleene's three-valued `and`/`or`, which is the only combination rule
/// that keeps a `None` honest: a false conjunct makes an `And` false
/// however much of the rest is unreadable, a true disjunct makes an `Or`
/// true the same way, and an unknown wins everywhere else. Reading the
/// unknown as `false` would answer "nontoken creature that shares a
/// subtype with your commander" with a confident no on every board.
#[test]
fn an_unreadable_part_settles_a_filter_only_where_it_decides_it() {
    use baylee_cards_dsl::{Filter, ZoneRef};
    static UNREADABLE: Filter = Filter::SharesSubtypeWithCommander;
    static AND_FALSE: Filter = Filter::And(&[UNREADABLE, Filter::PLANESWALKER]);
    static AND_TRUE: Filter = Filter::And(&[UNREADABLE, Filter::CREATURE]);
    static OR_TRUE: Filter = Filter::Or(&[UNREADABLE, Filter::CREATURE]);
    static OR_FALSE: Filter = Filter::Or(&[UNREADABLE, Filter::PLANESWALKER]);

    let a = agent();
    let mine = permanent(obj(1), PlayerId::new(0), 2);
    let v = view(0, &[20, 20], vec![mine.clone()]);
    let read = |f: &Filter| a.filter_matches(f, &v, &mine, ZoneRef::Battlefield, Some(obj(1)));

    assert_eq!(read(&AND_FALSE), Some(false), "a false conjunct settles it");
    assert_eq!(read(&AND_TRUE), None, "a true one leaves the unknown");
    assert_eq!(read(&OR_TRUE), Some(true), "a true disjunct settles it");
    assert_eq!(read(&OR_FALSE), None, "a false one leaves the unknown");
    assert_eq!(read(&Filter::Not(&UNREADABLE)), None);
}

/// A seat that agreed to a price pays it (CR 605.3a).
///
/// The mana window is an ordinary priority round, which is what hid it:
/// nothing is castable inside one, so every path in `priority` below the
/// new first step passed, and the agent lost a spell it had already
/// agreed to pay for. Three questions, because the failure was that the
/// first one was never asked and the other two are what stop the answer
/// being reckless.
#[test]
fn a_payment_window_is_answered_by_tapping_toward_the_price() {
    use baylee_core::generated::subtypes::land;
    let mut lands = Vec::new();
    for i in 0..2u32 {
        let mut source = permanent(obj(i + 1), PlayerId::new(0), 0);
        source.types = TypeSet::LAND;
        source.subtypes.insert(land::PLAINS);
        lands.push(source);
    }
    let legal = || {
        Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            mana_abilities: (1..=2).map(obj).collect(),
            ..Default::default()
        })
    };
    let base = view(0, &[20, 20], lands);

    // Owed {2} with two Plains offered: one tap, and the next round
    // plans one fewer because `plan` spends the pool first.
    let mut v = base.clone();
    v.awaiting = Some(v.seat);
    v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        baylee_core::mana::ManaCost::from_symbol_generic(2),
    ));
    let pending = Pending::Priority {
        player: v.seat,
        legal: legal(),
    };
    assert!(
        matches!(
            agent().act(&v, &pending),
            PlayerAction::ActivateManaAbility { .. }
        ),
        "the seat agreed to pay {{2}} and was handed priority over two \
         untapped Plains; passing there is how the spell was lost"
    );

    // Owed {3} with two Plains: the price cannot be reached, so nothing
    // is tapped. A seat that taps two of the three lands it needs has
    // lost the mana and the spell both.
    let mut v = base.clone();
    v.awaiting = Some(v.seat);
    v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        baylee_core::mana::ManaCost::from_symbol_generic(3),
    ));
    let pending = Pending::Priority {
        player: v.seat,
        legal: legal(),
    };
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "a window the seat cannot afford must cost it nothing more"
    );

    // The same price, owed by somebody else. Both fields ride in every
    // view, so a reader taking `owed` without `awaiting` would have this
    // seat paying for an opponent's window.
    let mut v = base;
    v.awaiting = Some(PlayerId::new(1));
    v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        baylee_core::mana::ManaCost::from_symbol_generic(2),
    ));
    let pending = Pending::Priority {
        player: v.seat,
        legal: legal(),
    };
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "this seat is not the one being asked for the payment"
    );
}

fn mana_permission() -> baylee_engine::choice::GrantedActionOffer {
    use baylee_core::mana::ManaColor;
    baylee_engine::choice::GrantedActionOffer {
        id: baylee_core::ids::GrantedActionId::new(7),
        source: baylee_core::ids::DamageSourceRef {
            object: obj(77),
            version: 1,
        },
        ability: None,
        timing: baylee_cards_dsl::SpecialActionTiming::ManaAbility,
        cost: baylee_cards_dsl::SpecialActionCost::Life(1),
        effect: baylee_engine::choice::GrantedActionKind::AddMana {
            color: ManaColor::Colorless,
            amount: 1,
        },
    }
}

#[test]
fn granted_mana_pays_only_an_affordable_chosen_debt_and_stops() {
    use baylee_core::mana::{ManaCost, ManaPayment};
    use baylee_engine::choice::LegalActions;
    let grant = mana_permission();
    let legal = LegalActions {
        can_pass: true,
        granted_actions: vec![grant.clone()],
        ..Default::default()
    };
    let mut v = view(0, &[20, 20], vec![]);
    v.awaiting = Some(v.seat);
    v.owed = Some(ManaPayment::Fixed(ManaCost::parse("{3}")));
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(legal.clone()),
    };
    for floating in 0..3 {
        v.seats[0].mana_pool.colorless = floating;
        assert_eq!(
            agent().act(&v, &pending),
            PlayerAction::TakeGrantedAction { id: grant.id }
        );
    }
    v.seats[0].mana_pool.colorless = 3;
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    v.seats[0].mana_pool.colorless = 0;
    v.owed = Some(ManaPayment::Fixed(ManaCost::parse("{G}")));
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    v.owed = Some(ManaPayment::Fixed(ManaCost::parse("{20}")));
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "never spend the last life for a generic debt"
    );
    v.owed = None;
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "no speculative life conversion"
    );
}

#[test]
fn granted_shield_spends_one_spare_unit_against_a_visible_targeted_threat() {
    use baylee_core::ids::TargetRef;
    use baylee_core::mana::ManaCost;
    use baylee_engine::choice::{GrantedActionKind, LegalActions};
    let mut grant = mana_permission();
    grant.timing = baylee_cards_dsl::SpecialActionTiming::Priority;
    grant.cost = baylee_cards_dsl::SpecialActionCost::Mana(ManaCost::parse("{1}"));
    grant.effect = GrantedActionKind::PreventNextDamage {
        target: TargetRef::Player(PlayerId::new(0)),
        amount: 1,
    };
    let legal = LegalActions {
        can_pass: true,
        granted_actions: vec![grant.clone()],
        ..Default::default()
    };
    let mut v = view(0, &[20, 20], vec![]);
    let mut bolt = stack_spell(2, "Lightning Bolt", PlayerId::new(1), obj(9));
    bolt.targets = vec![TargetRef::Player(v.seat)];
    v.stack.push(bolt);
    v.seats[0].mana_pool.white = 1;
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(legal),
    };
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::TakeGrantedAction { id: grant.id }
    );
    v.seats[0].mana_pool.white = 0;
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    v.seats[0].mana_pool.white = 1;
    v.stack[0].targets = vec![TargetRef::Player(PlayerId::new(1))];
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
}

#[test]
fn optional_prevention_uses_available_mana_without_overpaying() {
    use baylee_core::generated::subtypes::land;
    use baylee_core::mana::ManaPayment;
    use baylee_engine::choice::{LegalActions, NumberPrompt};
    let mut source = permanent(obj(1), PlayerId::new(0), 0);
    source.types = TypeSet::LAND;
    source.subtypes.insert(land::PLAINS);
    let mut v = view(0, &[20, 20], vec![source]);
    v.awaiting = Some(v.seat);
    v.owed = Some(ManaPayment::AnyAmount {
        preventable_damage: 2,
    });
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: vec![obj(1)],
        ..Default::default()
    };
    assert_eq!(
        policy::pay_owed(&v, &legal),
        Some(PlayerAction::ActivateManaAbility { source: obj(1) }),
        "one available mana still prevents one damage"
    );
    v.seats[0].mana_pool.white = 2;
    assert_eq!(
        policy::pay_owed(&v, &legal),
        None,
        "a covered optional payment must not tap an extra source"
    );
    for (max, want) in [(0, 0), (1, 1), (2, 2), (80, 2)] {
        assert_eq!(
            agent().act(
                &v,
                &Pending::ChooseNumber {
                    player: v.seat,
                    min: 0,
                    max,
                    reason: NumberPrompt::ManaPayment {
                        preventable_damage: 2
                    },
                }
            ),
            PlayerAction::ChooseNumber(want)
        );
    }
}

#[test]
fn sacrifice_for_another_player_respects_which_side_loses_the_permanent() {
    use baylee_engine::choice::ChoicePrompt;
    let other = PlayerId::new(1);
    let v = view(
        0,
        &[20, 20],
        vec![permanent(obj(1), other, 1), permanent(obj(2), other, 7)],
    );
    let pending = Pending::ChooseCards {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::SacrificeFor { player: other },
        total: None,
    };
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::ChooseObjects {
            objects: vec![obj(2)]
        }
    );
    let teammate = agent().with_teams(vec![Some(1), Some(1)]);
    assert_eq!(
        teammate.act(&v, &pending),
        PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }
    );
}

#[test]
fn evenly_split_burn_does_not_add_unbudgeted_targets() {
    use baylee_cards_dsl::{Amount, Effect, TargetSpec};
    use baylee_engine::engine::DecisionContext;
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), PlayerId::new(1), 2),
            permanent(obj(2), PlayerId::new(1), 3),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![PlayerId::new(1)],
        min: 0,
        max: 255,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::DealDamageEvenly {
        amount: Amount::X,
        target: TargetSpec::AnyTarget,
    }];
    let action = agent().act_with_context(
        &v,
        &pending,
        &DecisionContext {
            effects: &effects,
            x: 4,
            ..Default::default()
        },
    );
    let count = match action {
        PlayerAction::ChooseObjects { objects } => objects.len(),
        PlayerAction::ChooseTargets { objects, players } => objects.len() + players.len(),
        other => panic!("expected a target selection, got {other:?}"),
    };
    assert_eq!(
        count, 1,
        "concentrate damage without incurring another target's cost"
    );
}

#[test]
fn mandatory_target_selection_is_not_limited_to_a_byte() {
    let options: Vec<_> = (1..=300).map(obj).collect();
    let v = view(0, &[20, 20], vec![]);
    let action = agent().act(
        &v,
        &Pending::ChooseTargets {
            player: v.seat,
            options: options.clone(),
            player_options: vec![],
            min: 300,
            max: 300,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        },
    );
    let chosen = match action {
        PlayerAction::ChooseObjects { objects } | PlayerAction::ChooseTargets { objects, .. } => {
            objects
        }
        other => panic!("expected target selection, got {other:?}"),
    };
    assert_eq!(chosen, options);
}

/// A colour named inside this seat's own payment window is named for the
/// price, and outside it for the hand.
///
/// Owed `{1}{U}{U}` with an Island and a Mountain still untapped: the
/// dual just tapped must make blue, or the two lands left cannot finish
/// the price. Before, the question was answered for the (empty) hand and
/// named white, and the seat lost the pact it had agreed to pay. The
/// second half is the same prompt with the price owed by the other seat:
/// there the hand's Brainstorm decides, as it always did.
#[test]
fn a_colour_named_in_a_payment_window_keeps_the_price_payable() {
    use baylee_core::generated::subtypes::land;
    use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
    let seat = PlayerId::new(0);
    let lands = [land::ISLAND, land::MOUNTAIN]
        .into_iter()
        .enumerate()
        .map(|(i, basic)| {
            let mut source = permanent(obj(u32::try_from(i).unwrap() + 1), seat, 0);
            source.types = TypeSet::LAND;
            source.power = None;
            source.toughness = None;
            source.subtypes.insert(basic);
            source
        })
        .collect();
    let pending = Pending::ChooseColor {
        player: seat,
        options: vec![ManaColor::White, ManaColor::Blue],
    };
    let base = view(0, &[20, 20], lands);

    let mut v = base.clone();
    v.awaiting = Some(seat);
    v.owed = Some(baylee_core::mana::ManaPayment::Fixed(ManaCost::parse(
        "{1}{U}{U}",
    )));
    for profile in [
        AIProfile::NOVICE,
        AIProfile::CASUAL,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::ChooseColor(ManaColor::Blue),
            "{profile:?} named a colour that leaves {{1}}{{U}}{{U}} out of \
             reach of an Island and a Mountain"
        );
    }

    let mut v = base;
    v.awaiting = Some(PlayerId::new(1));
    v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        ManaCost::from_symbol(ManaSymbol::White),
    ));
    v.hand = vec![hand_card(3, "Brainstorm")];
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
        PlayerAction::ChooseColor(ManaColor::Blue),
        "the other seat's price is not this seat's to name a colour for"
    );
}

/// A mana ability a continuous effect granted is one source, read the
/// same way at both ends of the planner.
///
/// The ability is printed on no card: a Chromatic Lantern's grant reaches
/// the seat as `PublicObject::granted_mana` plus a synthetic index
/// (`choice::granted_ability`), and `baylee-ai` decodes that in two
/// places. `policy::sources` turns the engine's own offer into taps, and
/// `policy::remaining_sources` manufactures the same offer out of the
/// battlefield so a colour response can be priced before anything is
/// tapped. Neither is the defect on its own — the defect is the pair
/// *disagreeing*, which is a land the planner counts on and the engine
/// then refuses, or a colour the seat could have had and never asks for.
///
/// So one board and one question, asked from both ends, twice: with the
/// grant both must say yes, and without it both must say no. Two
/// independent assertions would each stay green while drifting apart,
/// which is the one failure they exist to catch. The second half is also
/// the premise — this permanent has no card, no basic land type and no
/// printed ability, so the grant is the only mana on the table and a
/// green first half cannot be coming from anywhere else.
#[test]
fn a_granted_mana_ability_is_read_the_same_by_the_estimate_and_the_offer() {
    use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
    use baylee_engine::choice::granted_ability;
    use baylee_view::GrantedMana;

    let seat = PlayerId::new(0);
    let land = obj(1);
    let board = |granted: bool| {
        let mut object = permanent(land, seat, 0);
        object.types = TypeSet::LAND;
        object.power = None;
        object.toughness = None;
        object.granted_mana = granted.then(|| GrantedMana {
            slot: 0,
            colors: vec![ManaColor::Green],
            amount: 1,
        });
        let mut v = view(0, &[20, 20], vec![object]);
        v.awaiting = Some(v.seat);
        v.owed = Some(baylee_core::mana::ManaPayment::Fixed(
            ManaCost::from_symbol(ManaSymbol::Green),
        ));
        v
    };
    let pending = || Pending::Priority {
        player: seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            abilities: vec![(land, granted_ability(0))],
            ..Default::default()
        }),
    };
    let green = ManaCost::from_symbol(ManaSymbol::Green);

    // With the grant: the estimate counts it, and the offer taps it.
    let granted = board(true);
    assert!(
        policy::can_pay(&granted, &green),
        "the estimate priced {{G}} as unpayable over a land that was \
         granted a green mana ability: a colour response the seat can \
         afford is one it will never reach for"
    );
    assert_eq!(
        agent().act(&granted, &pending()),
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: granted_ability(0),
        },
        "the engine offered the granted ability and owed {{G}}, and the \
         agent passed over it"
    );

    // Without it: both ends say no, which is what makes the pair above a
    // reading of the grant and not of anything else on the board.
    let bare = board(false);
    assert!(
        !policy::can_pay(&bare, &green),
        "the premise: with no grant this permanent makes no mana at all"
    );
    assert_eq!(
        agent().act(&bare, &pending()),
        PlayerAction::PassPriority,
        "nothing on this board can make green, so there is nothing to tap"
    );
}

/// A counterspell printed behind "unless you pay" is a counterspell.
///
/// Flusterstorm and Malevolent Hermit both spell their text as
/// `PlayerMayPayOr { effect: CounterTargetSpell }` — the spell is
/// countered unless its controller pays — and that variant carries a
/// single `&'static Effect` rather than a list. Every hand-rolled walker
/// in this workspace descended into the lists and stopped, so `meaning`
/// was handed these two cards and read an effect list that says nothing
/// at all. An agent holding a counterspell it does not know is a
/// counterspell holds it for ever: `policy` only casts one when there is
/// an opposing stack entry, and it never asks unless `counter` is set.
///
/// Two real cards and not a constructed effect, because this is the one
/// reader whose answer the pool actually moves — the census behind #109
/// found 35 effects behind that clause and these are the two that any
/// reader here asks about.
#[test]
fn a_counterspell_behind_a_price_is_read_as_one() {
    for name in ["Flusterstorm", "Malevolent Hermit"] {
        let index = baylee_cards::decks::by_name(name).expect("card in the pool");
        let def = baylee_cards::by_index(index).expect("card compiles");
        let counters = def.faces.iter().enumerate().any(|(face, _)| {
            def.abilities_for_face(face).iter().any(|ability| {
                let effects = match ability {
                    AbilityDef::Spell { effects, .. }
                    | AbilityDef::Triggered { effects, .. }
                    | AbilityDef::Activated { effects, .. }
                    | AbilityDef::ActivatedConditional { effects, .. } => *effects,
                    _ => &[],
                };
                tactics::meaning(effects, 0).counter
            })
        });
        assert!(
            counters,
            "{name} counters a spell unless its controller pays, and the \
             agent reads it as an effect list with no meaning"
        );
    }
}
