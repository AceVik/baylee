//! Wave 16b `scriptgen` tests:
//! 1. `A:SP$ PermanentCreature` & `A:SP$ PermanentNoncreature` mandatory spell costs
//!    (*Abhorrent Oculus*, *Dargo, the Shipwrecker*, *Fear of Abduction*, *Gorex*, *Bridge from Below*).
//! 2. `Blight<N>` costs & `DB$ Blight` APNAP effects (*Cinder Strike*, *Pyrrhic Strike*,
//!    *Bogslither's Embrace*, *Dawnhand Dissident*, *Evershrike's Gift*, *Gutsplitter Gang*,
//!    *High Perfect Morcant*).
//! 3. `Reveal<N/Valid>`, `Reveal<X/Valid>`, `Reveal<1/CARDNAME>` (`Forecast`),
//!    `RevealOrChoose<1/Valid>`, `RevealedCards$CardPower`, `Revealed$CardManaCost`,
//!    and Tarkir `Count$Presence_Dragon.1.0` (*Silvergill Adept*, *Dragon's Eye Savants*,
//!    *Martyr of Sands*, *Priest of the Wakening Sun*, *Spirit en-Dal*, *Dragon's Fire*,
//!    *Disaster Radius*, *Draconic Roar*, *Dragonlord's Prerogative*, *Orator of Ojutai*,
//!    *Scaleguard Sentinels*, *Silumgar's Scorn*).
//! 4. `S: Mode$ OptionalCost` extensions: `Waterbend<N>` (*Katara, Seeking Revenge*),
//!    `SpellCastMetric::OptionalCostPaid` (*Bite Down on Crime*), and entwine charm
//!    (*Pyrrhic Strike*).
//! 5. Graveyard target & static grants (*Archmage's Newt*, *Confession Dial*,
//!    *Cursecloth Wrappings*, *Filigree Racer*, *The Fugitive Doctor*).

use super::*;

fn w16b_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Dragon".into(),
            "Dinosaur".into(),
            "Elf".into(),
            "Elemental".into(),
            "Giant".into(),
            "Goblin".into(),
            "Kithkin".into(),
            "Merfolk".into(),
            "Pirate".into(),
            "Vampire".into(),
            "Wizard".into(),
        ],
        artifact: vec!["Clue".into()],
        spell: vec!["Lesson".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w16b(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w16b_cats();
    let toks = tokens();
    transcode(&parsed, &cats, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, Some(&toks))
        )
    })
}

fn refused_w16b(text: &str) -> bool {
    let cats = w16b_cats();
    let toks = tokens();
    transcode(&parse(text), &cats, Some(&toks)).is_none()
}

