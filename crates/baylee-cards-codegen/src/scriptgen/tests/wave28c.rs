use super::*;

fn wave28c_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Hero".into(),
        "Insect".into(),
        "Merfolk".into(),
        "Human".into(),
        "Faerie".into(),
        "Eldrazi".into(),
        "Spawn".into(),
        "Dinosaur".into(),
        "Zombie".into(),
        "Minotaur".into(),
        "Shaman".into(),
        "Dragon".into(),
    ]);
    c.artifact
        .extend(["Treasure".into(), "Blood".into(), "Food".into()]);
    c.spell.push("Lesson".into());
    c.normalize();
    c
}

fn read_w28c(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = wave28c_cats();
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

fn refused_w28c(text: &str) -> bool {
    let parsed = parse(text);
    let c = wave28c_cats();
    transcode(&parsed, &c, None).is_none()
}

#[test]
fn cast_with_flash_conditional_self_and_valid_sa_spell() {
    // Dragon Grip: Ferocious conditional self-Flash.
    let grip = read_w28c(
        "Name:Dragon Grip\nManaCost:2 R\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You | Condition$ Ferocious\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddKeyword$ First Strike | AddPower$ 2",
    );
    assert!(
        grip.abilities.iter().any(|a| a
            == "static_ability!(Filter::This, Modifier::GrantFlash { who: PlayerRel::You, filter: &Filter::This }, zone = ZoneRef::NotBattlefield, condition = Some(Condition::ControlCount(&Filter::YOUR_CREATURE_WITH_POWER_4_OR_GREATER, 1)))"),
        "expected self GrantFlash on NotBattlefield: {:?}",
        grip.abilities
    );

    // Hungering Yeti: IsPresent$ battlefield control condition.
    let yeti = read_w28c(
        "Name:Hungering Yeti\nManaCost:4 R\nTypes:Creature Yeti\nPT:4/4\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You | IsPresent$ Permanent.Green+YouCtrl,Permanent.Blue+YouCtrl",
    );
    assert!(
        yeti.abilities.iter().any(|a| a
            .contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &Filter::This }")
            && a.contains("zone = ZoneRef::NotBattlefield")
            && a.contains("condition = Some(")),
        "expected conditional self GrantFlash: {:?}",
        yeti.abilities
    );

    // Serpent of the Pass: PresentZone$ Graveyard condition.
    let serpent = read_w28c(
        "Name:Serpent of the Pass\nManaCost:5 U U\nTypes:Creature Serpent\nPT:6/5\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You | IsPresent$ Lesson.YouOwn | PresentZone$ Graveyard | PresentCompare$ GE3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$ValidGraveyard Card.nonCreature+nonLand+YouOwn",
    );
    assert!(
        serpent.abilities.iter().any(|a| a
            .contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &Filter::This }")
            && a.contains("zone = ZoneRef::NotBattlefield")
            && a.contains("ZoneSel::GraveyardOf(PlayerRel::You)")
            && a.contains(", 3)")),
        "expected graveyard-count self GrantFlash: {:?}",
        serpent.abilities
    );

    // Swift Reckoning: Spell mastery PresentZone$ Graveyard condition.
    let swift = read_w28c(
        "Name:Swift Reckoning\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.tapped\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You | IsPresent$ Instant.YouOwn,Sorcery.YouOwn | PresentZone$ Graveyard | PresentCompare$ GE2",
    );
    assert!(
        swift.abilities.iter().any(|a| a
            .contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &Filter::This }")
            && a.contains("zone = ZoneRef::NotBattlefield")
            && a.contains("ZoneSel::GraveyardOf(PlayerRel::You)")
            && a.contains(", 2)")),
        "expected spell mastery self GrantFlash: {:?}",
        swift.abilities
    );

    // Fated Clash: combined IsPresent$ + CheckSVar$.
    let fated = read_w28c(
        "Name:Fated Clash\nManaCost:3 W W\nTypes:Sorcery\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | IsPresent$ Creature.attacking | CheckSVar$ X | EffectZone$ All | Caster$ You\n\
         A:SP$ DestroyAll | ValidCards$ Creature\n\
         SVar:X:Count$Valid Creature.blocking",
    );
    assert!(
        fated.abilities.iter().any(|a| a
            .contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &Filter::This }")
            && a.contains("zone = ZoneRef::NotBattlefield")
            && a.contains("Condition::All(&[")),
        "expected combined condition self GrantFlash: {:?}",
        fated.abilities
    );

    // Blur of Heroism: ValidCard$ Card | ValidSA$ Spell.Hero.
    let blur = read_w28c(
        "Name:Blur of Heroism\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw\n\
         S:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell.Hero | Caster$ You",
    );
    assert!(
        blur.abilities
            .iter()
            .any(|a| a.contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &FLASH")),
        "expected Hero GrantFlash: {:?}",
        blur.abilities
    );
    assert!(
        blur.statics.contains("subtypes::creature::HERO"),
        "expected HERO subtype static: {}",
        blur.statics
    );
}

