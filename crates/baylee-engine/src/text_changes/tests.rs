use super::*;

#[test]
fn replacements_compose_in_order_and_preserve_semantic_families() {
    let mut map = TextChangeMap::IDENTITY;
    assert!(map.replace(TextReplacement {
        kind: TextWordKind::Color,
        from: 2,
        to: 1
    }));
    assert!(map.replace(TextReplacement {
        kind: TextWordKind::Color,
        from: 1,
        to: 4
    }));
    assert_eq!(
        map.color_words(ColorSet::from_slice(&[Color::Black, Color::Blue])),
        ColorSet::of(Color::Green)
    );
    assert_eq!(map.land_type(land::SWAMP), land::SWAMP);
    assert_eq!(
        map.filter_leaf(Filter::Named("Black Vise")),
        Filter::Named("Black Vise")
    );
    assert!(map.replace(TextReplacement {
        kind: TextWordKind::BasicLandType,
        from: 4,
        to: 1
    }));
    assert_eq!(map.land_type(land::FOREST), land::ISLAND);
    assert_eq!(
        map.keywords(KeywordSet::FORESTWALK.union(KeywordSet::FLYING)),
        KeywordSet::ISLANDWALK.union(KeywordSet::FLYING)
    );
    assert_eq!(map.color_word(Color::Green), Color::Green);
}

#[test]
fn type_lines_replace_simultaneously_and_keep_nonland_types() {
    let mut map = TextChangeMap::IDENTITY;
    assert!(map.replace(TextReplacement {
        kind: TextWordKind::BasicLandType,
        from: 1,
        to: 4
    }));
    let elf = baylee_core::generated::subtypes::creature::ELF;
    let types = SubtypeSet::from_slice(&[land::ISLAND, land::FOREST, elf]);
    assert_eq!(
        map.land_types(types),
        SubtypeSet::from_slice(&[land::FOREST, elf])
    );
    assert_eq!(map.land_type(elf), elf);
}

#[test]
fn choices_cover_every_distinct_pair_and_refuse_forged_answers() {
    for kind in [TextWordKind::Color, TextWordKind::BasicLandType] {
        let mut pairs = Vec::new();
        for choice in 0..20 {
            let replacement = TextReplacement::from_choice(kind, choice).expect("offered choice");
            assert!(replacement.is_valid());
            assert!(!pairs.contains(&replacement));
            pairs.push(replacement);
        }
        assert!(TextReplacement::from_choice(kind, 20).is_none());
        assert!(TextReplacement::from_choice(kind, u32::MAX).is_none());
        for (from, to) in [(0, 0), (4, 4), (5, 0), (0, 5), (u8::MAX, 1)] {
            let mut map = TextChangeMap::IDENTITY;
            assert!(!map.replace(TextReplacement { kind, from, to }));
            assert_eq!(map, TextChangeMap::IDENTITY);
        }
    }
}

#[test]
fn spell_entry_carries_changes_but_a_different_incarnation_does_not() {
    let spell = DamageSourceRef {
        object: ObjectId::new(4, 0),
        version: 1,
    };
    let permanent = DamageSourceRef {
        object: spell.object,
        version: 2,
    };
    let blinked = DamageSourceRef {
        object: spell.object,
        version: 3,
    };
    let mut changes = TextChanges::default();
    assert!(changes.replace(
        spell,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 0,
            to: 3
        }
    ));
    let snapshot = changes.get(spell);
    changes.carry(spell, permanent);
    assert_eq!(changes.get(spell), TextChangeMap::IDENTITY);
    assert_eq!(changes.get(permanent), snapshot);
    assert_eq!(changes.get(blinked), TextChangeMap::IDENTITY);
    assert!(changes.replace(
        permanent,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 3,
            to: 4
        }
    ));
    assert_eq!(
        snapshot.color_word(Color::White),
        Color::Red,
        "captured ability text does not follow later changes"
    );
    assert_eq!(
        changes.get(permanent).color_word(Color::White),
        Color::Green
    );
}
