use super::*;

fn w26a_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Dinosaur".into(),
            "Dragon".into(),
            "Goblin".into(),
            "Spider".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into(), "Vehicle".into()],
        enchantment: vec!["Aura".into(), "Rune".into()],
        land: vec![
            "Desert".into(),
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w26a(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w26a_cats();
    transcode(&parsed, &cats, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, None)
        )
    })
}

fn refused_w26a(text: &str) -> bool {
    let parsed = parse(text);
    transcode(&parsed, &w26a_cats(), None).is_none()
}

#[test]
fn reduce_cost_if_targets_self_stack_board_and_abilities() {
    // Luminous Rebuke / Seized from Slumber / Quicksand Whirlpool: self-stack generic target reduction.
    let rebuke = read_w26a(
        "Name:Luminous Rebuke\nManaCost:4 W\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 3 | EffectZone$ All | ValidTarget$ Creature.tapped\n\
         A:SP$ Destroy | ValidTgts$ Creature",
    );
    assert!(
        rebuke.abilities.iter().any(|a| {
            a.contains("Modifier::ReduceCostIfTargets")
                && a.contains("who: PlayerRel::You")
                && a.contains("spell_filter: &Filter::This")
                && a.contains("reduction: mana!(\"{3}\")")
                && a.contains("abilities: false")
                && a.contains("zone = ZoneRef::Stack")
        }),
        "{:?}",
        rebuke.abilities
    );
    assert!(
        rebuke
            .statics
            .contains("Filter::And(&[Filter::CREATURE, Filter::Tapped])"),
        "{}",
        rebuke.statics
    );

    // Brush Off: self-stack colored mana reduction targeting instant or sorcery spell.
    let brush_off = read_w26a(
        "Name:Brush Off\nManaCost:2 U U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 1 | Color$ 1 U | ValidTarget$ Spell.Instant,Spell.Sorcery | EffectZone$ All\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card",
    );
    assert!(
        brush_off.abilities.iter().any(|a| {
            a.contains("Modifier::ReduceCostIfTargets")
                && a.contains("reduction: mana!(\"{1}{U}\")")
                && a.contains("zone = ZoneRef::Stack")
        }),
        "{:?}",
        brush_off.abilities
    );

    // Torch Breath: `Permanent.Blue` retains battlefield zone constraint (`Filter::InZone(ZoneRef::Battlefield)`).
    let torch_breath = read_w26a(
        "Name:Torch Breath\nManaCost:X R\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | ValidTarget$ Permanent.Blue | EffectZone$ All\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        torch_breath
            .statics
            .contains("Filter::InZone(ZoneRef::Battlefield)")
            && torch_breath.statics.contains("Color::Blue"),
        "{}",
        torch_breath.statics
    );

    // Killian, Ink Duelist: board spell cost reduction for spells targeting a creature.
    let killian = read_w26a(
        "Name:Killian, Ink Duelist\nManaCost:W B\nTypes:Legendary Creature Human Warlock\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Creature | Activator$ You | Type$ Spell | Amount$ 2",
    );
    assert_eq!(
        killian.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReduceCostIfTargets { who: PlayerRel::You, spell_filter: &Filter::Any, target_filter: &Filter::CREATURE, reduction: mana!(\"{2}\"), abilities: false })"
        ]
    );

    // Strong Back (Aura spell line): filtered `ValidCard$ Aura` + `ValidTarget$ Creature.EnchantedBy`.
    let strong_back = read_w26a(
        "Name:Strong Back\nManaCost:2 G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:SP$ Attach | ValidTgts$ Creature\n\
         S:Mode$ ReduceCost | ValidCard$ Aura | ValidTarget$ Creature.EnchantedBy | Activator$ You | Type$ Spell | Amount$ 3",
    );
    assert!(
        strong_back.abilities.iter().any(|a| {
            a.contains("Modifier::ReduceCostIfTargets")
                && a.contains("reduction: mana!(\"{3}\")")
                && a.contains("abilities: false")
        }),
        "{:?}",
        strong_back.abilities
    );

    // Bladegraft Aspirant: general activated-ability reduction (`abilities: true`) targeting self.
    let bladegraft = read_w26a(
        "Name:Bladegraft Aspirant\nManaCost:2 R\nTypes:Creature\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidCard$ Equipment.YouCtrl | Activator$ You | Type$ Ability | Amount$ 1",
    );
    assert!(
        bladegraft.abilities.iter().any(|a| {
            a.contains("Modifier::ReduceCostIfTargets")
                && a.contains("target_filter: &Filter::This")
                && a.contains("reduction: mana!(\"{1}\")")
                && a.contains("abilities: true")
        }),
        "{:?}",
        bladegraft.abilities
    );

    // Colored board target reduction targeting self (unlocked from old Wave 12b refusal boundary).
    let colored_self = read_w26a(
        "Name:Azure Target\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | Activator$ You | Type$ Spell | Color$ U | Amount$ 1",
    );
    assert_eq!(
        colored_self.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReduceCostIfTargets { who: PlayerRel::You, spell_filter: &Filter::Any, target_filter: &Filter::This, reduction: mana!(\"{U}\"), abilities: false })"
        ]
    );
}

#[test]
fn adjust_spell_cost_by_amount_wave14_reductions_and_dynamic_taxes() {
    // Ghalta, Primal Hunger: `Count$Valid Creature.YouCtrl$CardPower` -> `TotalPowerAmong`.
    let ghalta = read_w26a(
        "Name:Ghalta, Primal Hunger\nManaCost:10 G G\nTypes:Legendary Creature Dinosaur\nPT:12/12\n\
         K:Trample\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$Valid Creature.YouCtrl$CardPower",
    );
    assert!(
        ghalta.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::AdjustSpellCostByAmount { who: PlayerRel::You, filter: &Filter::This, increase: false, amount: &Amount::TotalPowerAmong(&Filter::YOUR_CREATURE) }, zone = ZoneRef::Stack)"
        }),
        "{:?}",
        ghalta.abilities
    );

    // Daybreak Chimera / Drag to the Underworld: `Count$Devotion.White` -> `Amount::Devotion`.
    let chimera = read_w26a(
        "Name:Daybreak Chimera\nManaCost:3 W W\nTypes:Creature\nPT:3/3\n\
         K:Flying\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$Devotion.White",
    );
    assert!(
        chimera.abilities.iter().any(|a| {
            a.contains("Modifier::AdjustSpellCostByAmount")
                && a.contains("increase: false")
                && a.contains("Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::White]) }")
        }),
        "{:?}",
        chimera.abilities
    );

    // Deadly Alliance / Thwart the Grave: `Count$Party` -> `Amount::PartySize(PlayerRel::You)`.
    let alliance = read_w26a(
        "Name:Deadly Alliance\nManaCost:4 B\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$Party\n\
         A:SP$ Destroy | ValidTgts$ Creature",
    );
    assert!(
        alliance.abilities.iter().any(|a| {
            (a.contains("Modifier::SpellsCostLessByAmount(Amount::PartySize(PlayerRel::You))")
                || (a.contains("Modifier::AdjustSpellCostByAmount")
                    && a.contains("amount: &Amount::PartySize(PlayerRel::You)")))
                && a.contains("zone = ZoneRef::Stack")
        }),
        "{:?}",
        alliance.abilities
    );

    // Dynamic `RaiseCost` spell tax using `AdjustSpellCostByAmount { increase: true }`.
    let tax = read_w26a(
        "Name:Devoted Tax\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ X\n\
         SVar:X:Count$Devotion.White",
    );
    assert_eq!(
        tax.abilities,
        [
            "static_ability!(Filter::Any, Modifier::AdjustSpellCostByAmount { who: PlayerRel::EachOpponent, filter: &Filter::Any, increase: true, amount: &Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::White]) } })"
        ]
    );

    // Dynamic `RaiseCost` using base `static_count_expr` (`Count$Valid Artifact.YouCtrl`).
    let artifact_tax = read_w26a(
        "Name:Relic Toll\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ RaiseCost | ValidCard$ Card.nonCreature | Activator$ Opponent | Type$ Spell | Amount$ X\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
    );
    assert!(
        artifact_tax.abilities.iter().any(|a| {
            a.contains("Modifier::AdjustSpellCostByAmount")
                && a.contains("who: PlayerRel::EachOpponent")
                && a.contains("increase: true")
                && a.contains("ZoneSel::Battlefield")
        }),
        "{:?}",
        artifact_tax.abilities
    );
}