#[test]
fn wave16b_permanent_spells_and_mandatory_costs() {
    // Abhorrent Oculus shape: `ExileFromGrave<6/Card>` on `PermanentCreature`.
    let oculus = read_w16b(
        "Name:EyeFromDeep\nManaCost:2 U\nTypes:Creature\nPT:5/5\n\
         A:SP$ PermanentCreature | Cost$ 2 U ExileFromGrave<6/Card>\n\
         K:Flying",
    );
    assert_eq!(oculus.mandatory_additional_costs.len(), 6);
    assert!(
        oculus
            .mandatory_additional_costs
            .iter()
            .all(|c| c.starts_with("CostPart::ExileFromGraveyard("))
    );

    // Dargo shape: `Sac<X/Artifact;Creature>` + `SpellCastMetric::XPaid` + `PermanentsSacrificedThisTurn`.
    let dargo = read_w16b(
        "Name:Shipwrecker\nManaCost:6 R\nTypes:Legendary Creature Giant Pirate\nPT:7/5\n\
         A:SP$ PermanentCreature | Cost$ 6 R Sac<X/Artifact;Creature/any number> | AILogic$ SacToReduceCost | AdditionalDesc$ desc\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Y | EffectZone$ All | Relative$ True\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Z | EffectZone$ All\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:SVar$X/Times.2\n\
         SVar:Z:PlayerCountPropertyYou$SacrificedThisTurn Artifact,Creature/Times.2\n\
         K:Trample",
    );
    assert_eq!(
        dargo.mandatory_additional_costs,
        vec!["CostPart::SacrificeX(&COST1)"]
    );
    assert!(
        dargo
            .abilities
            .iter()
            .any(|a| a.contains("SpellCastMetric::XPaid") && a.contains("delta_per_unit: -2"))
    );

    // Fear of Abduction shape: `Exile<1/Creature>` + `Card.ExiledWithSource` LTB return.
    let abduction = read_w16b(
        "Name:Abductor\nManaCost:4 W W\nTypes:Enchantment Creature\nPT:5/5\n\
         A:SP$ PermanentCreature | Cost$ 4 W W Exile<1/Creature>\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZoneAll | ChangeType$ Card.ExiledWithSource | Origin$ Exile | Destination$ Hand",
    );
    assert_eq!(
        abduction.mandatory_additional_costs,
        vec!["CostPart::ExilePermanent(&Filter::YOUR_CREATURE)"]
    );

    // PermanentNoncreature on Enchantment (Bridge-like shape).
    let bridge = read_w16b(
        "Name:GraveBridge\nManaCost:B B B\nTypes:Enchantment\n\
         A:SP$ PermanentNoncreature | Cost$ B B B | AILogic$ Never\n\
         K:Shroud",
    );
    assert!(bridge.mandatory_additional_costs.is_empty());
    assert!(bridge.keywords.contains(&"KeywordSet::SHROUD".to_string()));

    // Refusals: type mismatch, land, instant, or unclaimed parameter.
    for bad in [
        "Name:BadNonCreature\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:SP$ PermanentNoncreature | Cost$ 1 G\nK:Trample",
        "Name:BadCreature\nManaCost:1 G\nTypes:Enchantment\nA:SP$ PermanentCreature | Cost$ 1 G\nK:Shroud",
        "Name:BadLand\nTypes:Land\nA:SP$ PermanentNoncreature | Cost$ 0",
        "Name:BadInstant\nManaCost:G\nTypes:Instant\nA:SP$ PermanentNoncreature | Cost$ G",
        "Name:BadExtra\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:SP$ PermanentCreature | Cost$ 1 G | SubAbility$ DBDraw\nSVar:DBDraw:DB$ Draw",
    ] {
        assert!(refused_w16b(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave16b_blight_costs_and_effects_and_optional_cost_extensions() {
    // Cinder Strike: `OptionalCost | Cost$ Blight<1>` + `Count$OptionalGenericCostPaid.4.2`.
    let cinder = read_w16b(
        "Name:CinderProbe\nManaCost:R\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Blight<1>\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$OptionalGenericCostPaid.4.2",
    );
    assert_eq!(
        cinder.optional_cost.as_deref(),
        Some(
            "Some(cost!(PutCounterOnOther { filter: &Filter::YOUR_CREATURE, kind: CounterKind::M1M1, n: 1 }))"
        )
    );
    assert!(cinder.abilities[0].contains("Condition::OptionalCostPaid"));

    // Pyrrhic Strike: `OptionalCost | Cost$ Blight<2>` + `CharmNum$ X` (`2.1`) -> `entwine`!
    let pyrrhic = read_w16b(
        "Name:PyrrhicProbe\nManaCost:2 W\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Blight<2>\n\
         A:SP$ Charm | CharmNum$ X | Choices$ DBDestroy1,DBDestroy2 | AdditionalDescription$ . If this spell's additional cost was paid, choose both instead.\n\
         SVar:DBDestroy1:DB$ Destroy | ValidTgts$ Artifact,Enchantment\n\
         SVar:DBDestroy2:DB$ Destroy | ValidTgts$ Creature.cmcGE3\n\
         SVar:X:Count$OptionalGenericCostPaid.2.1",
    );
    assert!(pyrrhic.optional_cost.is_none());
    assert!(
        pyrrhic
            .entwine
            .as_deref()
            .is_some_and(|e| e.contains("PutCounterOnOther") && e.contains("n: 2"))
    );
    assert!(pyrrhic.abilities[0].contains("ModeCount::ONE"));

    // Bite Down on Crime: `CollectEvidence<6>` + `ReduceCost | Amount$ Count$OptionalGenericCostPaid.2.0`.
    let bite = read_w16b(
        "Name:CrimeBite\nManaCost:3 G\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ CollectEvidence<6>\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Z | EffectZone$ All | Relative$ True\n\
         SVar:Z:Count$OptionalGenericCostPaid.2.0\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert!(bite.abilities.iter().any(|a| {
        a.contains("SpellCastMetric::OptionalCostPaid") && a.contains("delta_per_unit: -2")
    }));

    // Katara: `OptionalCost | Cost$ Waterbend<2>` + `NumCards$ CastSA>Count$OptionalGenericCostPaid.0.1`.
    let katara = read_w16b(
        "Name:Waterbender\nManaCost:3 UB\nTypes:Legendary Creature\nPT:3/3\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Waterbend<2>\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1 | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ You | NumCards$ X | Mode$ TgtChoose\n\
         SVar:X:CastSA>Count$OptionalGenericCostPaid.0.1",
    );
    assert!(katara.waterbend);
    assert_eq!(
        katara.optional_cost.as_deref(),
        Some("Some(cost!(\"{2}\"))")
    );
    assert!(
        katara.abilities[0].contains(
            "then: &[], otherwise: &[Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }]"
        ),
        "{}",
        katara.abilities[0]
    );

    // Bogslither's Embrace (`AlternateAdditionalCost:Blight<1>:3`), Evershrike's Gift (graveyard `Blight<2>`),
    // and High Perfect Morcant (`DB$ Blight | Defined$ Opponent | Num$ 1`).
    let bog = read_w16b(
        "Name:BogEmbrace\nManaCost:1 B\nTypes:Sorcery\n\
         K:AlternateAdditionalCost:Blight<1>:3\n\
         A:SP$ Destroy | ValidTgts$ Creature",
    );
    assert!(bog.mandatory_additional_costs[0].starts_with("CostPart::Either("));

    let evershrike = read_w16b(
        "Name:GraveBlight\nManaCost:W\nTypes:Enchantment\n\
         A:AB$ ChangeZone | Cost$ 1 W Blight<2> | Origin$ Graveyard | Destination$ Hand | ActivationZone$ Graveyard | SorcerySpeed$ True",
    );
    assert!(evershrike.abilities[0].contains("ActivationZone::Graveyard"));
    assert!(evershrike.abilities[0].contains("PutCounterOnOther"));

    let morcant = read_w16b(
        "Name:MorcantProbe\nManaCost:2 B G\nTypes:Legendary Creature Elf\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self,Elf.Other+YouCtrl | Execute$ TrigBlight\n\
         SVar:TrigBlight:DB$ Blight | Defined$ Opponent | Num$ 1",
    );
    assert!(morcant.abilities[0].contains(
        "Effect::PutCountersOnChosenControlled { who: PlayerRel::EachOpponent, filter: &Filter::CREATURE, kind: CounterKind::M1M1, count: Amount::Fixed(1) }"
    ));

    // Refuse `Blight<0>` and `Blight<X>` without engine X-blight support.
    assert!(refused_w16b(
        "Name:BadBlight0\nManaCost:B\nTypes:Sorcery\nA:AB$ Draw | Cost$ Blight<0> | NumCards$ 1"
    ));
    assert!(refused_w16b(
        "Name:BadBlightX\nManaCost:X B\nTypes:Sorcery\nA:SP$ Draw | Cost$ X B Blight<X> | NumCards$ 1\nSVar:X:Count$xPaid"
    ));
}

#[test]
fn wave16b_reveal_costs_forecast_beheld_stats_and_dragon_presence() {
    // Silvergill Adept (`AlternateAdditionalCost:Reveal<1/Merfolk>:3`).
    let silvergill = read_w16b(
        "Name:AdeptProbe\nManaCost:1 U\nTypes:Creature Merfolk Wizard\nPT:2/1\n\
         K:AlternateAdditionalCost:Reveal<1/Merfolk>:3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(
        silvergill.mandatory_additional_costs,
        vec!["CostPart::Either(&cost!(RevealFromHand(&COST1)), &cost!(\"{3}\"))"]
    );

    // Dragon's Eye Savants (`K:Morph:Reveal<1/Card.Blue/blue>`).
    let savants = read_w16b(
        "Name:SavantsProbe\nManaCost:1 U\nTypes:Creature Wizard\nPT:0/6\n\
         K:Morph:Reveal<1/Card.Blue/blue>",
    );
    assert_eq!(savants.morph.as_deref(), Some("Some(ManaCost::ZERO)"));
    assert_eq!(savants.morph_parts, vec!["RevealFromHand(&COST1)"]);

    // Martyr of Sands (`Reveal<X/Card.White>`).
    let martyr = read_w16b(
        "Name:MartyrProbe\nManaCost:W\nTypes:Creature\nPT:1/1\n\
         A:AB$ GainLife | Cost$ 1 Reveal<X/Card.White> Sac<1/CARDNAME> | LifeAmount$ Y\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:SVar$X/Times.3\n\
         Oracle:{1}, Reveal X white cards from your hand, Sacrifice MartyrProbe: You gain three times X life.",
    );
    assert!(martyr.abilities[0].contains("RevealFromHandX(&COST1)"));

    // Forecast (`Cost$ 1 W Reveal<1/CARDNAME> | Forecast$ True`).
    let forecast = read_w16b(
        "Name:ForecastProbe\nManaCost:2 W\nTypes:Creature\nPT:2/1\n\
         K:Shadow\n\
         A:AB$ Pump | Cost$ 1 W Reveal<1/CARDNAME> | ValidTgts$ Creature | KW$ Shadow | Forecast$ True",
    );
    assert!(forecast.abilities[0].contains("RevealSelfFromHand"));
    assert!(forecast.abilities[0].contains("ActivationZone::Hand"));
    assert!(forecast.abilities[0].contains("ActivationLimit::PerTurn(1)"));
    assert!(forecast.abilities[0].contains("StepKind::Upkeep"));

    // Dragon's Fire (`RevealOrChoose<1/Dragon>` + `RevealedCards$CardPower`) &
    // Disaster Radius (`Reveal<1/Creature>` + `Revealed$CardManaCost`).
    let dfire = read_w16b(
        "Name:DragonsFireProbe\nManaCost:1 R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ RevealOrChoose<1/Dragon>\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ Y\n\
         SVar:Y:Count$OptionalGenericCostPaid.X.3\n\
         SVar:X:RevealedCards$CardPower",
    );
    assert_eq!(
        dfire.optional_cost.as_deref(),
        Some("Some(cost!(Behold(&COST1)))")
    );
    assert!(dfire.abilities[0].contains("Amount::BeheldOrRevealedPower"));

    let disaster = read_w16b(
        "Name:DisasterProbe\nManaCost:5 R R\nTypes:Sorcery\n\
         A:SP$ DamageAll | Cost$ 5 R R Reveal<1/Creature> | NumDmg$ X | ValidCards$ Creature.OppCtrl\n\
         SVar:X:Revealed$CardManaCost",
    );
    assert_eq!(
        disaster.mandatory_additional_costs,
        vec!["CostPart::RevealFromHand(&Filter::CREATURE)"]
    );
    assert!(disaster.abilities[0].contains("Amount::BeheldOrRevealedManaValue"));
}

#[test]
fn wave16b_tarkir_dragon_presence_and_reveal_refusals() {
    // Tarkir `Count$Presence_Dragon.1.0` across spell SubAbility, R:Event$ Counter, T:ETB, and K:etbCounter.
    let roar = read_w16b(
        "Name:RoarProbe\nManaCost:1 R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Reveal<1/Dragon>\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ ParentTargetedController | NumDmg$ 3 | ConditionCheckSVar$ Count$Presence_Dragon.1.0",
    );
    assert!(
        roar.abilities[0]
            .contains("Condition::ControlledSubtypeOnCast(subtypes::creature::DRAGON)")
    );

    let prerogative = read_w16b(
        "Name:PrerogativeProbe\nManaCost:4 U U\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Reveal<1/Dragon>\n\
         R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen | CheckSVar$ Count$Presence_Dragon.1.0\n\
         A:SP$ Draw | NumCards$ 4",
    );
    assert!(
        prerogative
            .abilities
            .iter()
            .any(|a| a.contains("KeywordSet::UNCOUNTERABLE") && a.contains("ZoneRef::Stack"))
    );

    let orator = read_w16b(
        "Name:OratorProbe\nManaCost:1 W\nTypes:Creature\nPT:0/4\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Reveal<1/Dragon>\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | CheckSVar$ Count$Presence_Dragon.1.0 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        orator.abilities[0]
            .contains("Condition::ControlledSubtypeOnCast(subtypes::creature::DRAGON)")
    );

    let sentinels = read_w16b(
        "Name:SentinelsProbe\nManaCost:G G\nTypes:Creature\nPT:2/3\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Reveal<1/Dragon>\n\
         K:etbCounter:P1P1:1:CheckSVar$ Count$Presence_Dragon.1.0:desc",
    );
    assert!(
        sentinels.enter_modifiers[0]
            .contains("Condition::ControlledSubtypeOnCast(subtypes::creature::DRAGON)")
    );

    // Refusals: Illuminated Folio `SameColor`, Forecast without `Reveal<1/CARDNAME>`.
    assert!(refused_w16b(
        "Name:Folio\nManaCost:5\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 T Reveal<2/SameColor> | NumCards$ 1"
    ));
    assert!(refused_w16b(
        "Name:BadForecast\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nA:AB$ Draw | Cost$ 1 U | NumCards$ 1 | Forecast$ True"
    ));
}

#[test]
fn wave16b_graveyard_target_and_static_grants() {
    // Archmage's Newt: `DB$ Branch` over `TgtZone$ Graveyard` granting `Flashback:0` vs `Flashback`.
    let newt = read_w16b(
        "Name:NewtProbe\nManaCost:1 U\nTypes:Creature\nPT:2/2\n\
         K:Saddle:3\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TgtZone$ Graveyard | BranchConditionSVar$ Count$Valid Card.Self+IsSaddled | TrueSubAbility$ FBSaddled | FalseSubAbility$ FBUnsaddled\n\
         SVar:FBSaddled:DB$ Pump | Defined$ Targeted | KW$ Flashback:0 | PumpZone$ Graveyard\n\
         SVar:FBUnsaddled:DB$ Pump | Defined$ Targeted | KW$ Flashback | PumpZone$ Graveyard",
    );
    assert!(newt.abilities[1].contains("Modifier::GrantsFlashbackCost(ManaCost::ZERO)"));
    assert!(newt.abilities[1].contains("Effect::GrantFlashback"));

    // The Fugitive Doctor: `KW$ Flashback:2 R G`.
    let doctor = read_w16b(
        "Name:FugitiveProbe\nManaCost:3 R G\nTypes:Legendary Creature\nPT:4/4\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TgtZone$ Graveyard | KW$ Flashback:2 R G | PumpZone$ Graveyard",
    );
    assert!(doctor.abilities[0].contains("Modifier::GrantsFlashbackCost(mana!(\"{2}{R}{G}\"))"));

    // Confession Dial: `KW$ Escape:CardManaCost ExileFromGrave<3/Card.Other/other>`.
    let dial = read_w16b(
        "Name:DialProbe\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Creature.Legendary+YouCtrl | KW$ Escape:CardManaCost ExileFromGrave<3/Card.Other/other> | TgtZone$ Graveyard | PumpZone$ Graveyard",
    );
    assert!(dial.abilities[0].contains("Modifier::GrantsEscape(3)"));

    // Cursecloth Wrappings: `KW$ Embalm:CardManaCost` & Araumi-style `KW$ Encore:CardManaCost` & Filigree Racer `KW$ Jump-start`.
    let wrappings = read_w16b(
        "Name:WrappingsProbe\nManaCost:2 B B\nTypes:Artifact\n\
         A:AB$ Pump | Cost$ T | TgtZone$ Graveyard | ValidTgts$ Creature.YouOwn | PumpZone$ Graveyard | KW$ Embalm:CardManaCost",
    );
    assert!(wrappings.abilities[0].contains("Modifier::GrantsEmbalmManaCost"));

    let encore = read_w16b(
        "Name:EncoreProbe\nManaCost:1 U B\nTypes:Creature\nPT:1/4\n\
         A:AB$ Pump | Cost$ T | TgtZone$ Graveyard | ValidTgts$ Creature.YouOwn | PumpZone$ Graveyard | KW$ Encore:CardManaCost",
    );
    assert!(encore.abilities[0].contains("Modifier::GrantsEncoreManaCost"));

    let jump = read_w16b(
        "Name:JumpProbe\nManaCost:3 R\nTypes:Artifact\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Instant.YouOwn,Sorcery.YouOwn | KW$ Jump-start | TgtZone$ Graveyard | PumpZone$ Graveyard",
    );
    assert!(jump.abilities[0].contains("Modifier::GrantsJumpStart"));

    // Refusal: non-CardManaCost Escape (e.g. Grim Captain's Locker `Escape:3 B ExileFromGrave<4/...>`).
    assert!(refused_w16b(
        "Name:FixedEscape\nManaCost:3 B\nTypes:Artifact\n\
         A:AB$ PumpAll | Cost$ T | PumpZone$ Graveyard | ValidCards$ Creature.YouOwn | KW$ Escape:3 B ExileFromGrave<4/Card.Other/other>"
    ));
}
