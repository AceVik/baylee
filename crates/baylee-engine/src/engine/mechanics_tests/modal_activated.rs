//! Chosen activation modes survive announcement, payment, source loss and copying.
use super::*;
use crate::choice::{CastModeKind, TargetPrompt};
use baylee_cards_dsl::{Amount, CostPart, Filter, ManaColor, SpellMode, TargetSpec};

const SOURCE: u32 = 7990;
const COPIER: u32 = 7991;
const MANA: u32 = 7992;
const CREATURE: u32 = 7993;

static OPTIONS: &[SpellMode] = &[
    SpellMode {
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ..SpellMode::new(&[Effect::GainLife {
            amount: Amount::Fixed(11),
        }])
    },
    SpellMode::new(&[Effect::GainLife {
        amount: Amount::Fixed(3),
    }]),
];
static SOURCE_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::modal_activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::PayLife(2), CostPart::SacrificeSelf],
        },
        OPTIONS
    ),
    baylee_cards_dsl::modal_activated!(
        Cost {
            mana: ManaCost::parse("{X}"),
            parts: &[CostPart::SacrificeSelf],
        },
        OPTIONS,
        x_value = baylee_cards_dsl::ActivationX::Defined(Amount::Fixed(1))
    ),
    baylee_cards_dsl::modal_activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::SacrificeSelf],
        },
        &[
            SpellMode {
                targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
                ..SpellMode::new(&[Effect::Destroy {
                    target: TargetSpec::Object(&Filter::CREATURE),
                    no_regen: false
                }])
            },
            SpellMode {
                targets: Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
                ..SpellMode::new(&[Effect::Destroy {
                    target: TargetSpec::Object(&Filter::LAND),
                    no_regen: false
                }])
            },
        ]
    ),
];
static COPIER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::CopyTargetAbility],
    Some(TargetReq::one(TargetSpec::AbilityOnStack(
        &Filter::ControlledByYou,
    ))),
)];
static MANA_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::modal_activated!(
    Cost::TAP,
    &[
        SpellMode::new(&[Effect::mana(ManaColor::Green, 1)]),
        SpellMode::new(&[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]),
    ],
    mana_ability = true
)];

