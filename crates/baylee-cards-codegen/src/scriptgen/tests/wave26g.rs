//! Wave 26g unit tests:
//! - `mana.rs`: Wave 14a dynamic `SVar` filter atoms (`Filter::{CmcAtMostAmount,
//!   CmcEqualsAmount, PowerAtMostAmount, PowerAtLeastAmount}`), generalized
//!   `source_power_atom`, Wave 14 `Amount` counters (`PartySize`,
//!   `GreatestManaValueAmong`, `SacrificedTotalToughness`,
//!   `SacrificedTotalManaValue`, `SacrificedDistinctColors`), `ManaReflected`
//!   `Produce` (`Gate.YouCtrl`, `Basic.YouCtrl`, general costs), and
//!   `RestrictValid$ Spell.Kicked`.
//! - `saga_ward_convoke.rs`: Wave 14a `AffectedZone$ Library` / `Graveyard`
//!   permissions (`MayLookAtTopOfLibrary`, `MayCastWithAltCost` with
//!   `AltCastPrice::{Normal, NormalPlus}`, filtered and self-graveyard
//!   `PlayLandsFromGraveyard`), Wave 14a `Modifier::GrantFlash`,
//!   `K:Ward:Blight<N>`, and expanded `extra_svar_condition_clause`.
//! - `kicker.rs`: dual-kicker `Card.kicked <1|2>` / `Count$Kicked<1|2>.1.0`,
//!   inverted `Count$Kicked.0.1` / `Count$OptionalGenericCostPaid.0.1`,
//!   broader `Count$TimesKicked` comparisons, and spell cost tokens (`Forage`
//!   on `OptionalCost`/spell `Cost$`, `RevealOrChoose`, `Exile`,
//!   `Discard<X/...>`, `ExileFromGrave<X/...>`, `ExileFromGrave<4/...>`).
//! - `divided_damage.rs`: `DivisionAmount::XDoubleFrom`, inline
//!   `Count$xPaid/Plus.N`, `Number$N`, `TargetMax$ <Count$Valid same>`, safe
//!   non-retargeting `SubAbility$` continuations on `divided_damage_spell`,
//!   and `LastStateBattlefield` / unscoped `LastStateGraveyardWithFallback` on
//!   `read_divided_counters`.
//! - Strict pure Paper-MTG and soundness refusal guardrails.

use super::*;