#[test]
fn continuous_check_svar_wave14_devotion_party_draws_spells_and_power() {
    // Heliod, Sun-Crowned (mono-color Theros God): `Devotion.White` LT5.
    let heliod = read_w26a(
        "Name:Heliod, Sun-Crowned\nManaCost:2 W\nTypes:Legendary Enchantment Creature\nPT:5/5\n\
         K:Indestructible\n\
         S:Mode$ Continuous | AffectedDefined$ Self | RemoveType$ Creature | CheckSVar$ X | SVarCompare$ LT5\n\
         SVar:X:Count$Devotion.White",
    );
    assert!(
        heliod.abilities.iter().any(|a| {
            a.contains("Modifier::RemoveType(TypeSet::CREATURE)")
                && a.contains("Condition::Not(&Condition::AmountAtLeast(Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::White]) }, 5))")
        }),
        "{:?}",
        heliod.abilities
    );

    // Phenax, God of Deception (dual-color Theros God): `DevotionDual.Blue.Black` LT7.
    let phenax = read_w26a(
        "Name:Phenax, God of Deception\nManaCost:3 U B\nTypes:Legendary Enchantment Creature\nPT:4/7\n\
         K:Indestructible\n\
         S:Mode$ Continuous | AffectedDefined$ Self | RemoveType$ Creature | CheckSVar$ X | SVarCompare$ LT7\n\
         SVar:X:Count$DevotionDual.Blue.Black",
    );
    assert!(
        phenax.abilities.iter().any(|a| {
            a.contains("Condition::Not(&Condition::AmountAtLeast(Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Blue, Color::Black]) }, 7))")
        }),
        "{:?}",
        phenax.abilities
    );

    // Spinehorn Minotaur: "As long as you've drawn two or more cards this turn, has double strike."
    let minotaur = read_w26a(
        "Name:Spinehorn Minotaur\nManaCost:2 R\nTypes:Creature\nPT:2/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Double Strike | CheckSVar$ X | SVarCompare$ GE2\n\
         SVar:X:Count$YouDrewThisTurn",
    );
    assert!(
        minotaur.abilities.iter().any(|a| {
            a.contains("KeywordSet::DOUBLE_STRIKE")
                && a.contains(
                    "Condition::AmountAtLeast(Amount::CardsDrawnThisTurn(PlayerRel::You), 2)",
                )
        }),
        "{:?}",
        minotaur.abilities
    );

    // Full party (`EQ4`) + second-spell (`GE2` on `ThisTurnCast_Card.YouCtrl`) + greatest-power compare (`GTY`).
    let party_and_spells = read_w26a(
        "Name:Party Tactician\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Vigilance | CheckSVar$ P | SVarCompare$ EQ4\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ C | SVarCompare$ GE2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | CheckSVar$ MyPow | SVarCompare$ GTOppPow\n\
         SVar:P:Count$Party\n\
         SVar:C:Count$ThisTurnCast_Card.YouCtrl\n\
         SVar:MyPow:Count$Valid Creature.YouCtrl$GreatestCardPower\n\
         SVar:OppPow:Count$Valid Creature.OppCtrl$GreatestCardPower",
    );
    assert!(
        party_and_spells
            .abilities
            .iter()
            .any(|a| a.contains("Condition::AmountAtLeast(Amount::PartySize(PlayerRel::You), 4)")),
        "{:?}",
        party_and_spells.abilities
    );
    assert!(
        party_and_spells.abilities.iter().any(|a| {
            a.contains("Condition::AmountAtLeast(Amount::SpellsCastThisTurn(PlayerRel::You), 2)")
        }),
        "{:?}",
        party_and_spells.abilities
    );
    assert!(
        party_and_spells.abilities.iter().any(|a| {
            a.contains("Condition::AmountCompare")
                && a.contains("Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE)")
                && a.contains("AmountCmp::GreaterThan")
                && a.contains("Amount::GreatestPowerAmong(&Filter::OPPONENT_CREATURE)")
        }),
        "{:?}",
        party_and_spells.abilities
    );

    // Knowledge Is Power + Emissary Escort dynamic P/T via Wave 14 amounts.
    let pt_wave14 = read_w26a(
        "Name:Scholarly Escort\nManaCost:3 U\nTypes:Creature\nPT:1/1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ X | AddToughness$ X\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ Y\n\
         SVar:X:Count$YouDrewThisTurn\n\
         SVar:Y:Count$Valid Artifact.Other+YouCtrl$GreatestCardManaCost",
    );
    assert!(
        pt_wave14
            .abilities
            .iter()
            .any(|a| { a.contains("Amount::CardsDrawnThisTurn(PlayerRel::You)") }),
        "{:?}",
        pt_wave14.abilities
    );
    assert!(
        pt_wave14
            .abilities
            .iter()
            .any(|a| { a.contains("Amount::GreatestManaValueAmong(") }),
        "{:?}",
        pt_wave14.abilities
    );
}

