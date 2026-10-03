use super::*;
use crate::object::PrintedFace;
use baylee_core::ids::CardIndex;

static ORIGINAL: &[AbilityDef] = &[AbilityDef::Ward { mana: 2 }];
static EXTRA: AbilityDef = AbilityDef::Ward { mana: 3 };

fn list(card: u32, abilities: &'static [AbilityDef]) -> AbilityList {
    AbilityList::from_static(abilities, PrintedFace::new(CardIndex::new(card), 0), None)
}

#[test]
fn second_generation_keeps_each_printed_origin_and_index() {
    let original = list(5, ORIGINAL);
    let copier = list(7, &[AbilityDef::Ward { mana: 1 }]);
    let first = compose(original, &copier, 0, &[CopyMod::GrantAbility(&EXTRA)], None);
    let second = compose(first.clone(), &list(9, &[]), 0, &[], None);
    assert_eq!(second.abilities.len(), 2);
    assert_eq!(second.abilities.get(0), Some(&AbilityDef::Ward { mana: 2 }));
    assert_eq!(second.abilities.get(1), Some(&EXTRA));
    assert_eq!(
        second.origin(0).origin.and_then(AbilityOrigin::printed),
        PrintedFace::new(CardIndex::new(5), 0)
    );
    assert_eq!(
        second.origin(1).origin.and_then(AbilityOrigin::printed),
        PrintedFace::new(CardIndex::new(7), 0)
    );
    assert_eq!(second.origin(1).copy_modifier, Some(0));
    assert_eq!(first.origin(1), second.origin(1));
}

#[test]
fn copy_exception_provenance_preserves_indices_above_u16() {
    let index = usize::from(u16::MAX) + 1;
    let mut mods = vec![CopyMod::NoManaCost; index + 1];
    mods[index] = CopyMod::GrantAbility(&EXTRA);
    let copied = compose(list(5, ORIGINAL), &list(7, ORIGINAL), 0, &mods, None);
    assert_eq!(
        copied.origin(1).copy_modifier,
        Some(u64::from(u16::MAX) + 1)
    );
    assert_eq!(
        copied.origin(1).ability_ref(),
        Some(baylee_core::ids::AbilityRef::new(CardIndex::new(7), 0))
    );
}

#[test]
fn unaddressable_ability_index_has_no_aliasing_printed_identity() {
    let Ok(index) = usize::try_from(u64::from(u32::MAX) + 1) else {
        return;
    };
    let origin = list(5, ORIGINAL).origin(index);
    assert_eq!(origin.origin, None);
    assert_eq!(origin.ability_ref(), None);
    assert_eq!(origin.index, baylee_core::ids::AbilityRef::SYNTHETIC);
}

#[test]
fn repeated_self_copies_keep_independent_ability_instances() {
    let own = list(7, ORIGINAL);
    let mut copied = compose(
        list(5, ORIGINAL),
        &own,
        0,
        &[CopyMod::GrantAbility(&EXTRA)],
        None,
    );
    let retained = copied.entry(1).expect("quoted ability");
    for expected in 3..12 {
        copied = compose(
            copied.clone(),
            &own,
            0,
            &[CopyMod::KeepResolvingAbility],
            Some(retained),
        );
        assert_eq!(copied.abilities.len(), expected);
        assert_eq!(
            copied
                .abilities
                .iter()
                .filter(|ability| **ability == EXTRA)
                .count(),
            expected - 1
        );
    }
    let captured = copied.abilities.iter();
    drop(copied);
    assert_eq!(
        captured.count(),
        11,
        "a snapshot survives the source changing form"
    );
}

#[test]
fn keep_other_abilities_is_copyable_and_has_no_static_only_limit() {
    let own = list(
        7,
        &[
            AbilityDef::CopyOnEnter {
                target: baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
                mods: &[CopyMod::KeepOtherAbilities],
            },
            AbilityDef::Ward { mana: 4 },
        ],
    );
    let copied = compose(
        list(5, ORIGINAL),
        &own,
        0,
        &[CopyMod::KeepOtherAbilities],
        None,
    );
    assert_eq!(copied.abilities.len(), 2);
    assert_eq!(copied.abilities.get(1), Some(&AbilityDef::Ward { mana: 4 }));
    assert_eq!(copied.origin(1).index, 1);
    let bundle = copied.into_bundle();
    assert_eq!(bundle.entries().len(), 2);
    assert_eq!(
        std::mem::size_of::<Option<Arc<AbilityBundle>>>(),
        std::mem::size_of::<usize>()
    );
}

#[test]
fn retaining_color_omits_the_copied_color_defining_ability() {
    use baylee_cards_dsl::{Filter, Modifier};
    use baylee_core::color::ColorSet;
    static DEVOID: &[AbilityDef] = &[
        baylee_cards_dsl::static_ability!(Filter::This, Modifier::SetColor(ColorSet::EMPTY)),
        AbilityDef::Ward { mana: 2 },
    ];
    let source = list(7, ORIGINAL);
    let ordinary = compose(list(5, DEVOID), &source, 0, &[], None);
    assert_eq!(ordinary.abilities.len(), 2);
    let retaining = compose(list(5, DEVOID), &source, 0, &[CopyMod::KeepColor], None);
    assert_eq!(retaining.abilities.len(), 1);
    assert_eq!(
        retaining.abilities.get(0),
        Some(&AbilityDef::Ward { mana: 2 })
    );
    assert_eq!(
        retaining.origin(0).index,
        1,
        "removing a CDA does not relabel the other sentence"
    );
    let second = compose(retaining, &source, 0, &[], None);
    assert_eq!(second.abilities.len(), 1, "the omission is itself copiable");
}

#[test]
fn token_defined_static_survives_second_copy_and_kept_ability_composition() {
    use crate::text_changes::{TextChangeMap, TextReplacement};
    use baylee_cards_dsl::{Filter, Modifier, TextWordKind};
    let mut words = TextChangeMap::IDENTITY;
    words.replace(TextReplacement {
        kind: TextWordKind::Color,
        from: 4,
        to: 1,
    });
    let ability = RuntimeStatic {
        modifier: Modifier::ModifyPTPerCount {
            filter: &Filter::ARTIFACT,
            p: 1,
            t: 1,
        },
        base_text: words,
    };
    let token = list(5, &[]).with_runtime_static(ability);
    let first = compose(token, &list(7, ORIGINAL), 0, &[CopyMod::KeepColor], None);
    let second = compose(first.clone(), &list(9, &[]), 0, &[], None);
    assert_eq!(second.runtime_statics(), &[ability]);
    let retaining = compose(
        first.clone(),
        &first,
        0,
        &[CopyMod::KeepOtherAbilities],
        None,
    );
    assert_eq!(retaining.runtime_statics(), &[ability, ability]);
    // A +1/+1-per-artifact static is not a characteristic-defining ability:
    // a copy with a fixed base P/T still has this ordinary layer-7c ability.
    let fixed = compose(second, &list(9, &[]), 0, &[CopyMod::SetPT(4, 4)], None);
    assert_eq!(fixed.runtime_statics(), &[ability]);
    assert_ne!(fixed.abilities, list(5, &[]).abilities);
}