#[test]
fn panharmonicon_attack_triggers_isshin() {
    let isshin = read_w28c(
        "Name:Isshin, Two Heavens as One\nManaCost:R W B\nTypes:Legendary Creature Human Samurai\nPT:3/4\n\
         S:Mode$ Panharmonicon | ValidCard$ Permanent.YouCtrl | ValidMode$ Attacks,AttackersDeclared,AttackersDeclaredOneTarget | ValidCause$ Creature",
    );
    assert_eq!(
        isshin.abilities,
        vec![
            "AbilityDef::Replacement(ReplacementRule::TriggerMultiplier { source_filter: &Filter::ControlledByYou, event: TriggerEventKind::Attack })"
        ]
    );
}

#[test]
fn reduce_cost_second_spell_each_turn() {
    let bell = read_w28c(
        "Name:Highspire Bell-Ringer\nManaCost:2 U\nTypes:Creature Bird Monk\nPT:1/4\n\
         K:Flying\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1 | CheckSVar$ YouCastThisTurn | SVarCompare$ EQ1\n\
         SVar:YouCastThisTurn:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert!(
        bell.abilities.iter().any(|a| a.contains("Modifier::SpellsCostLess(1)")
            && a.contains("Condition::AmountCompare { left: &Amount::SpellsCastThisTurn(PlayerRel::You), op: AmountCmp::Equal, right: &Amount::Fixed(1) }")),
        "expected EQ1 SpellsCastThisTurn condition: {:?}",
        bell.abilities
    );

    // Canonical EQ0 must continue emitting Condition::YouCastNoSpellThisTurn.
    let first = read_w28c(
        "Name:First Spell Reducer\nManaCost:2 U\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1 | CheckSVar$ YouCastThisTurn | SVarCompare$ EQ0\n\
         SVar:YouCastThisTurn:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert!(
        first
            .abilities
            .iter()
            .any(|a| a.contains("condition = Some(Condition::YouCastNoSpellThisTurn)")),
        "expected canonical YouCastNoSpellThisTurn for EQ0: {:?}",
        first.abilities
    );
}

#[test]
fn raise_cost_valid_target_you_control() {
    // Esior, Wardwing Familiar: commander permanent you control.
    let esior = read_w28c(
        "Name:Esior, Wardwing Familiar\nManaCost:1 U\nTypes:Legendary Creature Bird\nPT:1/3\n\
         K:Flying\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ 3 | ValidTarget$ Card.IsCommander+YouCtrl+inZoneBattlefield",
    );
    assert!(
        esior.abilities.iter().any(|a| a
            .contains("Filter::TargetsYouControl(&Filter::IsCommander)")
            && a.contains("Modifier::SpellsCostMore(3)")),
        "expected TargetsYouControl(IsCommander): {:?}",
        esior.abilities
    );

    // Kasmina, Enigmatic Mentor: creature or planeswalker you control.
    let kasmina = read_w28c(
        "Name:Kasmina, Enigmatic Mentor\nManaCost:3 U\nTypes:Legendary Planeswalker Kasmina\nLoyalty:5\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ 2 | ValidTarget$ Creature.YouCtrl+inZoneBattlefield,Planeswalker.YouCtrl+inZoneBattlefield",
    );
    assert!(
        kasmina.abilities.iter().any(|a| a
            .contains("Filter::TargetsYouControl(&Filter::CREATURE_OR_PLANESWALKER)")
            && a.contains("Modifier::SpellsCostMore(2)")),
        "expected TargetsYouControl(Creature|Planeswalker): {:?}",
        kasmina.abilities
    );

    // Merfolk subtype you control (spell half of Kopala).
    let kopala_spell = read_w28c(
        "Name:Merfolk Ward\nManaCost:1 U U\nTypes:Creature Merfolk Wizard\nPT:2/2\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ 2 | ValidTarget$ Merfolk.YouCtrl+inZoneBattlefield",
    );
    assert!(
        kopala_spell
            .abilities
            .iter()
            .any(|a| a.contains("Filter::TargetsYouControl(&TARGET_YOU_CTRL")),
        "expected TargetsYouControl(Merfolk): {:?}",
        kopala_spell.abilities
    );
    assert!(
        kopala_spell.statics.contains("subtypes::creature::MERFOLK"),
        "expected MERFOLK static: {}",
        kopala_spell.statics
    );
}