#[test]
fn must_block_static_unlocks_paper_creatures_and_enchantments() {
    // Watchdog / Relentless Raptor / Grand Melee / Spirespine shapes.
    let watchdog = read_w26a(
        "Name:Watchdog\nManaCost:3\nTypes:Artifact Creature\nPT:1/2\n\
         S:Mode$ MustBlock | ValidCreature$ Card.Self",
    );
    assert_eq!(
        watchdog.abilities,
        ["static_ability!(Filter::This, Modifier::MustBlock)"]
    );

    let grand_melee = read_w26a(
        "Name:Grand Melee\nManaCost:3 R\nTypes:Enchantment\n\
         S:Mode$ MustAttack | ValidCreature$ Creature\n\
         S:Mode$ MustBlock | ValidCreature$ Creature",
    );
    assert_eq!(
        grand_melee.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttacksEachCombat)",
            "static_ability!(Filter::CREATURE, Modifier::MustBlock)",
        ]
    );

    let spirespine = read_w26a(
        "Name:Spirespine\nManaCost:2 G\nTypes:Enchantment Creature\nPT:4/1\n\
         S:Mode$ MustBlock | ValidCreature$ Card.Self,Card.EnchantedBy",
    );
    assert!(
        spirespine.abilities.iter().any(|a| {
            a.contains("Filter::Or(&[Filter::This, Filter::AttachedToBySource])")
                && a.contains("Modifier::MustBlock")
        }),
        "{:?}",
        spirespine.abilities
    );
}

