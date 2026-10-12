//! Selected mode sets, kicker and copied targeting permissions.
#[allow(clippy::wildcard_imports)]
use super::*;
const MULTI_EDGE: u32 = 98_724;
const KICK_EDGE: u32 = 98_725;
static UNION_MODES: &[SpellMode] = &[
    baylee_cards_dsl::mode!(
        EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL)))
    ),
    baylee_cards_dsl::mode!(
        EFFECTS,
        targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::ARTIFACT)))
    ),
];
static UNION_ABILITIES: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: UNION_MODES,
    choose: ModeCount::TWO,
}];
fn select_both(e: &mut Engine<SyntheticLookup>) {
    if let Pending::ChooseCastMode { options, .. } = e.pending().clone() {
        let both = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Modes(3)))
            .expect("selected two-mode set");
        e.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    }
    assert!(
        matches!(e.pending(), Pending::ChooseTargets { .. }),
        "{:?}",
        e.pending()
    );
}
fn empty_optional_artifact(e: &mut Engine<SyntheticLookup>) {
    if let Pending::ChooseTargets {
        min, max, options, ..
    } = e.pending()
    {
        assert_eq!((*min, *max), (0, 1));
        assert!(options.iter().all(|id| {
            e.state
                .object(*id)
                .unwrap()
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::ARTIFACT)
        }));
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    }
}
#[test]
fn selected_wall_and_optional_artifact_modes_allow_shadows_even_without_artifact_candidates() {
    for artifact in [false, true] {
        let def = fixture(MULTI_EDGE, index::LIGHTNING_BOLT, UNION_ABILITIES);
        let mut board = vec![index::WALL_OF_SHADOWS.get(), index::WALL_OF_STONE.get()];
        if artifact {
            board.push(index::SOL_RING.get());
        }
        let mut e = setup(def, &[MULTI_EDGE], &board);
        let shadow = object(&e, index::WALL_OF_SHADOWS.get());
        cast(&mut e, MULTI_EDGE);
        select_both(&mut e);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            unreachable!()
        };
        assert!(
            options.contains(&shadow),
            "permission includes optional artifacts regardless of current candidates"
        );
        target(&mut e, shadow);
        empty_optional_artifact(&mut e);
        walk(&mut e, empty);
        assert_eq!(
            e.state.object(shadow).unwrap().zone,
            crate::zone::Zone::Battlefield
        );
    }
}
#[test]
fn kicked_wall_first_and_optional_artifact_second_keep_full_domain() {
    static PAIR: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::ARTIFACT)))
    )];
    let baseline = fixture(KICK_EDGE, index::LIGHTNING_BOLT, PAIR);
    let face = baylee_cards_dsl::FaceDef {
        additional_costs: &[baylee_cards_dsl::cost!("{1}")],
        kicked_targets: Some(TargetReq::one(TargetSpec::Object(&WALL))),
        ..baseline.faces[0]
    };
    let def = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([face])),
        ..*baseline
    }));
    let mut e = setup(
        def,
        &[KICK_EDGE],
        &[index::WALL_OF_SHADOWS.get(), index::WALL_OF_STONE.get()],
    );
    let shadow = object(&e, index::WALL_OF_SHADOWS.get());
    cast(&mut e, KICK_EDGE);
    match e.pending().clone() {
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Kicker,
            ..
        } => {
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        }
        Pending::ChooseCastMode { options, .. } => {
            let kicked = options
                .iter()
                .position(|o| matches!(o.kind, crate::choice::CastModeKind::Kicked))
                .expect("kicked offer");
            e.apply(P0, PlayerAction::ChooseMode(kicked)).unwrap();
        }
        other => panic!("kicker decision required: {other:?}"),
    }
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(
        options.contains(&shadow),
        "kicked first becomes Wall-only but second remains artifact permission"
    );
    target(&mut e, shadow);
    empty_optional_artifact(&mut e);
    walk(&mut e, empty);
}
#[test]
fn copied_two_mode_spell_keeps_broad_domain_on_its_wall_retarget_menu() {
    let def = fixture(MULTI_EDGE, index::LIGHTNING_BOLT, UNION_ABILITIES);
    let mut e = setup(
        def,
        &[MULTI_EDGE, index::FORK.get()],
        &[
            index::WALL_OF_SHADOWS.get(),
            index::WALL_OF_STONE.get(),
            index::SOL_RING.get(),
        ],
    );
    let shadow = object(&e, index::WALL_OF_SHADOWS.get());
    let stone = object(&e, index::WALL_OF_STONE.get());
    let ring = object(&e, index::SOL_RING.get());
    cast(&mut e, MULTI_EDGE);
    select_both(&mut e);
    target(&mut e, stone);
    target(&mut e, ring);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, index::FORK.get());
    target(&mut e, original);
    walk(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(
        options.contains(&shadow),
        "copy preserves artifact mode while offering Wall target group"
    );
    target(&mut e, shadow);
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        target(&mut e, ring);
    }
    walk(&mut e, empty);
    assert_eq!(
        e.state.object(shadow).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}
