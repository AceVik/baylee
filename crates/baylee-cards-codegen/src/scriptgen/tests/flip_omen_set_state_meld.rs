use super::*;

fn rich_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.push("Cyberman".into());
    c.planeswalker.push("Urza".into());
    c
}

fn read_rich(script: &str) -> CardBody {
    let parsed = parse(script);
    let c = rich_cats();
    let t = tokens();
    transcode(&parsed, &c, Some(&t)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, Some(&t))
        )
    })
}

#[test]
fn flip_card_lowers_both_faces_and_set_state_flip() {
    let body = read(
        "Name:Akki Proxy\nManaCost:3 R\nTypes:Creature Goblin Warrior\nPT:1/1\n\
         K:Haste\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Opponent | Execute$ TrigFlip\n\
         SVar:TrigFlip:DB$ SetState | Defined$ Self | Mode$ Flip\n\
         AlternateMode:Flip\n\
         ALTERNATE\n\
         Name:Tok-Tok Proxy\nManaCost:3 R\nColors:red\nTypes:Legendary Creature Goblin Shaman\nPT:2/2\n\
         K:Haste",
    );
    assert!(body.back_is_flip);
    assert!(!body.back_is_omen);
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("StateSubject::Source") && a.contains("StateTransition::Flip")),
        "{:?}",
        body.abilities
    );
    let back = body.back.as_deref().expect("flip back face");
    assert!(back.keywords.iter().any(|k| k == "KeywordSet::HASTE"));
}