#[test]
fn mana_restrict_cant_cast_spell_from_hand() {
    let dynamic_hand_lock = read_w28c(
        "Name:Offhand Researcher\nManaCost:3 U\nTypes:Legendary Creature Human Wizard\nPT:2/3\n\
         K:Vigilance\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ X | RestrictValid$ CantCastSpellFromHand\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        dynamic_hand_lock
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::NonSpellPaymentsOrMatchingSpells")),
        "expected NonSpellPaymentsOrMatchingSpells: {:?}",
        dynamic_hand_lock.abilities
    );
    assert!(
        dynamic_hand_lock.statics.contains("Filter::OwnedByYou")
            && dynamic_hand_lock
                .statics
                .contains("Filter::InZone(ZoneRef::Hand)")
            && dynamic_hand_lock
                .statics
                .contains("Filter::InZone(ZoneRef::Stack)")
            && dynamic_hand_lock.statics.contains("Filter::CastFromHand"),
        "expected exact hand-cast exclusion filter: {}",
        dynamic_hand_lock.statics
    );

    let karolina = read_w28c(
        "Name:Karolina Dean, Runaway\nManaCost:3 W\nTypes:Legendary Creature Alien Hero\nPT:3/4\n\
         K:Flying\n\
         A:AB$ Mana | Cost$ T | Produced$ W U B R G | RestrictValid$ CantCastSpellFromHand",
    );
    assert!(
        karolina
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::NonSpellPaymentsOrMatchingSpells")),
        "expected multi-mana NonSpellPaymentsOrMatchingSpells: {:?}",
        karolina.abilities
    );
}

#[test]
fn mixed_land_and_spell_play() {
    // Glarb, Calamity's Augur: library top lands + MV >= 4 spells.
    let glarb = read_w28c(
        "Name:Glarb, Calamity's Augur\nManaCost:B G U\nTypes:Legendary Creature Frog Wizard Noble\nPT:2/4\n\
         K:Deathtouch\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | Affected$ Land.TopLibrary+YouCtrl,Card.TopLibrary+nonLand+YouCtrl | ValidAfterStack$ Spell.cmcGE4 | AffectedZone$ Library | MayPlay$ True\n\
         A:AB$ Surveil | Cost$ T | Amount$ 2",
    );
    assert!(
        glarb
            .abilities
            .iter()
            .any(|a| a == "static_ability!(Filter::Any, Modifier::PlayLandsFromLibraryTop)"),
        "expected PlayLandsFromLibraryTop: {:?}",
        glarb.abilities
    );
    assert!(
        glarb
            .abilities
            .iter()
            .any(|a| a.contains("Filter::CmcAtLeast(4)")
                && a.contains("Modifier::MayCastWithAltCost")
                && a.contains("AltCastZones::TopOfLibrary")
                && a.contains("allow_lands: false")),
        "expected TopOfLibrary MV>=4 MayCastWithAltCost: {:?}",
        glarb.abilities
    );

    // Zask, Skittering Swarmlord: graveyard lands + Insect spells.
    let zask = read_w28c(
        "Name:Zask, Skittering Swarmlord\nManaCost:3 G G\nTypes:Legendary Creature Insect\nPT:5/5\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn,Insect.YouOwn | ValidAfterStack$ Spell.Insect | MayPlay$ True | AffectedZone$ Graveyard",
    );
    assert!(
        zask.abilities
            .iter()
            .any(|a| a == "static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard)"),
        "expected PlayLandsFromGraveyard: {:?}",
        zask.abilities
    );
    assert!(
        zask.abilities
            .iter()
            .any(|a| a.contains("subtypes::creature::INSECT")
                && a.contains("Filter::OwnedByYou")
                && a.contains("Modifier::MayCastWithAltCost")
                && a.contains("AltCastZones::Zones(&[ZoneRef::Graveyard])")
                && a.contains("allow_lands: false")),
        "expected graveyard Insect MayCastWithAltCost: {:?}",
        zask.abilities
    );

    // Glowcap Lantern: Equipment granting LookAtLibraryTop via AddStaticAbility$.
}