#[test]
fn exile_and_command_graveyard_may_play_normal_and_normal_plus() {
    // Eternal Scourge / Misthollow Griffin: self-cast from exile with normal cost.
    let scourge = read_w26a(
        "Name:Eternal Scourge\nManaCost:3\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.Self | MayPlay$ True | AffectedZone$ Exile | EffectZone$ Exile",
    );
    assert_eq!(
        scourge.abilities,
        [
            "static_ability!(Filter::This, Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Exile]), grant_zone_permission: true, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Normal }, zone = ZoneRef::Exile)"
        ]
    );

    // Self-cast from exile with `RaiseCost$` -> `AltCastPrice::NormalPlus`.
    let taxed_exile = read_w26a(
        "Name:Ashen Phantom\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | MayPlay$ True | AffectedZone$ Exile | EffectZone$ Exile | RaiseCost$ PayLife<2>",
    );
    assert!(
        taxed_exile.abilities.iter().any(|a| {
            a.contains("price: AltCastPrice::NormalPlus(cost!(PayLife(2)))")
                && a.contains("zone = ZoneRef::Exile")
        }),
        "{:?}",
        taxed_exile.abilities
    );

    // Grolnok-style counter-marked owned exile (`Card.YouOwn+counters_GE1_CHARGE` with `AltCastPrice::Normal`).
    let exile_counter = read_w26a(
        "Name:Archive Toad\nManaCost:2 G U\nTypes:Legendary Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.YouOwn+counters_GE1_CHARGE | AffectedZone$ Exile | MayPlay$ True",
    );
    assert!(
        exile_counter.abilities.iter().any(|a| {
            a.contains("AltCastZones::Zones(&[ZoneRef::Exile])")
                && a.contains("grant_zone_permission: true")
                && a.contains("allow_lands: true")
                && a.contains("price: AltCastPrice::Normal")
                && a.contains("Filter::OwnedByYou")
                && a.contains("CounterKind::Charge")
        }),
        "{:?}",
        exile_counter.abilities
    );

    // Command-zone filtered graveyard land play (`Modifier::PlayLandsFromGraveyard` with subtype filter).
    let desert_emblem = read_w26a(
        "Name:Desert Emblem\nTypes:Emblem\n\
         S:Mode$ Continuous | EffectZone$ Command | AffectedZone$ Graveyard | Affected$ Land.Desert+YouOwn | MayPlay$ True",
    );
    assert_eq!(
        desert_emblem.abilities,
        [
            "static_ability!(Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::DESERT)]), Modifier::PlayLandsFromGraveyard, zone = ZoneRef::Command)"
        ]
    );

    // Command-zone graveyard cast with `RaiseCost$` -> `AltCastPrice::NormalPlus`.
    let gy_raise_emblem = read_w26a(
        "Name:Ember Emblem\nTypes:Emblem\n\
         S:Mode$ Continuous | EffectZone$ Command | AffectedZone$ Graveyard | Affected$ Creature.YouOwn | MayPlay$ True | RaiseCost$ PayLife<2>",
    );
    assert!(
        gy_raise_emblem.abilities.iter().any(|a| {
            a.contains("AltCastZones::Zones(&[ZoneRef::Graveyard])")
                && a.contains("grant_zone_permission: true")
                && a.contains("allow_lands: false")
                && a.contains("price: AltCastPrice::NormalPlus(cost!(PayLife(2)))")
                && a.contains("zone = ZoneRef::Command")
        }),
        "{:?}",
        gy_raise_emblem.abilities
    );
}

