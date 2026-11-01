use super::*;

mod affected_defined;
mod animation_rules;
mod archon_kismet;
mod blocks_and_conditions;
mod costs_and_refusals;
mod divided_damage;
mod effects;
mod imprison;
mod prevention_rules;
mod reading;
mod static_supertypes;
mod tokens_and_zones;

fn cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec!["Goblin".into(), "Wizard".into()],
        // The Clue is here because a token's own type line is read
        // against this catalog, and investigating reaches one.
        artifact: vec!["Clue".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats(), None)
        )
    })
}

fn refused(text: &str) -> bool {
    transcode(&parse(text), &cats(), None).is_none()
}

/// The three token scripts the `Token` tests below read against, in the
/// reference's own shape. Held rather than read off disk: the corpus is
/// not vendored, so a test that needed a checkout is a test CI skips.
fn tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "r_1_1_goblin",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
        ),
        (
            "u_1_1_wizard_flying",
            "Name:Wizard\nTypes:Creature Wizard\nColors:blue\nPT:1/1\nK:Flying",
        ),
        // A token that carries an ability, which is what 190 of the
        // reference's 852 token scripts do: the definition names it, and
        // the constant does not — two Goblins that differ only there are
        // one name.
        (
            "r_1_1_goblin_sac",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
        ),
        // And one whose ability makes a token, which is the shape
        // [`crate::tokengen`] refuses: it hands the transcoder no
        // lookup, so a token cannot read another token into existence.
        (
            "r_1_1_goblin_maker",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1\n\
             A:AB$ Token | Cost$ T | TokenScript$ r_1_1_goblin | TokenOwner$ You",
        ),
        // The Clue, in the reference's own shape, because CR 701.16a
        // makes it the whole definition of investigating and the rule
        // reaches it through this same lookup.
        (
            CLUE_TOKEN_SCRIPT,
            "Name:Clue Token\nTypes:Artifact Clue\n\
             A:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1",
        ),
    ])
}

fn read_with_tokens(text: &str) -> CardBody {
    transcode(&parse(text), &cats(), Some(&tokens())).expect("should be read in full")
}

fn refused_with_tokens(text: &str) -> bool {
    transcode(&parse(text), &cats(), Some(&tokens())).is_none()
}

fn filter(valid: &str) -> String {
    let svars = BTreeMap::new();
    let cats = cats();
    let tx = Tx {
        svars: &svars,
        oracle: "",
        cats: &cats,
        tokens: None,
        has_x: false,
        on_a_spell: false,
        trigger_mode: None,
        block_line: None,
        block_half: None,
        in_delayed: false,
        cost_sacrifices_self: false,
        cost_sacrifices_other: false,
        trigger_is_self: false,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    tx.filter_expr(valid).expect("the valid-string is read")
}

#[test]
fn song_reader_requires_whole_immediate_continuation_and_layer_cohort() {
    let active = "S:Mode$ Continuous | Affected$ Artifact.nonCreature | SetPower$ AffectedX | SetToughness$ AffectedX | AddType$ Artifact & Creature | RemoveCardTypes$ True | RemoveAllAbilities$ True";
    let script = format!(
        "Name:X\nTypes:Enchantment\n{active}\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ Continue | Static$ True\nSVar:Continue:DB$ Effect | StaticAbilities$ Animate\nSVar:Animate:Mode$ Continuous | AffectedZone$ Battlefield | Affected$ Artifact.nonCreature | SetPower$ AffectedX | SetToughness$ AffectedX | AddType$ Creature | RemoveAllAbilities$ True\nSVar:AffectedX:Count$CardManaCost"
    );
    let body = read(&script);
    assert_eq!(body.abilities.len(), 1);
    assert!(
        body.abilities[0]
            .contains("AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture")
    );
    assert!(!body.abilities[0].contains("triggered!"));
    assert!(
        refused(&format!(
            "Name:X\nTypes:Enchantment\n{active}\nSVar:AffectedX:Count$CardManaCost"
        )),
        "missing continuation may not claim half card"
    );
    for (from, to) in [
        ("Static$ True", "Static$ False"),
        ("Destination$ Any", "Destination$ Graveyard"),
        ("Count$CardManaCost", "Count$CardPower"),
        ("RemoveAllAbilities$ True", "RemoveAllAbilities$ False"),
        ("AffectedZone$ Battlefield", "AffectedZone$ Graveyard"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
    assert!(refused(&script.replace(
        "DB$ Effect | StaticAbilities$ Animate",
        "DB$ Effect | StaticAbilities$ Animate | Duration$ Permanent"
    )));
}