#[test]
fn class_and_equipment_library_statics() {
    // Glowcap Lantern: Equipment granting LookAtLibraryTop via AddStaticAbility$.
    let lantern = read_w28c(
        "Name:Glowcap Lantern\nManaCost:G\nTypes:Artifact Equipment\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | AddStaticAbility$ StaticLook\n\
         SVar:StaticLook:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         K:Equip:2",
    );
    assert!(
        lantern.abilities.iter().any(
            |a| a.contains("Modifier::GrantStatic") && a.contains("Modifier::LookAtLibraryTop")
        ),
        "expected GrantStatic(LookAtLibraryTop): {:?}",
        lantern.abilities
    );

    // Ranger Class Level 3: Class AddStaticAbility$ with AffectedZone$ Library.
    let ranger = read_w28c(
        "Name:Ranger Class\nManaCost:1 G\nTypes:Enchantment Class\n\
         K:Class:2:1 G:AddTrigger$ Trig\n\
         SVar:Trig:Mode$ AttackersDeclared | AttackingPlayer$ You | TriggerZones$ Battlefield | Execute$ Pump | Secondary$ True\n\
         SVar:Pump:DB$ PutCounter | ValidTgts$ Creature.attacking | CounterType$ P1P1 | CounterNum$ 1\n\
         K:Class:3:3 G:AddStaticAbility$ SMayLook & SMayPlay\n\
         SVar:SMayLook:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You | Secondary$ True\n\
         SVar:SMayPlay:Mode$ Continuous | Affected$ Creature.nonLand+TopLibrary+YouCtrl | AffectedZone$ Library | MayPlay$ True | Secondary$ True",
    );
    assert!(
        ranger
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::LookAtLibraryTop")
                && a.contains("Condition::ClassLevelAtLeast(3)")),
        "expected Class level 3 LookAtLibraryTop: {:?}",
        ranger.abilities
    );
    assert!(
        ranger
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::MayCastWithAltCost")
                && a.contains("Condition::ClassLevelAtLeast(3)")),
        "expected Class level 3 MayCastWithAltCost: {:?}",
        ranger.abilities
    );

    // Class level 2 conditional top-of-library play (Fortune Teller's Talent level 2 shape).
    let talent = read_w28c(
        "Name:Future Sight Class\nManaCost:U\nTypes:Enchantment Class\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         K:Class:2:3 U:AddStaticAbility$ SFutureSight\n\
         SVar:SFutureSight:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | CheckSVar$ X | AffectedZone$ Library | MayPlay$ True\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert!(
        talent
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::MayCastWithAltCost")
                && a.contains("Condition::ClassLevelAtLeast(2)")),
        "expected Class level 2 conditional MayCastWithAltCost: {:?}",
        talent.abilities
    );
}

#[test]
fn computed_pt_graveyard_greatest_power_and_exile_count() {
    // Carrion Grub: greatest power among creature cards in your graveyard.
    let grub = read_w28c(
        "Name:Carrion Grub\nManaCost:3 B\nTypes:Creature Insect\nPT:0/5\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn$GreatestCardPower\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | Defined$ You | NumCards$ 4",
    );
    assert!(
        grub.abilities
            .iter()
            .any(|a| a.contains("Modifier::ModifyPTByAmount")
                && a.contains("Amount::GreatestPowerAmong(&COUNT")),
        "expected ModifyPTByAmount with GreatestPowerAmong: {:?}",
        grub.abilities
    );
    assert!(
        grub.statics.contains("Filter::InZone(ZoneRef::Graveyard)")
            && grub.statics.contains("Filter::OwnedByYou"),
        "expected graveyard-scoped filter static: {}",
        grub.statics
    );

    // Unscoped / YouOwn ValidExile count.
    let exile_grower = read_w28c(
        "Name:Exile Watcher\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$ValidExile Card.YouOwn",
    );
    assert!(
        exile_grower.abilities.iter().any(|a| a
            .contains("Amount::CountOf { filter: &Filter::OwnedByYou, zone: ZoneSel::ExileAll }")),
        "expected ExileAll YouOwn count: {:?}",
        exile_grower.abilities
    );
}

#[test]
fn optional_cost_refusal_reason_reports_downstream_cause() {
    let parsed = parse(
        "Name:Optional Cost Probe\nManaCost:2 B\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<6> | EffectZone$ All\n\
         A:SP$ MysteryApi | ValidTgts$ Creature",
    );
    let reason = refusal_reason(&parsed, &cats(), None);
    assert_ne!(
        reason.as_deref(),
        Some("static ability `S: Mode$ OptionalCost`"),
        "supported OptionalCost must not mask the real downstream refusal: {reason:?}"
    );
}