#[test]
fn alternative_cost_wave14_conditions_and_graveyard_counts() {
    // Self `AlternativeCost` with Wave 14 `CheckSVar` conditions: Devotion, Party (`EQ4`),
    // CardsDrawnThisTurn, SpellsCastThisTurn (`GE2`), LifeYouGainedThisTurn, and filtered graveyard count.
    let alt_devotion = read_w26a(
        "Name:Devoted Bolt\nManaCost:3 R\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ R | CheckSVar$ X | SVarCompare$ GE5\n\
         SVar:X:Count$Devotion.Red\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        alt_devotion.alternative_costs,
        [
            "AlternativeCost { cost: cost!(\"{R}\"), condition: AltCondition::If(&Condition::AmountAtLeast(Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Red]) }, 5)) }"
        ]
    );

    let alt_party_and_draws = read_w26a(
        "Name:Full Party Trick\nManaCost:4 U\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ U | CheckSVar$ P | SVarCompare$ EQ4 | CheckSecondSVar$ D | SecondSVarCompare$ GE2\n\
         SVar:P:Count$Party\n\
         SVar:D:Count$YouDrewThisTurn\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert!(
        alt_party_and_draws.alternative_costs[0]
            .contains("Condition::AmountAtLeast(Amount::PartySize(PlayerRel::You), 4)")
            && alt_party_and_draws.alternative_costs[0].contains(
                "Condition::AmountAtLeast(Amount::CardsDrawnThisTurn(PlayerRel::You), 2)"
            ),
        "{:?}",
        alt_party_and_draws.alternative_costs
    );

    let alt_gy_and_spells = read_w26a(
        "Name:Grave Surge\nManaCost:3 B\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ B | CheckSVar$ G | SVarCompare$ GE3 | CheckSecondSVar$ S | SecondSVarCompare$ GE2\n\
         SVar:G:Count$ValidGraveyard Creature.YouOwn\n\
         SVar:S:Count$ThisTurnCast_Card.YouCtrl\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert!(
        alt_gy_and_spells.alternative_costs[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)")
            && alt_gy_and_spells.alternative_costs[0].contains(
                "Condition::AmountAtLeast(Amount::SpellsCastThisTurn(PlayerRel::You), 2)"
            ),
        "{:?}",
        alt_gy_and_spells.alternative_costs
    );

    // `AlternativeCost` conditioned on `Count$LifeYouGainedThisTurn`.
    let life_gained_alt = read_w26a(
        "Name:Angelic Surge\nManaCost:4 W\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ W | CheckSVar$ L | SVarCompare$ GE3\n\
         SVar:L:Count$LifeYouGainedThisTurn\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert_eq!(
        life_gained_alt.alternative_costs,
        [
            "AlternativeCost { cost: cost!(\"{W}\"), condition: AltCondition::If(&Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 3)) }"
        ]
    );

    // `AlternativeCost` conditioned on `Count$DevotionDual.White.Blue` and `Count$ThisTurnCast_Card`.
    let dual_devotion_alt = read_w26a(
        "Name:Azorius Surge\nManaCost:4 W U\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ W U | CheckSVar$ D | SVarCompare$ GE7 | CheckSecondSVar$ C | SecondSVarCompare$ GE3\n\
         SVar:D:Count$DevotionDual.White.Blue\n\
         SVar:C:Count$ThisTurnCast_Card\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert!(
        dual_devotion_alt.alternative_costs[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::White, Color::Blue]) }"
        ) && dual_devotion_alt.alternative_costs[0]
            .contains("Condition::AmountAtLeast(Amount::SpellsCastThisTurn(PlayerRel::EachPlayer), 3)"),
        "{:?}",
        dual_devotion_alt.alternative_costs
    );
}