#[test]
fn omen_card_lowers_omen_face_and_refuses_alchemy_seek() {
    let body = read(
        "Name:Storm Dragon\nManaCost:3 G G\nTypes:Creature Dragon\nPT:4/5\n\
         K:Flying\n\
         AlternateMode:Omen\n\
         ALTERNATE\n\
         Name:Roiling Omen\nManaCost:1 G\nTypes:Sorcery Omen\n\
         A:SP$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(body.back_is_omen);
    assert!(!body.back_is_adventure);
    let back = body.back.as_deref().expect("omen back face");
    assert!(back.abilities.iter().any(|a| a.contains("Effect::draw(1)")));

    // Omen alternate mode without the Omen subtype is refused.
    assert!(refused(
        "Name:Bad Omen\nManaCost:3 G\nTypes:Creature Dragon\nPT:3/3\nK:Flying\n\
         AlternateMode:Omen\n\
         ALTERNATE\n\
         Name:Not An Omen\nManaCost:G\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ 1"
    ));

    // Alchemy Omen card using Seek must stay refused.
    assert!(refused(
        "Name:Cunning Azurescale\nManaCost:2 U\nTypes:Creature Dragon\nPT:2/3\nK:Flying\n\
         AlternateMode:Omen\n\
         ALTERNATE\n\
         Name:Divining Dive\nManaCost:U\nTypes:Sorcery Omen\n\
         A:SP$ Seek | Defined$ You | Type$ Card.nonLand"
    ));
}

#[test]
fn set_state_modes_and_ixidron_etb_lower_cleanly_and_refuse_alchemy() {
    // Self TurnFaceDown (Obscuring Aether)
    let aether = read(
        "Name:Obscuring Proxy\nManaCost:G\nTypes:Enchantment\n\
         A:AB$ SetState | Cost$ 1 G | Defined$ Self | Mode$ TurnFaceDown",
    );
    assert!(
        aether
            .abilities
            .iter()
            .any(|a| a.contains("StateSubject::Source")
                && a.contains("StateTransition::TurnFaceDown(FaceDownSpec::DEFAULT)")),
        "{:?}",
        aether.abilities
    );

    // Targeted TurnFaceDown with morph filter (Backslide)
    let backslide = read(
        "Name:Backslide Proxy\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ SetState | ValidTgts$ Creature.withMorph,Creature.withMegamorph | Mode$ TurnFaceDown",
    );
    assert!(
        backslide.statics.contains("Filter::HasMorph"),
        "{}",
        backslide.statics
    );
    assert!(
        backslide
            .abilities
            .iter()
            .any(|a| a.contains("StateSubject::Targets")
                && a.contains("StateTransition::TurnFaceDown(FaceDownSpec::DEFAULT)")),
        "{:?}",
        backslide.abilities
    );

    // Custom FaceDownSpec (Cyber Conversion)
    let cyber = read_rich(
        "Name:Cyber Proxy\nManaCost:U U\nTypes:Instant\n\
         A:SP$ SetState | ValidTgts$ Creature | Mode$ TurnFaceDown | \
         FaceDownSetType$ Artifact & Creature & Cyberman | FaceDownPower$ 2 | FaceDownToughness$ 2",
    );
    assert!(
        cyber
            .abilities
            .iter()
            .any(|a| a.contains("TypeSet::ARTIFACT.union(TypeSet::CREATURE)")
                && a.contains("CYBERMAN")
                && a.contains("power: Some(2), toughness: Some(2)")),
        "{:?}",
        cyber.abilities
    );

    // RevealFirst + ValidNewFace optional TurnFaceUp (Hauntwoods Shrieker)
    let shrieker = read(
        "Name:Shrieker Proxy\nManaCost:1 G G\nTypes:Creature Beast\nPT:3/3\n\
         A:AB$ SetState | ValidTgts$ Card.faceDown | Cost$ 1 G | Mode$ TurnFaceUp | \
         Optional$ True | RevealFirst$ True | ValidNewFace$ Creature",
    );
    assert!(
        shrieker
            .abilities
            .iter()
            .any(|a| a.contains("StateTransition::RevealAndTurnFaceUpIf")
                && a.contains("optional: true")
                && !a.contains("Effect::MayDo")),
        "{:?}",
        shrieker.abilities
    );

    // Ixidron ETBReplacement:Other TurnFaceDown
    let ixidron = read(
        "Name:Ixidron Proxy\nManaCost:3 U U\nTypes:Creature Illusion\nPT:*/*\n\
         K:ETBReplacement:Other:TrigTurnFaceDown\n\
         SVar:TrigTurnFaceDown:DB$ SetState | Defined$ Valid Creature.!token+Other+faceUp | Mode$ TurnFaceDown",
    );
    assert!(
        ixidron
            .enter_modifiers
            .iter()
            .any(|m| m.starts_with("EnterModifier::TurnMatchingFaceDown(&")),
        "{:?}",
        ixidron.enter_modifiers
    );

    // Targeted non-self Transform with DoubleFaced / CanTransform filter atoms
    let moonmist = read(
        "Name:Transform Target Proxy\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ SetState | ValidTgts$ Creature.CanTransform+DoubleFaced | Mode$ Transform",
    );
    assert!(
        moonmist.statics.contains("Filter::CanTransform"),
        "{}",
        moonmist.statics
    );
    assert!(
        moonmist.statics.contains("Filter::DoubleFaced"),
        "{}",
        moonmist.statics
    );
    assert!(
        moonmist.abilities.iter().any(
            |a| a.contains("StateSubject::Targets") && a.contains("StateTransition::Transform")
        ),
        "{:?}",
        moonmist.abilities
    );

    // Defined$ Remembered lowers to StateSubject::Remembered (for CaptureAffected chains)
    let rem = read(
        "Name:Remembered Flip Proxy\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ SetState | Defined$ Remembered | Mode$ Flip",
    );
    assert!(
        rem.abilities
            .iter()
            .any(|a| a.contains("StateSubject::Remembered") && a.contains("StateTransition::Flip")),
        "{:?}",
        rem.abilities
    );

    // Alchemy Unspecialize stays strictly refused.
    assert!(refused(
        "Name:Alchemy Unspec\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ SetState | ValidTgts$ Creature | Mode$ Unspecialize"
    ));
}

#[test]
fn meld_pair_secondary_and_primary_cards_lower_cleanly() {
    // Secondary meld card (no ALTERNATE face)
    let secondary = read(
        "Name:Argoth Proxy\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ G\n\
         MeldPair:Titania Proxy\n\
         AlternateMode:Meld",
    );
    assert_eq!(secondary.meld_partner.as_deref(), Some("Titania Proxy"));
    assert!(secondary.back.is_none());

    // Primary meld card (with ALTERNATE melded TokenDef + Artifact partner type)
    let primary = read_rich(
        "Name:Urza Proxy\nManaCost:1 W U\nTypes:Legendary Creature Human Artificer\nPT:2/4\n\
         A:AB$ Meld | Cost$ 7 | ConditionPresent$ Card.Self+YouCtrl+YouOwn | ConditionCheckSVar$ X | \
         Name$ Urza Melded | Primary$ Urza Proxy | Secondary$ Mightstone Proxy | \
         SecondaryType$ Artifact | SorcerySpeed$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl+YouOwn+namedMightstone Proxy\n\
         MeldPair:Mightstone Proxy\n\
         AlternateMode:Meld\n\
         ALTERNATE\n\
         Name:Urza Melded\nManaCost:no cost\nTypes:Legendary Planeswalker Urza\nLoyalty:7\n\
         A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | NumCards$ 2",
    );
    assert_eq!(primary.meld_partner.as_deref(), Some("Mightstone Proxy"));
    assert!(primary.back.is_none());
    assert!(
        primary
            .statics
            .contains("static MELDED_FACE: TokenDef = TokenDef {"),
        "{}",
        primary.statics
    );
    assert!(
        primary.statics.contains("loyalty: Some(7),"),
        "{}",
        primary.statics
    );
    assert!(
        primary
            .abilities
            .iter()
            .any(|a| a.contains("Effect::MeldPair")
                && a.contains("partner_types: TypeSet::ARTIFACT")
                && a.contains("melded: &MELDED_FACE")),
        "{:?}",
        primary.abilities
    );

    // Melded back face with unsupported non-TokenDef CardBody fields (e.g. enter_modifiers)
    // must be refused rather than silently dropped.
    assert!(refused(
        "Name:Urza Bad Back\nManaCost:1 W U\nTypes:Legendary Creature Human Artificer\nPT:2/4\n\
         A:AB$ Meld | Cost$ 7 | ConditionPresent$ Card.Self+YouCtrl+YouOwn | ConditionCheckSVar$ X | \
         Name$ Urza Melded Bad | Primary$ Urza Bad Back | Secondary$ Mightstone Proxy | \
         SecondaryType$ Artifact | SorcerySpeed$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl+YouOwn+namedMightstone Proxy\n\
         MeldPair:Mightstone Proxy\n\
         AlternateMode:Meld\n\
         ALTERNATE\n\
         Name:Urza Melded Bad\nManaCost:no cost\nTypes:Legendary Creature Human\nPT:7/7\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBTapped\n\
         SVar:ETBTapped:DB$ Tap | Defined$ Self | ETB$ True"
    ));
}
