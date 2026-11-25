use super::*;

fn wave28c_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Hero".into(),
        "Insect".into(),
        "Merfolk".into(),
        "Human".into(),
        "Faerie".into(),
    ]);
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