fn setup(battlefield: &[u32]) -> Bench {
    let mut e = bench(
        7990,
        vec![
            card(
                SOURCE,
                face("Modal device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SOURCE_ABILITIES,
            ),
            card(
                COPIER,
                face("Ability copier", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                COPIER_ABILITIES,
            ),
            card(
                MANA,
                face("Modal mana", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                MANA_ABILITIES,
            ),
            card(
                CREATURE,
                creature_face("Target body", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(battlefield), Seat::default()],
    );
    to_main(&mut e, me());
    e
}

fn choose(e: &mut Bench, position: usize) {
    e.apply(me(), PlayerAction::ChooseMode(position)).unwrap();
}
fn target(e: &mut Bench, object: ObjectId) {
    e.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![object],
            players: vec![],
        },
    )
    .unwrap();
}

#[test]
fn illegal_mode_is_filtered_and_wrong_answers_leave_the_unpaid_prompt_intact() {
    let mut e = setup(&[SOURCE]);
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    let initial = life(&e, me());
    activate(&mut e, me(), source, 0);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("mode first")
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::Mode(1));
    assert_eq!(life(&e, me()), initial);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    let before = e.snapshot_hash();
    assert!(e.apply(them(), PlayerAction::ChooseMode(0)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    assert!(e.apply(me(), PlayerAction::ChooseMode(1)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    choose(&mut e, 0);
    assert_eq!(life(&e, me()), initial - 2);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    let stacked = e.state.zones.list(ZoneLocation::Stack)[0];
    assert_eq!(e.state.object(stacked).unwrap().mode_index, Some(1));
    settle(&mut e, me());
    assert_eq!(life(&e, me()), initial + 1);
}

#[test]
fn targets_are_chosen_before_sacrifice_and_the_copy_keeps_only_that_modes_targets() {
    let mut e = setup(&[SOURCE, COPIER, CREATURE, forest(), forest()]);
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    let copier = the(&e, ZoneLocation::Battlefield, COPIER);
    let body = the(&e, ZoneLocation::Battlefield, CREATURE);
    let lands = objects(&e, ZoneLocation::Battlefield, forest());
    activate(&mut e, me(), source, 2);
    choose(&mut e, 1);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("mode targets")
    };
    assert!(!options.contains(&body));
    target(&mut e, lands[0]);
    let original = e.state.zones.list(ZoneLocation::Stack)[0];
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    activate(&mut e, me(), copier, 0);
    target(&mut e, original);
    walk_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(options.contains(&lands[1]));
    assert!(!options.contains(&body));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![body],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    target(&mut e, lands[1]);
    assert!(e.state.zones.list(ZoneLocation::Stack).iter().all(|id| {
        let object = e.state.object(*id).unwrap();
        object.mode_index == Some(1)
    }));
    settle(&mut e, me());
    assert_eq!(e.state.object(lands[0]).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(lands[1]).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(body).unwrap().zone, Zone::Battlefield);
}

#[test]
fn nested_modal_mana_keeps_the_outer_mode_and_only_pays_after_the_window_closes() {
    let mut e = setup(&[SOURCE, MANA]);
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    let mana = the(&e, ZoneLocation::Battlefield, MANA);
    let initial = life(&e, me());
    activate(&mut e, me(), source, 1);
    choose(&mut e, 0); // The only legal outer mode is original mode 1.
    assert!(e.mana_window.is_some());
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    activate(&mut e, me(), mana, 0);
    choose(&mut e, 0);
    assert!(e.state.zones.stack_is_empty());
    e.apply(me(), PlayerAction::PayWindow { steps: vec![] })
        .unwrap();
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    let original = e.state.zones.list(ZoneLocation::Stack)[0];
    assert_eq!(e.state.object(original).unwrap().mode_index, Some(1));
    settle(&mut e, me());
    assert_eq!(life(&e, me()), initial + 3);
}

#[test]
fn leaving_an_unpaid_modal_window_reverses_the_attempt_and_clears_its_mode() {
    let mut e = setup(&[SOURCE, MANA]);
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    activate(&mut e, me(), source, 1);
    choose(&mut e, 0);
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    assert!(e.mana_window.is_none());
    assert!(e.activation_mode.is_none());
    assert!(e.state.zones.stack_is_empty());
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    activate(&mut e, me(), source, 0);
    assert!(matches!(e.pending(), Pending::ChooseCastMode { .. }));
}

#[test]
fn a_non_mana_producing_mode_of_a_mana_ability_still_resolves_immediately() {
    let mut e = setup(&[MANA]);
    let mana = the(&e, ZoneLocation::Battlefield, MANA);
    let initial = life(&e, me());
    activate(&mut e, me(), mana, 0);
    choose(&mut e, 1);
    assert!(e.state.zones.stack_is_empty());
    assert_eq!(life(&e, me()), initial + 1);
}

#[test]
fn unsupported_mode_costs_are_refused_atomically_instead_of_being_waived() {
    for override_cost in [true, false] {
        let mut mode = SpellMode::new(&[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]);
        if override_cost {
            mode.cost_override = Some(ManaCost::parse("{2}"));
        } else {
            mode.additional_cost = Some(ManaCost::parse("{2}"));
        }
        let mut definition = SOURCE_ABILITIES[0];
        let AbilityDef::ModalActivated { modes, .. } = &mut definition else {
            unreachable!()
        };
        *modes = Box::leak(vec![mode].into_boxed_slice());
        let abilities = Box::leak(vec![definition].into_boxed_slice());
        let mut e = bench(
            7994,
            vec![card(
                SOURCE,
                face("Unsupported modal price", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                abilities,
            )],
            [Seat::with(&[SOURCE]), Seat::default()],
        );
        to_main(&mut e, me());
        let source = the(&e, ZoneLocation::Battlefield, SOURCE);
        assert!(!offer(&e).1.abilities.contains(&(source, 0)));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                me(),
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn a_granted_modal_ability_retains_its_selected_effect_after_sacrificing_the_grantor() {
    static GRANT_EFFECTS: &[Effect] = &[Effect::continuous(
        &Filter::This,
        baylee_cards_dsl::Modifier::GrantActivatedDefinition(&SOURCE_ABILITIES[0]),
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    )];
    static GRANT: &[AbilityDef] = &[free(GRANT_EFFECTS, None)];
    let mut e = bench(
        7995,
        vec![card(
            SOURCE,
            face("Granted modal device", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            GRANT,
        )],
        [Seat::with(&[SOURCE]), Seat::default()],
    );
    to_main(&mut e, me());
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    activate(&mut e, me(), source, 0);
    settle(&mut e, me());
    let initial = life(&e, me());
    let (_, index) = offer(&e)
        .1
        .abilities
        .into_iter()
        .find(|(id, index)| *id == source && crate::choice::granted_slot(*index).is_some())
        .expect("granted modal ability offered");
    activate(&mut e, me(), source, index);
    choose(&mut e, 0);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    settle(&mut e, me());
    assert_eq!(life(&e, me()), initial + 1);
}