#[test]
fn wave26a_refusals_soundness_and_non_paper() {
    for bad in [
        // Player targets in `ReduceCost.ValidTarget` must stay refused (engine matches object targets only).
        "Name:BadPlayerTgt\nManaCost:3 R\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | ValidTarget$ Player.Opponent\n\
         A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3",
        // `IgnoreGeneric$ True` with `ReduceCost.ValidTarget` must stay refused (`ReduceCostIfTargets` spills colored excess).
        "Name:BadIgnoreGenericTgt\nManaCost:2 U U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 1 | Color$ U | IgnoreGeneric$ True | EffectZone$ All | ValidTarget$ Spell.Blue\n\
         A:SP$ Draw | NumCards$ 1",
        // `ValidSpell$ Activated.Equip` with `ValidTarget$` and `MinMana$ 1` must stay refused.
        "Name:BadEquipTgtMinMana\nManaCost:R\nTypes:Creature\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 3 | MinMana$ 1",
        // `Type$ Ability` with `ValidTarget$` and `MinMana$ 1` must stay refused (unfloored engine modifier).
        "Name:BadMinManaAbilityTgt\nManaCost:2 R\nTypes:Creature\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidCard$ Equipment.YouCtrl | Activator$ You | Type$ Ability | Amount$ 1 | MinMana$ 1",
        // Chosen-color devotion has no DSL `Amount::Devotion` variant.
        "Name:BadChosenDevotion\nManaCost:3 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ D | SVarCompare$ GE5\n\
         SVar:D:Count$Devotion.Chosen",
        // Land card with self-exile `MayPlay$ True` must stay refused.
        "Name:BadExileLand\nTypes:Land\n\
         S:Mode$ Continuous | Affected$ Card.Self | MayPlay$ True | AffectedZone$ Exile | EffectZone$ Exile",
        // Unlinked `ExiledWithSource` without any exile producer must stay refused.
        "Name:BadUnlinkedExile\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Card.ExiledWithSource | AffectedZone$ Exile | MayPlay$ True",
        // `RaiseCost$` combined with land-play permission (`Card.YouOwn+counters_GE1_CHARGE` allows lands) must stay refused.
        "Name:BadLandRaiseCost\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.YouOwn+counters_GE1_CHARGE | AffectedZone$ Exile | MayPlay$ True | RaiseCost$ PayLife<2>",
        // `MustBlock` with extra/unsupported attacker restriction must stay refused.
        "Name:BadMustBlockAttacker\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ MustBlock | ValidCreature$ Card.Self | ValidAttacker$ Creature.withFlying",
        // Alchemy / digital-only conjure alongside Wave 26a static must stay refused.
        "Name:BadAlchemy\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ MustBlock | ValidCreature$ Card.Self\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ C\n\
         SVar:C:DB$ MakeCard | Conjure$ True | Name$ Plains | Zone$ Hand",
        // Un-set / Sticker / Planechase mechanics alongside Wave 26a static must stay refused.
        "Name:BadUnset\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         S:Mode$ MustBlock | ValidCreature$ Card.Self\n\
         T:Mode$ PlaneswalkedTo | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1",
    ] {
        assert!(refused_w26a(bad), "must refuse:\n{bad}");
    }
}
