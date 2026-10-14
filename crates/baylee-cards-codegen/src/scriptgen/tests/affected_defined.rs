//! The upstream split of an object definition and its validity restriction.
use super::*;

fn ability(selection: &str) -> String {
    format!(
        "Name:Test\nTypes:Enchantment\nS:Mode$ Continuous | {selection} | AddPower$ -2 | AddToughness$ -1"
    )
}

#[test]
fn defined_objects_preserve_legacy_filters_and_modifiers() {
    for (defined, relation) in [
        ("Self", "Self"),
        ("Enchanted", "EnchantedBy"),
        ("Equipped", "EquippedBy"),
        ("AttachedBy Self", "AttachedBy"),
    ] {
        for base in [
            "Card",
            "Creature",
            "Land",
            "Creature.YouCtrl",
            "Creature,Artifact",
        ] {
            let old = base
                .split(',')
                .map(|part| {
                    let (base, tail) = part.split_once('.').unwrap_or((part, ""));
                    if tail.is_empty() {
                        format!("{base}.{relation}")
                    } else {
                        format!("{base}.{relation}+{tail}")
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let legacy = read(&ability(&format!("Affected$ {old}")));
            let modern = read(&ability(&format!(
                "AffectedDefined$ {defined} | Affected$ {base}"
            )));
            assert_eq!(legacy.abilities, modern.abilities, "{defined}: {base}");
            assert_eq!(modern.abilities.len(), 1);
        }
        assert_eq!(
            read(&ability(&format!("AffectedDefined$ {defined}"))).abilities,
            read(&ability(&format!("Affected$ Card.{relation}"))).abilities,
        );
    }
    assert!(
        read(&ability("AffectedDefined$ Enchanted | Affected$ Creature")).abilities[0]
            .contains("Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])")
    );
}

#[test]
fn unsupported_definitions_and_validity_never_widen_the_selection() {
    for defined in [
        "EffectSource",
        "AttachedBy Remembered",
        "Remembered",
        "Unknown",
    ] {
        let script = ability(&format!("AffectedDefined$ {defined} | Affected$ Creature"));
        assert!(refused(&script), "{defined}");
        assert!(format!("{:?}", refusal_reason(&parse(&script), &cats(), None)).contains(defined));
    }
    for extra in [
        " | Affected$ Player",
        " | Affected$ Creature.Unknown",
        " | Unexpected$ True",
    ] {
        assert!(refused(&ability(&format!("AffectedDefined$ Self{extra}"))));
    }
}