#[test]
fn wave28c_refusal_guards() {
    // Wulfgar: Creature.YouCtrl attack Panharmonicon must stay refused (no cause_filter).
    assert!(refused_w28c(
        "Name:Wulfgar\nManaCost:3 R G\nTypes:Legendary Creature Human Barbarian\nPT:4/4\n\
         S:Mode$ Panharmonicon | ValidCard$ Permanent.YouCtrl | ValidMode$ Attacks,AttackersDeclared,AttackersDeclaredOneTarget | ValidCause$ Creature.YouCtrl"
    ));

    // Partial attack modes or conditional Panharmonicon must stay refused.
    assert!(refused_w28c(
        "Name:Partial Attack Doubler\nManaCost:3 W\nTypes:Enchantment\n\
         S:Mode$ Panharmonicon | ValidCard$ Permanent.YouCtrl | ValidMode$ Attacks | ValidCause$ Creature"
    ));
    assert!(refused_w28c(
        "Name:Conditional Attack Doubler\nManaCost:3 W\nTypes:Enchantment\n\
         S:Mode$ Panharmonicon | ValidCard$ Permanent.YouCtrl | ValidMode$ Attacks,AttackersDeclared,AttackersDeclaredOneTarget | ValidCause$ Creature | IsPresent$ Goblin.YouCtrl"
    ));
    assert!(refused_w28c(
        "Name:Panharmonicon\nManaCost:4\nTypes:Artifact\n\
         S:Mode$ Panharmonicon | ValidCard$ Permanent.YouCtrl | ValidMode$ ChangesZone | Destination$ Battlefield | ValidCause$ Artifact,Creature"
    ));

    // CastWithFlash targeting / X-cost / activated / unconditional self must stay refused.
    assert!(refused_w28c(
        "Name:Flash Photography\nManaCost:2 U U\nTypes:Sorcery\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell.IsTargeting Valid Permanent.YouCtrl | EffectZone$ All | Caster$ You\n\
         A:SP$ Draw"
    ));
    assert!(refused_w28c(
        "Name:Silver Scrutiny\nManaCost:X U U\nTypes:Sorcery\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell.XCostLE3 | EffectZone$ All | Caster$ You\n\
         SVar:X:Count$xPaid\nA:SP$ Draw | NumCards$ X"
    ));
    assert!(refused_w28c(
        "Name:Leonin Shikari\nManaCost:1 W\nTypes:Creature Cat Soldier\nPT:2/2\n\
         S:Mode$ CastWithFlash | ValidSA$ Activated.Equip | Caster$ You"
    ));
    assert!(refused_w28c(
        "Name:Teferi\nManaCost:2 U U\nTypes:Legendary Planeswalker Teferi\nLoyalty:3\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Activated.Loyalty | Caster$ You"
    ));
    assert!(refused_w28c(
        "Name:Uncond Self Flash\nManaCost:2 U\nTypes:Sorcery\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You\n\
         A:SP$ Draw"
    ));

    // RaiseCost non-YouCtrl targets, ability targets, and non-mana ability taxes stay refused.
    assert!(refused_w28c(
        "Name:Dragons Prey\nManaCost:2 B\nTypes:Instant\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | ValidTarget$ Dragon | EffectZone$ All\n\
         A:SP$ Destroy | ValidTgts$ Creature"
    ));
    assert!(refused_w28c(
        "Name:Kopala Ability Half\nManaCost:1 U U\nTypes:Legendary Creature Merfolk Wizard\nPT:2/2\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Ability | Amount$ 2 | ValidTarget$ Merfolk.YouCtrl+inZoneBattlefield"
    ));
    assert!(refused_w28c(
        "Name:Suppression Field\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidSpell$ Activated.!ManaAbility | Amount$ 2"
    ));

    // ReduceCost Bargain and Equip-target discounts stay refused.
    assert!(refused_w28c(
        "Name:Hamlet Glutton\nManaCost:5 G G\nTypes:Creature Giant\nPT:6/6\n\
         K:Bargain\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | ValidSpell$ Spell.Bargain | Amount$ 2 | EffectZone$ All"
    ));
    assert!(refused_w28c(
        "Name:Fervent Champion\nManaCost:R\nTypes:Creature Human Knight\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidSpell$ Activated.Equip | Activator$ You | ValidTarget$ Card.Self | Amount$ 3"
    ));

    // UntapOtherPlayer, CantAttack.Target$ You,Planeswalker, Propaganda Target$ You stay refused.
    assert!(refused_w28c(
        "Name:Seedborn Muse\nManaCost:3 G G\nTypes:Creature Spirit\nPT:2/4\n\
         S:Mode$ UntapOtherPlayer | ValidCard$ Permanent.YouCtrl"
    ));
    assert!(refused_w28c(
        "Name:Vow of Duty\nManaCost:2 W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CantAttack | ValidCard$ Creature.EnchantedBy | Target$ You,Planeswalker.YouCtrl"
    ));
    assert!(refused_w28c(
        "Name:Propaganda\nManaCost:2 U\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Cost$ 2 | Target$ You"
    ));

    // Yixlid Jailer, Wildsear Cascade, Serra Paragon shared-limit mixed play, OppOwn exile stay refused.
    assert!(refused_w28c(
        "Name:Yixlid Jailer\nManaCost:1 B\nTypes:Creature Zombie Wizard\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Card | AffectedZone$ Graveyard | RemoveAllAbilities$ True"
    ));
    assert!(refused_w28c(
        "Name:Wildsear\nManaCost:3 R G\nTypes:Legendary Creature Elemental Wolf\nPT:6/6\n\
         S:Mode$ Continuous | Affected$ Enchantment.YouCtrl | AffectedZone$ Stack | AddKeyword$ Cascade"
    ));
    assert!(refused_w28c(
        "Name:Serra Paragon\nManaCost:2 W W\nTypes:Creature Angel\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Permanent.YouOwn+cmcLE3,Land.YouOwn | ValidAfterStack$ Spell.cmcLE3 | Condition$ PlayerTurn | MayPlay$ True | MayPlayLimit$ 1 | EffectZone$ Battlefield | AffectedZone$ Graveyard"
    ));
    assert!(refused_w28c(
        "Name:Opp Exile Approximator\nManaCost:3 U B\nTypes:Creature Nightmare\nPT:1/1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$ValidExile Card.OppOwn"
    ));

    // Unmodeled Mana.RestrictValid forms stay refused.
    assert!(refused_w28c(
        "Name:Equip Mana\nManaCost:R\nTypes:Creature\nPT:1/1\n\
         A:AB$ Mana | Cost$ T | Produced$ R | RestrictValid$ Activated.Equip"
    ));
    assert!(refused_w28c(
        "Name:X Cost Mana\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 4 | RestrictValid$ Spell.CostContainsX"
    ));
}