fn w26g_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Cat".into(),
            "Demon".into(),
            "Djinn".into(),
            "Dragon".into(),
            "Elf".into(),
            "Goblin".into(),
            "Human".into(),
            "Monk".into(),
            "Nightmare".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into(), "Food".into()],
        enchantment: vec!["Aura".into(), "Saga".into()],
        land: vec![
            "Desert".into(),
            "Forest".into(),
            "Gate".into(),
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

fn read26(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w26g_cats();
    transcode(&parsed, &cats, None).unwrap_or_else(|| {
        panic!(
            "expected Wave 26g translation, refused: {:?}\n{text}",
            refusal_reason(&parsed, &cats, None)
        )
    })
}

fn refused26(text: &str) -> bool {
    let parsed = parse(text);
    let cats = w26g_cats();
    transcode(&parsed, &cats, None).is_none()
}

fn rendered(body: &CardBody) -> String {
    format!("{}\n{}", body.abilities.join("\n"), body.statics)
}

#[test]
fn mana_wave14a_dynamic_cmc_filter_atoms() {
    // Lay Down Arms: `Creature.cmcLEX` bounded by `Count$Valid Plains.YouCtrl`.
    let lay_down_arms = read26(
        "Name:Lay Down Arms\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.cmcLEX | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 3 | Defined$ TargetedController\n\
         SVar:X:Count$Valid Plains.YouCtrl\n",
    );
    let text = rendered(&lay_down_arms);
    assert!(text.contains("Filter::CmcAtMostAmount"), "{text}");
    assert!(text.contains("subtypes::land::PLAINS"), "{text}");
    assert!(text.contains("Effect::exile("), "{text}");

    // `Count$CastTotalManaSpent` on spell `cmcLEX`.
    let cast_spent = read26(
        "Name:ManaBound\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLEX\n\
         SVar:X:Count$CastTotalManaSpent\n",
    );
    let text = rendered(&cast_spent);
    assert!(
        text.contains("Filter::CmcAtMostAmount(&Amount::ManaSpentToCast)"),
        "{text}"
    );

    // `TriggerCount$DamageAmount` on `DamageDone` trigger `cmcLEX`.
    let dmg_bound = read26(
        "Name:DamageBound\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ E\n\
         SVar:E:DB$ Destroy | ValidTgts$ Artifact.cmcLEX\n\
         SVar:X:TriggerCount$DamageAmount\n",
    );
    let text = rendered(&dmg_bound);
    assert!(
        text.contains("Filter::CmcAtMostAmount(&Amount::EventAmount)"),
        "{text}"
    );

    // `cmcEQX` with `Count$CardPower`, `cmcGEX`, and `cmcGTX`.
    let cmc_eq_power = read26(
        "Name:PowerMatch\nManaCost:2 G\nTypes:Creature\nPT:3/3\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Artifact.cmcEQX\n\
         SVar:X:Count$CardPower\n",
    );
    assert!(
        rendered(&cmc_eq_power).contains("Filter::CmcEqualsAmount(&Amount::SourcePower)"),
        "{}",
        rendered(&cmc_eq_power)
    );

    let cmc_ge = read26(
        "Name:HighCmc\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcGEX\n\
         SVar:X:Count$Valid Creature.YouCtrl\n",
    );
    let text = rendered(&cmc_ge);
    assert!(
        text.contains("Filter::CmcEqualsAmount")
            && text.contains("Filter::Not(&Filter::CmcAtMostAmount"),
        "{text}"
    );

    let cmc_gt = read26(
        "Name:StrictlyHighCmc\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Creature.cmcGTX\n\
         SVar:X:Count$CardPower\n",
    );
    assert!(
        rendered(&cmc_gt).contains("Filter::Not(&Filter::CmcAtMostAmount(&Amount::SourcePower))"),
        "{}",
        rendered(&cmc_gt)
    );

    // Scalar dynamic counts (`Count$YouDrewThisTurn`, `Count$ThisTurnCast_Card.OppCtrl`).
    let drew_bound = read26(
        "Name:DrewBound\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLEX\n\
         SVar:X:Count$YouDrewThisTurn\n",
    );
    assert!(
        rendered(&drew_bound)
            .contains("Filter::CmcAtMostAmount(&Amount::CardsDrawnThisTurn(PlayerRel::You))"),
        "{}",
        rendered(&drew_bound)
    );

    let opp_cast_bound = read26(
        "Name:OppCastBound\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLEX\n\
         SVar:X:Count$ThisTurnCast_Card.OppCtrl\n",
    );
    assert!(
        rendered(&opp_cast_bound).contains(
            "Filter::CmcAtMostAmount(&Amount::SpellsCastThisTurn(PlayerRel::EachOpponent))"
        ),
        "{}",
        rendered(&opp_cast_bound)
    );
}

#[test]
fn mana_wave14a_dynamic_power_filter_atoms_and_refusals() {
    let power_le = read26(
        "Name:PowerLe\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerLEX\n\
         SVar:X:Count$Valid Plains.YouCtrl\n",
    );
    assert!(
        rendered(&power_le).contains("Filter::PowerAtMostAmount"),
        "{}",
        rendered(&power_le)
    );

    let power_ge = read26(
        "Name:PowerGe\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerGEX\n\
         SVar:X:Count$Valid Creature.OppCtrl$GreatestCardPower\n",
    );
    assert!(
        rendered(&power_ge).contains("Filter::PowerAtLeastAmount(&Amount::GreatestPowerAmong"),
        "{}",
        rendered(&power_ge)
    );

    let power_gt = read26(
        "Name:PowerGt\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerGTX\n\
         SVar:X:Count$Valid Creature.YouCtrl\n",
    );
    let text = rendered(&power_gt);
    assert!(
        text.contains("Filter::PowerAtLeastAmount(&Amount::Plus { base:")
            && text.contains("offset: 1"),
        "{text}"
    );

    let power_eq = read26(
        "Name:PowerEq\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerEQX\n\
         SVar:X:Count$Valid Creature.YouCtrl\n",
    );
    let text = rendered(&power_eq);
    assert!(
        text.contains("Filter::PowerAtLeastAmount") && text.contains("Filter::PowerAtMostAmount"),
        "{text}"
    );

    // Generalized `source_power_atom` for non-`X` SVar names (e.g. `powerLTY`).
    let lena_shape = read26(
        "Name:LenaShape\nManaCost:4 W W\nTypes:Creature\nPT:3/3\n\
         A:AB$ PumpAll | Cost$ Sac<1/CARDNAME> | ValidCards$ Creature.YouCtrl+powerLTY | KW$ Indestructible\n\
         SVar:Y:Count$CardPower\n",
    );
    assert!(
        rendered(&lena_shape).contains("Filter::PowerLessThanSourcePower"),
        "{}",
        rendered(&lena_shape)
    );

    // Refusals: `cmcLTX` (zero-floor hazard), recursive dynamic SVar filter.
    for bad in [
        "Name:BadCmcLt\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLTX\n\
         SVar:X:Count$Valid Plains.YouCtrl\n",
        "Name:BadRecursive\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLEX\n\
         SVar:X:Count$Valid Creature.cmcLEX\n",
    ] {
        assert!(refused26(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn mana_wave14_counts_reflected_and_kicked_restrict() {
    // Wave 14 `Amount` counts in `count_expr_depth`.
    let party = read26(
        "Name:PartyLife\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\n\
         SVar:X:Count$Party\n",
    );
    assert!(
        rendered(&party).contains("Amount::PartySize(PlayerRel::You)"),
        "{}",
        rendered(&party)
    );

    let greatest_mv = read26(
        "Name:GreatestMv\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardManaCost\n",
    );
    assert!(
        rendered(&greatest_mv).contains("Amount::GreatestManaValueAmong"),
        "{}",
        rendered(&greatest_mv)
    );

    let sac_tough = read26(
        "Name:SacTough\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ GainLife | Cost$ 1 B Sac<1/Creature> | LifeAmount$ X\n\
         SVar:X:Sacrificed$TotalCardToughness\n",
    );
    assert!(
        rendered(&sac_tough).contains("Amount::SacrificedTotalToughness"),
        "{}",
        rendered(&sac_tough)
    );

    let sac_mv = read26(
        "Name:SacMv\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ GainLife | Cost$ 1 B Sac<1/Creature> | LifeAmount$ X\n\
         SVar:X:Sacrificed$TotalCardManaCost\n",
    );
    assert!(
        rendered(&sac_mv).contains("Amount::SacrificedTotalManaValue"),
        "{}",
        rendered(&sac_mv)
    );

    let sac_colors = read26(
        "Name:SacColors\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ GainLife | Cost$ 1 G Sac<1/Creature> | LifeAmount$ X\n\
         SVar:X:Sacrificed$Colors\n",
    );
    assert!(
        rendered(&sac_colors).contains("Amount::SacrificedDistinctColors"),
        "{}",
        rendered(&sac_colors)
    );

    // Gond Gate: `ManaReflected` `Produce` with `Gate.YouCtrl`.
    let gond_gate = read26(
        "Name:Gond Gate\nManaCost:no cost\nTypes:Land Gate\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 1\n\
         A:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Gate.YouCtrl | ReflectProperty$ Produce\n",
    );
    let text = rendered(&gond_gate);
    assert!(
        text.contains(
            "Effect::mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, false)"
        ),
        "{text}"
    );

    // `ManaReflected` `Produce` with `Basic.YouCtrl` and non-tap cost `1 T`.
    let basic_reflect = read26(
        "Name:BasicReflect\nManaCost:2\nTypes:Artifact\n\
         A:AB$ ManaReflected | Cost$ 1 T | ColorOrType$ Color | Valid$ Basic.YouCtrl | ReflectProperty$ Produce\n",
    );
    let text = rendered(&basic_reflect);
    assert!(
        text.contains("Effect::mana_from_lands(&Filter::BASIC_LAND, true, false)"),
        "{text}"
    );

    // `RestrictValid$ Spell.Kicked`.
    let kicked_mana = read26(
        "Name:KickedMana\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ G | Amount$ 2 | RestrictValid$ Spell.Kicked\n",
    );
    let text = rendered(&kicked_mana);
    assert!(text.contains("Filter::Kicked"), "{text}");
    assert!(text.contains(".restricted("), "{text}");
}

#[test]
fn saga_ward_convoke_wave14a_library_top_permissions() {
    // Elven Chorus: look at top + cast creature spells from top of library.
    let elven_chorus = read26(
        "Name:Elven Chorus\nManaCost:3 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | Affected$ Creature.TopLibrary+YouCtrl+nonLand | AffectedZone$ Library | MayPlay$ True\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddAbility$ AnyMana\n\
         SVar:AnyMana:AB$ Mana | Cost$ T | Produced$ Any | Amount$ 1\n\
         Oracle:You may look at the top card of your library any time.\\nYou may cast creature spells from the top of your library.\\nCreatures you control have \"{T}: Add one mana of any color.\"\n",
    );
    let text = rendered(&elven_chorus);
    assert!(text.contains("Modifier::LookAtLibraryTop"), "{text}");
    assert!(
        text.contains(
            "Modifier::MayCastWithAltCost { who: PlayerRel::You, zones: AltCastZones::TopOfLibrary, grant_zone_permission: true, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: AltCastPrice::Normal }"
        ),
        "{text}"
    );

    // Elsha of the Infinite: cast noncreature spells from top with flash.
    let elsha = read26(
        "Name:Elsha of the Infinite\nManaCost:2 U R W\nTypes:Legendary Creature Djinn Monk\nPT:3/3\n\
         K:Prowess\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl+nonLand+nonCreature | AffectedZone$ Library | MayPlay$ True | MayPlayWithFlash$ True\n",
    );
    let text = rendered(&elsha);
    assert!(
        text.contains(
            "zones: AltCastZones::TopOfLibrary, grant_zone_permission: true, allow_lands: false, with_flash: true"
        ),
        "{text}"
    );

    // Assemble the Players: once per turn (`MayPlayLimit$ 1`) + `powerLE2`.
    let assemble = read26(
        "Name:Assemble the Players\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | MayPlay$ True | MayPlayLimit$ 1 | Affected$ Card.Creature+nonLand+powerLE2+TopLibrary+YouCtrl | AffectedZone$ Library\n",
    );
    let text = rendered(&assemble);
    assert!(
        text.contains("once_per_turn: true, price: AltCastPrice::Normal"),
        "{text}"
    );
    assert!(text.contains("Filter::PowerAtMost(2)"), "{text}");

    // Experimental Frenzy: play lands and cast spells from top (`allow_lands: true`).
    let frenzy = read26(
        "Name:Experimental Frenzy\nManaCost:3 R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayPlay$ True\n\
         Oracle:You may look at the top card of your library any time.\\nYou may play lands and cast spells from the top of your library.\n",
    );
    let text = rendered(&frenzy);
    assert!(
        text.contains(
            "zones: AltCastZones::TopOfLibrary, grant_zone_permission: true, allow_lands: true"
        ),
        "{text}"
    );

    // Combined `MayLookAt$ You | MayPlay$ True` on unfiltered top card emits `MayLookAtTopOfLibrary`.
    let combined = read26(
        "Name:CombinedTop\nManaCost:3 U\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You | MayPlay$ True\n",
    );
    let text = rendered(&combined);
    assert!(
        text.contains("Modifier::MayLookAtTopOfLibrary { who: PlayerRel::You }"),
        "{text}"
    );
    assert!(text.contains("Modifier::MayCastWithAltCost"), "{text}");

    // Refusals: `MayLookAt$` on filtered top card, `MayPlayLimit$ 2`, invalid `MayPlayWithFlash$`.
    for bad in [
        "Name:BadLookFiltered\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You | MayPlay$ True",
        "Name:BadLimit2\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.TopLibrary+YouCtrl+nonLand | AffectedZone$ Library | MayPlay$ True | MayPlayLimit$ 2",
        "Name:BadFlashVal\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.TopLibrary+YouCtrl+nonLand | AffectedZone$ Library | MayPlay$ True | MayPlayWithFlash$ False",
    ] {
        assert!(refused26(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn saga_ward_convoke_wave14a_graveyard_permissions() {
    // Lurrus of the Dream-Den's graveyard casting static (with redundant `ValidAfterStack$`).
    let lurrus = read26(
        "Name:LurrusShape\nManaCost:1 W B\nTypes:Legendary Creature Cat Nightmare\nPT:3/2\n\
         K:Lifelink\n\
         S:Mode$ Continuous | EffectZone$ Battlefield | Condition$ PlayerTurn | MayPlay$ True | MayPlayLimit$ 1 | Affected$ Permanent.nonLand+YouOwn+cmcLE2 | ValidAfterStack$ Spell.cmcLE2 | AffectedZone$ Graveyard\n",
    );
    let text = rendered(&lurrus);
    assert!(
        text.contains(
            "zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), grant_zone_permission: true, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: true, price: AltCastPrice::Normal"
        ),
        "{text}"
    );
    assert!(text.contains("Filter::OwnedByYou"), "{text}");
    assert!(
        text.contains("condition = Some(Condition::YourTurn)"),
        "{text}"
    );

    // Demonic Embrace: cast self from graveyard with `RaiseCost$` (`AltCastPrice::NormalPlus`).
    let demonic_embrace = read26(
        "Name:Demonic Embrace\nManaCost:1 B B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ 3 | AddToughness$ 1 | AddKeyword$ Flying | AddType$ Demon\n\
         S:Mode$ Continuous | Affected$ Card.Self | MayPlay$ True | AffectedZone$ Graveyard | EffectZone$ Graveyard | RaiseCost$ PayLife<3> Discard<1/Card>\n",
    );
    let text = rendered(&demonic_embrace);
    assert!(text.contains("AltCastPrice::NormalPlus("), "{text}");
    assert!(text.contains("PayLife(3)"), "{text}");
    assert!(text.contains("zone = ZoneRef::Graveyard"), "{text}");

    // Filtered graveyard land permission (Deserts).
    let desert_gy = read26(
        "Name:DesertCrucible\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Desert.YouOwn | MayPlay$ True\n",
    );
    assert_eq!(
        desert_gy.abilities,
        [
            "static_ability!(Filter::HasSubtype(subtypes::land::DESERT), Modifier::PlayLandsFromGraveyard)"
        ]
    );

    // Self-graveyard land permission.
    let self_gy_land = read26(
        "Name:GraveyardLand\nTypes:Land\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True\n\
         A:AB$ Mana | Cost$ T | Produced$ C\n",
    );
    assert!(
        self_gy_land.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::PlayLandsFromGraveyard, zone = ZoneRef::Graveyard)"
        }),
        "{:?}",
        self_gy_land.abilities
    );

    // Refusals: finality counter description, non-redundant `ValidAfterStack$`,
    // opponent graveyard, self-graveyard land with `RaiseCost$`.
    for bad in [
        "Name:BadFinality\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Creature.YouOwn+nonLand | MayPlay$ True | Description$ You may cast creature spells from your graveyard. If you do, it enters with a finality counter.",
        "Name:BadVas\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Permanent.nonLand+YouOwn+cmcLE2 | ValidAfterStack$ Spell.cmcLE1 | MayPlay$ True",
        "Name:BadOppGy\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Creature.OppOwn+nonLand | MayPlay$ True",
        "Name:BadSelfLandCost\nTypes:Land\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | RaiseCost$ PayLife<2>",
    ] {
        assert!(refused26(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn saga_ward_convoke_wave14a_grant_flash_and_ward_blight() {
    // Raff Capashen: `Modifier::GrantFlash` for Historic spells.
    let raff = read26(
        "Name:Raff Capashen, Ship's Mage\nManaCost:2 W U\nTypes:Legendary Creature Human Wizard\nPT:3/3\n\
         K:Flash\nK:Flying\n\
         S:Mode$ CastWithFlash | ValidCard$ Card.Historic | ValidSA$ Spell | Caster$ You\n",
    );
    let text = rendered(&raff);
    assert!(
        text.contains("Modifier::GrantFlash { who: PlayerRel::You, filter:"),
        "{text}"
    );
    assert!(text.contains("Filter::Historic"), "{text}");

    // Yeva, Nature's Herald: `Creature.Green` -> `Modifier::GrantFlash`.
    let yeva = read26(
        "Name:Yeva, Nature's Herald\nManaCost:2 G G\nTypes:Legendary Creature Elf\nPT:4/4\n\
         K:Flash\n\
         S:Mode$ CastWithFlash | ValidCard$ Creature.Green | ValidSA$ Spell | Caster$ You\n",
    );
    let text = rendered(&yeva);
    assert!(
        text.contains("Modifier::GrantFlash { who: PlayerRel::You, filter:"),
        "{text}"
    );
    assert!(
        text.contains("ColorSet::from_slice(&[Color::Green])"),
        "{text}"
    );

    // Tidal Barracuda: `Caster$ Player` -> `PlayerRel::EachPlayer`.
    let barracuda = read26(
        "Name:Tidal Barracuda\nManaCost:3 U\nTypes:Creature\nPT:3/4\n\
         S:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell | Caster$ Player\n",
    );
    assert_eq!(
        barracuda.abilities,
        [
            "static_ability!(Filter::Any, Modifier::GrantFlash { who: PlayerRel::EachPlayer, filter: &Filter::Any })"
        ]
    );

    // `K:Ward:Blight<2>` lowers to `CostPart::PutCounterOnOther` with `-1/-1` counters.
    let ward_blight = read26(
        "Name:BlightWarder\nManaCost:2 B G\nTypes:Creature\nPT:3/3\n\
         K:Ward:Blight<2>\n",
    );
    let text = rendered(&ward_blight);
    assert!(
        text.contains(
            "CostPart::PutCounterOnOther { filter: &Filter::YOUR_CREATURE, kind: CounterKind::MinusOneMinusOne, n: 2 }"
        ),
        "{text}"
    );

    // Refusals: `Ward:Blight<0>`, `Ward:CollectEvidence<3>`, `Blight<2>` in `OptionalCost`.
    for bad in [
        "Name:BadWardZero\nTypes:Creature\nPT:2/2\nK:Ward:Blight<0>",
        "Name:BadWardCE\nTypes:Creature\nPT:2/2\nK:Ward:CollectEvidence<3>",
        "Name:BadOptBlight\nManaCost:1 B\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Blight<2>\n\
         A:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused26(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn saga_ward_convoke_wave26g_extra_svar_conditions() {
    // `extra_svar_condition_clause` additions (both positive and negated).
    for (svar_def, cmp, expected) in [
        (
            "Count$wasCastFromYourHandByYou.1.0",
            "GE1",
            "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceMatches(&Filter::CastFromHand)])",
        ),
        (
            "Count$Morbid.1.0",
            "EQ0",
            "Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))",
        ),
        ("Count$Raid.1.0", "GE1", "Condition::AttackedWithAtLeast(1)"),
        (
            "Count$AttackedThisTurn.1.0",
            "EQ1",
            "Condition::AttackedWithAtLeast(1)",
        ),
        ("Count$Revolt.1.0", "GE1", "Condition::Revolt"),
        (
            "Count$Threshold.1.0",
            "GE1",
            "Condition::GraveyardCountAtLeast(7)",
        ),
        (
            "Count$Delirium.1.0",
            "GE1",
            "Condition::GraveyardTypesAtLeast(4)",
        ),
        (
            "PlayerCountOpponents$HasPropertyIsCorrupted",
            "GE1",
            "Condition::OpponentPoisonAtLeast(3)",
        ),
        (
            "PlayerCountOpponents$HasPropertywasDealtDamageThisTurn",
            "GE1",
            "Condition::OpponentDealtDamageThisTurn",
        ),
    ] {
        let body = read26(&format!(
            "Name:CondTest\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
             S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 2 | CheckSVar$ X | SVarCompare$ {cmp}\n\
             SVar:X:{svar_def}\n"
        ));
        let text = rendered(&body);
        assert!(
            text.contains(expected),
            "expected `{expected}` for `{svar_def}` `{cmp}`, got:\n{text}"
        );
    }
}

#[test]
fn kicker_dual_and_inverted_conditions() {
    // Dual-kicker `ConditionDefined$ Self | ConditionPresent$ Card.kicked 1`
    // and `Count$Kicked2.1.0`.
    let dual_kicker = read26(
        "Name:DualKicker\nManaCost:1 W\nTypes:Sorcery\n\
         K:Kicker:U:B\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ DBK1\n\
         SVar:DBK1:DB$ GainLife | LifeAmount$ 2 | ConditionDefined$ Self | ConditionPresent$ Card.kicked 1 | SubAbility$ DBK2\n\
         SVar:DBK2:DB$ Mill | NumCards$ 2 | Defined$ You | ConditionCheckSVar$ K2 | ConditionSVarCompare$ GE1\n\
         SVar:K2:Count$Kicked2.1.0\n\
         Oracle:Kicker {U} and/or {B}\\nDraw a card. If this spell was kicked with its {U} kicker, you gain 2 life. If this spell was kicked with its {B} kicker, mill two cards.\n",
    );
    let text = rendered(&dual_kicker);
    assert!(text.contains("Condition::KickedWith(0)"), "{text}");
    assert!(text.contains("Condition::KickedWith(1)"), "{text}");

    // Inverted `Count$Kicked.0.1` and `Count$OptionalGenericCostPaid.0.1`.
    let inverted_kicker = read26(
        "Name:InvKicker\nManaCost:1 R\nTypes:Sorcery\n\
         K:Kicker:2\n\
         A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ NK | ConditionSVarCompare$ GE1\n\
         SVar:NK:Count$Kicked.0.1\n",
    );
    assert!(
        rendered(&inverted_kicker).contains("otherwise: &[Effect::draw(1)]"),
        "{}",
        rendered(&inverted_kicker)
    );

    let inverted_opt = read26(
        "Name:InvOpt\nManaCost:1 U\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 2\n\
         A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ NO | ConditionSVarCompare$ GE1\n\
         SVar:NO:Count$OptionalGenericCostPaid.0.1\n",
    );
    assert!(
        rendered(&inverted_opt)
            .contains("Condition::OptionalCostPaid, then: &[], otherwise: &[Effect::draw(1)]"),
        "{}",
        rendered(&inverted_opt)
    );

    // Broader `Count$TimesKicked` comparisons (`GT0`).
    let times_kicked_gt0 = read26(
        "Name:TimesGt0\nManaCost:1 G\nTypes:Sorcery\n\
         K:Multikicker:1\n\
         A:SP$ GainLife | LifeAmount$ 3 | ConditionCheckSVar$ TK | ConditionSVarCompare$ GT0\n\
         SVar:TK:Count$TimesKicked\n",
    );
    assert!(
        rendered(&times_kicked_gt0).contains("Effect::IfKicked { then: &[Effect::gain_life(3)]"),
        "{}",
        rendered(&times_kicked_gt0)
    );
}

#[test]
fn kicker_optional_and_mandatory_spell_cost_tokens() {
    // Spell cost tokens: `Forage` on `OptionalCost`, `RevealOrChoose<1/Dragon>`,
    // `Exile<1/Creature>`, `Discard<X/Card>`, `ExileFromGrave<X/Card>`,
    // `ExileFromGrave<4/Card>`.
    let forage_opt = read26(
        "Name:ForageSpell\nManaCost:1 G\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Forage\n\
         A:SP$ GainLife | LifeAmount$ 2\n",
    );
    assert!(
        forage_opt
            .optional_cost
            .as_deref()
            .is_some_and(|c| c.contains("Forage")),
        "{:?}",
        forage_opt.optional_cost
    );

    let behold_opt = read26(
        "Name:Dragon's Fire Shape\nManaCost:1 R\nTypes:Instant\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ RevealOrChoose<1/Dragon>\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3\n",
    );
    assert!(
        behold_opt
            .optional_cost
            .as_deref()
            .is_some_and(|c| c.contains("Behold("))
            && behold_opt.statics.contains("subtypes::creature::DRAGON"),
        "opt: {:?}\nstatics: {}",
        behold_opt.optional_cost,
        behold_opt.statics
    );

    let exile_cost = read26(
        "Name:ExileAdditional\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 W Exile<1/Creature> | NumCards$ 2\n",
    );
    assert!(
        exile_cost
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("ExilePermanent(")),
        "{:?}",
        exile_cost.mandatory_additional_costs
    );

    let discard_x = read26(
        "Name:DiscardXSpell\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 R Discard<X/Card> | NumCards$ X\n\
         SVar:X:Count$xPaid\n",
    );
    assert!(
        discard_x
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("DiscardX(")),
        "{:?}",
        discard_x.mandatory_additional_costs
    );

    let exile_gy_x = read26(
        "Name:ExileGyXSpell\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ GainLife | Cost$ 1 B ExileFromGrave<X/Card> | LifeAmount$ X\n\
         SVar:X:Count$xPaid\n",
    );
    assert!(
        exile_gy_x
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("ExileFromGraveyardX(")),
        "{:?}",
        exile_gy_x.mandatory_additional_costs
    );

    let exile_gy_4 = read26(
        "Name:ExileGy4Spell\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 B ExileFromGrave<4/Card> | NumCards$ 2\n",
    );
    assert_eq!(exile_gy_4.mandatory_additional_costs.len(), 4);
    assert!(
        exile_gy_4
            .mandatory_additional_costs
            .iter()
            .all(|c| c.contains("ExileFromGraveyard(")),
        "{:?}",
        exile_gy_4.mandatory_additional_costs
    );
}

#[test]
fn divided_damage_shatterskull_inline_expr_target_max_and_subability() {
    // Shatterskull Smashing front face (`DivisionAmount::XDoubleFrom(6)`).
    let shatterskull = read26(
        "Name:Shatterskull Smashing\nManaCost:X R R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ Y | ValidTgts$ Creature,Planeswalker | TargetMin$ 0 | TargetMax$ 2 | DividedAsYouChoose$ Y\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$Compare X LT6.X.Z\n\
         SVar:Z:Count$xPaid/Twice\n",
    );
    let text = rendered(&shatterskull);
    assert!(text.contains("DivisionAmount::XDoubleFrom(6)"), "{text}");
    assert!(text.contains("TargetReq::up_to("), "{text}");

    // Inline `Count$xPaid/Plus.2` and `Number$4`.
    let inline_plus = read26(
        "Name:InlinePlus\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ Y | ValidTgts$ Any | TargetMin$ 0 | TargetMax$ 3 | DividedAsYouChoose$ Y\n\
         SVar:Y:Count$xPaid/Plus.2\n",
    );
    assert!(
        rendered(&inline_plus).contains("DivisionAmount::XPlus(2)"),
        "{}",
        rendered(&inline_plus)
    );

    let number_svar = read26(
        "Name:NumberDiv\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ N | ValidTgts$ Any | TargetMin$ 0 | TargetMax$ 4 | DividedAsYouChoose$ N\n\
         SVar:N:Number$4\n",
    );
    assert!(
        rendered(&number_svar).contains("DivisionAmount::Fixed(4)"),
        "{}",
        rendered(&number_svar)
    );

    // `TargetMax$` as `Count$Valid` matching `ValidTgts$` (`u8::MAX`) + safe non-retargeting `SubAbility$`.
    let max_and_draw = read26(
        "Name:DivAndDraw\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ 3 | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ M | DividedAsYouChoose$ 3 | SubAbility$ DBDraw\n\
         SVar:M:Count$Valid Creature\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1\n",
    );
    let text = rendered(&max_and_draw);
    assert!(
        text.contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), u8::MAX)"),
        "{text}"
    );
    assert!(text.contains("Effect::DealDamageDivided"), "{text}");
    assert!(text.contains("Effect::draw(1)"), "{text}");

    // `PutCounter` divided with `Count$LastStateBattlefield` and matching `TargetMax$`.
    let div_counters_bf = read26(
        "Name:DivCountersBf\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ M | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X\n\
         SVar:M:Count$Valid Creature.YouCtrl\n\
         SVar:X:Count$LastStateBattlefield Creature.YouCtrl\n",
    );
    let text = rendered(&div_counters_bf);
    assert!(
        text.contains(
            "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::CountOf"
        ),
        "{text}"
    );
    assert!(text.contains("ZoneSel::Battlefield"), "{text}");
    assert!(text.contains("u8::MAX"), "{text}");

    // Unscoped `Count$LastStateGraveyardWithFallback Creature` (`ZoneSel::GraveyardAll`).
    let div_counters_gy_all = read26(
        "Name:DivCountersGyAll\nManaCost:2 B G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ X | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X\n\
         SVar:X:Count$LastStateGraveyardWithFallback Creature\n",
    );
    assert!(
        rendered(&div_counters_gy_all).contains("ZoneSel::GraveyardAll"),
        "{}",
        rendered(&div_counters_gy_all)
    );

    // Refusals: retargeting `SubAbility$`, mismatched `TargetMax$`.
    for bad in [
        "Name:BadRetarget\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ 3 | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 3 | DividedAsYouChoose$ 3 | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Artifact\n",
        "Name:BadMaxMismatch\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | NumDmg$ 3 | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ M | DividedAsYouChoose$ 3\n\
         SVar:M:Count$Valid Artifact\n",
    ] {
        assert!(refused26(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn paper_mtg_strict_refusals() {
    // Zero Alchemy (`Intensity`, `Conjure`, `Seek`, `Perpetual`, `Specialize`, `Heist`, `Boon`, `MakeCard`)
    // and zero Un-set / Acorn / Sticker / Planechase mechanics.
    for non_paper in [
        "Name:BadIntensity\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature.cmcLEX | NumDmg$ 2\n\
         SVar:X:Count$Intensity\n",
        "Name:BadSeek\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Seek | Type$ Creature\n",
        "Name:BadConjure\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ MakeCard | Name$ Lightning Bolt\n",
        "Name:BadSticker\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         K:Ward:Sticker<1>\n",
        "Name:BadPlaneswalk\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ PlaneswalkedTo | Execute$ E\n\
         SVar:E:DB$ Draw | NumCards$ 1\n",
    ] {
        assert!(
            refused26(non_paper),
            "must refuse non-Paper script:\n{non_paper}"
        );
    }
}
