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

#[test]
fn wave23a_modified_filter_atoms_and_conditions() {
    // Positive & negated `modified` / `isModified` / `nonModified` atoms on statics.
    for atom in ["modified", "isModified", "IsModified"] {
        let out = read(&ability(&format!("Affected$ Creature.{atom}+YouCtrl")));
        assert!(
            out.abilities[0].contains("Filter::Modified"),
            "{atom}: {:?}",
            out.abilities
        );
    }
    for atom in ["!modified", "nonModified", "!isModified", "!IsModified"] {
        let out = read(&ability(&format!("Affected$ Creature.{atom}+YouCtrl")));
        assert!(
            out.abilities[0].contains("Filter::Not(&Filter::Modified)"),
            "{atom}: {:?}",
            out.abilities
        );
    }

    // Targeted spell + Aura enchant + dies trigger + `Count$Valid` with `modified`.
    let tgt = read(
        "Name:ModBolt\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.modified | NumDmg$ 3",
    );
    assert!(
        tgt.statics.contains("Filter::Modified"),
        "{} / {:?}",
        tgt.statics,
        tgt.abilities
    );

    let dies = read(
        "Name:ModScavenger\nManaCost:2 B\nTypes:Creature Rogue\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Creature.YouCtrl+modified+!token | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        dies.statics.contains("Filter::Modified") || dies.abilities[0].contains("Filter::Modified"),
        "{} / {:?}",
        dies.statics,
        dies.abilities
    );

    let count_valid = read(
        "Name:ModPulse\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.modified+YouCtrl",
    );
    assert!(
        count_valid.statics.contains("Filter::Modified")
            || count_valid.abilities[0].contains("Filter::Modified"),
        "{} / {:?}",
        count_valid.statics,
        count_valid.abilities
    );

    // `ConditionPresent$ Creature.YouCtrl+modified` canonicalizes to `YOUR_MODIFIED_CREATURE`.
    for present in [
        "Creature.YouCtrl+modified",
        "Creature.modified+YouCtrl",
        "Creature.YouCtrl+isModified",
    ] {
        let cond_spell = read(&format!(
            "Name:ModFlare\nManaCost:1 R\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ DBBonus\n\
             SVar:DBBonus:DB$ DealDamage | Defined$ TargetedController | NumDmg$ 2 | \
             ConditionPresent$ {present}"
        ));
        assert!(
            cond_spell.abilities[0]
                .contains("Condition::ControlCount(&Filter::YOUR_MODIFIED_CREATURE, 1)"),
            "{present}: {:?}",
            cond_spell.abilities
        );
    }
}