#[test]
fn wave28c_p2_graveyard_self_alt_cost_and_combat_mana() {
    let raffines = read_w28c(
        "Name:Raffine's Guidance\nManaCost:W\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ 1 | AddToughness$ 1\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ Graveyard | Cost$ 2 W",
    );
    assert!(
        raffines.abilities.iter().any(|a| a
            == "static_ability!(Filter::This, Modifier::MayCastWithAltCost { \
                who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), \
                grant_zone_permission: true, allow_lands: false, with_flash: false, \
                max_mana_value: None, once_per_turn: false, \
                price: AltCastPrice::Mana(mana!(\"{2}{W}\")) }, zone = ZoneRef::Graveyard)"),
        "expected graveyard self MayCastWithAltCost: {:?}",
        raffines.abilities
    );

    let roku = read_w28c(
        "Name:Avatar Roku, Firebender\nManaCost:3 R R R\nTypes:Legendary Creature Human Avatar\nPT:6/6\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigMana\n\
         SVar:TrigMana:DB$ Mana | Produced$ R | Amount$ 6 | CombatMana$ True\n\
         A:AB$ Pump | Cost$ R R R | ValidTgts$ Creature | NumAtt$ +3",
    );
    assert!(
        roku.abilities
            .iter()
            .any(|a| a.contains("Effect::AddCombatMana { color: ManaColor::Red, amount: 6 }")),
        "expected AddCombatMana(Red, 6): {:?}",
        roku.abilities
    );
}

#[test]
fn wave28c_p2_multi_object_costs_up_to_15() {
    let hand_of_emrakul = read_w28c(
        "Name:Hand of Emrakul\nManaCost:9\nTypes:Creature Eldrazi\nPT:7/7\n\
         K:Annihilator:1\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Sac<4/Eldrazi.Spawn>",
    );
    assert_eq!(hand_of_emrakul.alternative_costs.len(), 1);
    assert_eq!(
        hand_of_emrakul.alternative_costs[0]
            .matches("Sacrifice(&COST")
            .count(),
        4,
        "expected four Sacrifice parts: {:?}",
        hand_of_emrakul.alternative_costs
    );

    let magda = read_w28c(
        "Name:Magda, Brazen Outlaw\nManaCost:1 R\nTypes:Legendary Creature Dwarf Berserker\nPT:2/1\n\
         A:AB$ ChangeZone | Cost$ Sac<5/Treasure> | Origin$ Library | Destination$ Battlefield | \
         ChangeType$ Card.Artifact,Card.Dragon | ChangeNum$ 1",
    );
    assert_eq!(
        magda.abilities[0].matches("Sacrifice(&COST").count(),
        5,
        "expected five Sacrifice parts: {:?}",
        magda.abilities
    );

    let bone_dragon = read_w28c(
        "Name:Bone Dragon\nManaCost:3 B B\nTypes:Creature Dragon Skeleton\nPT:5/4\n\
         K:Flying\n\
         A:AB$ ChangeZone | Cost$ 3 B B ExileFromGrave<7/Card.Other> | Origin$ Graveyard | \
         Destination$ Battlefield | ActivationZone$ Graveyard | Tapped$ True",
    );
    assert_eq!(
        bone_dragon.abilities[0]
            .matches("ExileFromGraveyard(&Filter::Another)")
            .count(),
        7,
        "expected seven ExileFromGraveyard parts: {:?}",
        bone_dragon.abilities
    );

    let citadel = read_w28c(
        "Name:Bolas's Citadel\nManaCost:3 B B B\nTypes:Legendary Artifact\n\
         A:AB$ LoseLife | Cost$ T Sac<10/Permanent.nonLand/nonland permanent> | \
         Defined$ Player.Opponent | LifeAmount$ 10",
    );
    assert_eq!(
        citadel.abilities[0].matches("Sacrifice(&COST").count(),
        10,
        "expected ten Sacrifice parts: {:?}",
        citadel.abilities
    );

    let glass_cast = read_w28c(
        "Name:Glass-Cast Heart\nManaCost:2 B\nTypes:Artifact\n\
         A:AB$ LoseLife | Cost$ B B T Sac<1/CARDNAME> Sac<13/Blood.token> | \
         Defined$ Player.Opponent | LifeAmount$ 13",
    );
    assert_eq!(
        glass_cast.abilities[0].matches("Sacrifice(&COST").count(),
        13,
        "expected thirteen Sacrifice parts: {:?}",
        glass_cast.abilities
    );
    assert!(glass_cast.abilities[0].contains("SacrificeSelf"));
}

#[test]
fn wave28c_p2_devotion_amounts() {
    let rebuff = read_w28c(
        "Name:Thassa's Rebuff\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ X\n\
         SVar:X:Count$Devotion.Blue",
    );
    assert!(
        rebuff.abilities[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Blue]) }"
        ),
        "expected blue devotion unless-cost: {:?}",
        rebuff.abilities
    );

    let aspect = read_w28c(
        "Name:Aspect of Hydra\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Devotion.Green",
    );
    assert!(
        aspect.abilities[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Green]) }"
        ),
        "expected green devotion pump: {:?}",
        aspect.abilities
    );

    let dual_devotion = read_w28c(
        "Name:Dual Devotion Probe\nManaCost:2 W B\nTypes:Sorcery\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$DevotionDual.White.Black",
    );
    assert!(
        dual_devotion.abilities[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::White, Color::Black]) }"
        ),
        "expected dual devotion gain life: {:?}",
        dual_devotion.abilities
    );

    let fanatic = read_w28c(
        "Name:Fanatic of Mogis\nManaCost:3 R\nTypes:Creature Minotaur Shaman\nPT:4/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ Player.Opponent | NumDmg$ X\n\
         SVar:X:Count$Devotion.Red",
    );
    assert!(
        fanatic.abilities[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Red]) }"
        ),
        "expected red devotion damage trigger: {:?}",
        fanatic.abilities
    );
}

#[test]
fn wave28c_p2_greatest_power_limit_min_and_graveyard_owner() {
    let wildspeaker = read_w28c(
        "Name:Return of the Wildspeaker\nManaCost:4 G\nTypes:Instant\n\
         A:SP$ Charm | Choices$ DBDraw,DBPumpAll\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl+nonHuman$GreatestCardPower\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Creature.YouCtrl+nonHuman | NumAtt$ +3 | NumDef$ +3",
    );
    assert!(
        wildspeaker.abilities[0].contains("Amount::GreatestPowerAmong(&"),
        "expected GreatestPowerAmong in charm draw: {:?}",
        wildspeaker.abilities
    );

    let chomp = read_w28c(
        "Name:Triumphant Chomp\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$Valid Dinosaur.YouCtrl$GreatestCardPower/LimitMin.2",
    );
    assert!(
        chomp.abilities[0].contains("Amount::IfCondition")
            && chomp.abilities[0].contains("Amount::GreatestPowerAmong(&")
            && chomp.abilities[0].contains("otherwise: &Amount::Fixed(2)"),
        "expected LimitMin.2 GreatestPowerAmong: {:?}",
        chomp.abilities
    );

    let prophet = read_w28c(
        "Name:Prophet of the Scarab\nManaCost:4 U\nTypes:Creature Zombie Wizard\nPT:3/4\n\
         K:Vigilance\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$Valid Zombie.YouCtrl/LimitMin.Y\n\
         SVar:Y:Count$ValidGraveyard Zombie.YouOwn",
    );
    assert!(
        prophet.abilities[0].contains("Amount::IfCondition")
            && prophet.abilities[0].contains("ZoneSel::Battlefield")
            && prophet.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)"),
        "expected LimitMin SVar comparison: {:?}",
        prophet.abilities
    );

    let mixed_case_ctr = read_w28c(
        "Name:Time Battery\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ X\n\
         SVar:X:Count$CardCounters.Time",
    );
    assert!(
        mixed_case_ctr.abilities[0].contains("Amount::CountersOnSource(CounterKind::Time)"),
        "expected mixed-case CounterKind::Time: {:?}",
        mixed_case_ctr.abilities
    );

    // Graveyard `YouCtrl` normalizes to `Filter::OwnedByYou` (CR 404.1 / 108.4a).
    let gy_mv = read_w28c(
        "Name:Graveyard MV Probe\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard Card.YouCtrl$GreatestCardManaCost",
    );
    assert!(
        gy_mv.statics.contains("Filter::OwnedByYou")
            && !gy_mv.statics.contains("Filter::ControlledByYou")
            && gy_mv.statics.contains("Filter::InZone(ZoneRef::Graveyard)"),
        "expected OwnedByYou + InZone(Graveyard): {}",
        gy_mv.statics
    );
}

#[test]
fn wave28c_p2_refusal_guards() {
    // Unknown Forge runtime feedback SVar (`AFCustomProbe:Number$0`) must never degrade to Fixed(0)
    // (whereas `AFLifeLost` is lowered by wave28b to `Amount::ThisWay(ThisWay::LifeLost)`).
    assert!(refused_w28c(
        "Name:Unknown AF Feedback Probe\nManaCost:3 B B\nTypes:Creature Zombie\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ AFCustomProbe\n\
         SVar:AFCustomProbe:Number$0\n\
         SVar:X:Count$Devotion.Black"
    ));

    // Graveyard greatest-power/MV with OppOwn or OppCtrl stays refused (no OwnedByOpponent).
    assert!(refused_w28c(
        "Name:Opp Graveyard MV\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard Card.OppOwn$GreatestCardManaCost"
    ));
    // Non-mana graveyard AlternativeCost and Count$AltCost riders stay refused.
    assert!(refused_w28c(
        "Name:Scourge of Nel Toth\nManaCost:5 B B\nTypes:Creature Zombie Dragon\nPT:6/6\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ Graveyard | Cost$ B B Sac<2/Creature>"
    ));
    assert!(refused_w28c(
        "Name:Worldheart Phoenix\nManaCost:3 R\nTypes:Creature Phoenix\nPT:2/2\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ Graveyard | Cost$ W U B R G\n\
         SVar:AltCostPaid:Count$AltCost.1.0"
    ));
    assert!(refused_w28c(
        "Name:X Graveyard Alt\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ Graveyard | Cost$ X U"
    ));

    // CombatMana$ True with dynamic amount, Any color, or SpendRestriction stays refused.
    assert!(refused_w28c(
        "Name:Dynamic Combat Mana\nManaCost:3 R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigMana\n\
         SVar:TrigMana:DB$ Mana | Produced$ R | Amount$ X | CombatMana$ True\n\
         SVar:X:Count$CardPower"
    ));
    assert!(refused_w28c(
        "Name:Any Combat Mana\nManaCost:3 R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigMana\n\
         SVar:TrigMana:DB$ Mana | Produced$ Any | Amount$ 2 | CombatMana$ True"
    ));

    // Chosen devotion, LimitMin.0, and >15 object costs stay refused.
    assert!(refused_w28c(
        "Name:Chosen Devotion\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ X\nSVar:X:Count$Devotion.Chosen"
    ));
    assert!(refused_w28c(
        "Name:LimitMin Zero\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$YourLifeTotal/LimitMin.0"
    ));
    assert!(refused_w28c(
        "Name:Too Many Sacrifices\nManaCost:2 B\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ Sac<16/Treasure> | NumCards$ 1"
    ));
    // Multi-object UnlessCost stays refused (PlayerMayPayCostOr holds one CostPart).
    assert!(refused_w28c(
        "Name:Jokulmorder\nManaCost:4 U U U\nTypes:Creature Leviathan\nPT:12/12\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSacUnless\n\
         SVar:TrigSacUnless:DB$ Sacrifice | UnlessCost$ Sac<5/Land> | UnlessPayer$ You"
    ));
}
