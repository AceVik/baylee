//! Codegen Wave 24c: filter atoms, filter base qualifiers, graveyard target
//! specs, `ChooseColor` animation variants, and strict Paper-MTG refusals.

use super::*;

fn rendered(body: &CardBody) -> String {
    format!("{}\n{}", body.statics, body.abilities.join("\n"))
}

#[test]
fn cmc_var_atoms_map_only_exact_dsl_predicates_and_refuse_others() {
    let x_le = read(
        "Name:XLe\nManaCost:X B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.cmcLEX\n\
         SVar:X:Count$xPaid\n",
    );
    assert!(
        rendered(&x_le).contains("Filter::CmcAtMostX"),
        "{}",
        rendered(&x_le)
    );

    let x_eq = read(
        "Name:XEq\nManaCost:X U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.cmcEQX\n\
         SVar:X:Count$xPaid\n",
    );
    assert!(
        rendered(&x_eq).contains("Filter::CmcExactlyX"),
        "{}",
        rendered(&x_eq)
    );

    let snow = read(
        "Name:SnowSweep\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Creature.cmcLEX\n\
         SVar:X:Count$CastSnowManaSpent\n",
    );
    assert!(
        rendered(&snow).contains("Filter::CmcAtMostSnowManaSpent"),
        "{}",
        rendered(&snow)
    );

    let colors = read(
        "Name:ColorSweep\nManaCost:X W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Permanent.nonLand+cmcLEX\n\
         SVar:X:Count$ColorsSpent\n",
    );
    assert!(
        rendered(&colors).contains("Filter::CmcAtMostColorsSpent"),
        "{}",
        rendered(&colors)
    );

    let counters = read(
        "Name:CounterCheck\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Card.cmcEQX | TriggerZones$ Battlefield | Execute$ TrigD\n\
         SVar:TrigD:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$CardCounters.CHARGE\n",
    );
    assert!(
        rendered(&counters).contains("Filter::CmcEqualsCountersOnSource(CounterKind::Charge)"),
        "{}",
        rendered(&counters)
    );

    let same_mv = read(
        "Name:SameMv\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Artifact.cmcEQX\n\
         SVar:X:Count$CardManaCost\n",
    );
    assert!(
        rendered(&same_mv).contains("Filter::SameManaValueAsSource"),
        "{}",
        rendered(&same_mv)
    );

    // Unrepresented or semantically mismatched `SVar:X` definitions must stay refused.
    for bad in [
        // ETB converge: `obj.paid` is cleared on Stack->Battlefield.
        "Name:Bad4\nManaCost:4\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ Destroy | ValidTgts$ Artifact.cmcLEX\nSVar:X:Count$ColorsSpent\n",
        // Self-sacrifice with `Count$CardCounters.*`: `eval.rs` reads live counters (0).
        "Name:Bad5\nManaCost:2\nTypes:Artifact\n\
         A:AB$ DestroyAll | Cost$ T Sac<1/CARDNAME> | ValidCards$ Permanent.nonLand+cmcEQX\n\
         SVar:X:Count$CardCounters.CHARGE\n",
        // `SpellCast` self-trigger does not populate `x_value` like `Trigger::ETB`.
        "Name:Bad6\nManaCost:X G G\nTypes:Creature\nPT:0/0\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ Destroy | ValidTgts$ Artifact.cmcLEX\nSVar:X:Count$xPaid\n",
    ] {
        assert!(refused(bad), "expected refusal:\n{bad}");
    }
}

#[test]
fn graveyard_below_event_and_owner_targets_transcode_and_guard_etb() {
    let trawler = read(
        "Name:Scrap Trawler\nManaCost:3\nTypes:Artifact Creature\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Artifact.Other+YouCtrl | TriggerZones$ Battlefield | Secondary$ True | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | ValidTgts$ Artifact.cmcLTX+YouCtrl | Origin$ Graveyard | Destination$ Hand\n\
         SVar:X:TriggeredCard$CardManaCost\n\
         Oracle:Whenever Scrap Trawler or another artifact you control is put into a graveyard from the battlefield, return to your hand target artifact card in your graveyard with lesser mana value.\n",
    );
    assert!(
        trawler.abilities.iter().all(|a| a
            .contains("TargetSpec::CardInGraveyardBelowEvent(&Filter::ARTIFACT, PlayerRel::You)")),
        "{:?}",
        trawler.abilities
    );

    let dragster = read(
        "Name:Deluxe Raider\nManaCost:4 U\nTypes:Creature\nPT:4/3\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ TrigExile | CombatDamage$ True\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Instant.OwnedBy TriggeredTarget,Sorcery.OwnedBy TriggeredTarget | Origin$ Graveyard | Destination$ Exile\n",
    );
    assert!(
        dragster.abilities.join("").contains(
            "TargetSpec::CardInGraveyard(&Filter::INSTANT_OR_SORCERY, PlayerRel::DamagedPlayer)"
        ),
        "{:?}",
        dragster.abilities
    );

    let pickpocket = read(
        "Name:Pickpocket\nManaCost:B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigCast\n\
         SVar:TrigCast:DB$ Play | ValidTgts$ Permanent.nonLand+ControlledBy TriggeredTarget | TgtZone$ Graveyard | ValidSA$ Spell | Optional$ True | ManaConversion$ AnyType->AnyType\n",
    );
    assert!(
        pickpocket
            .abilities
            .join("")
            .contains("PlayerRel::DamagedPlayer"),
        "{:?}",
        pickpocket.abilities
    );

    let attack_gy = read(
        "Name:Grave Raider\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.OwnedBy TriggeredDefendingPlayer | Origin$ Graveyard | Destination$ Exile\n",
    );
    assert!(
        attack_gy
            .abilities
            .join("")
            .contains("TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::DefendingPlayer)"),
        "{:?}",
        attack_gy.abilities
    );

    // Non-suffix `ControlledBy TriggeredTarget` on battlefield targets.
    let mid_ctrl = read(
        "Name:MidCtrl\nManaCost:3 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigD\n\
         SVar:TrigD:DB$ Destroy | ValidTgts$ Creature.ControlledBy TriggeredTarget+nonBlack\n",
    );
    assert!(
        mid_ctrl
            .abilities
            .join("")
            .contains("TargetSpec::ObjectOfEventPlayer("),
        "{:?}",
        mid_ctrl.abilities
    );

    // ETB trigger does NOT populate `state.ltb_mana_values` — must refuse!
    assert!(refused(
        "Name:BadEtbBelow\nManaCost:3\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | ValidTgts$ Creature.cmcLTX+YouOwn | Origin$ Graveyard | Destination$ Hand\n\
         SVar:X:TriggeredCard$CardManaCost\n\
         Oracle:When BadEtbBelow enters, return target creature card with lesser mana value from your graveyard to your hand.\n"
    ));

    // Mixed `cmcLTX` across comma-separated alternatives must refuse.
    assert!(refused(
        "Name:BadMixed\nManaCost:3\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | ValidTgts$ Artifact.cmcLTX+YouOwn,Creature.YouOwn | Origin$ Graveyard | Destination$ Hand\n\
         SVar:X:TriggeredCard$CardManaCost\n\
         Oracle:When BadMixed dies, return target artifact or creature card from your graveyard to your hand.\n"
    ));

    // `OwnedBy TriggeredTarget` outside a damage trigger must refuse.
    assert!(refused(
        "Name:BadOwnerTrigger\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.OwnedBy TriggeredTarget | Origin$ Graveyard | Destination$ Exile\n"
    ));
}

#[test]
fn filter_base_qualifiers_and_strict_refusals() {
    let pillar = read(
        "Name:Pillar\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.MultiColor\n",
    );
    assert!(
        pillar.statics.contains("Filter::Monocolored"),
        "{}",
        pillar.statics
    );

    let lotus = read(
        "Name:CmdLotus\nManaCost:0\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any | Amount$ 3 | RestrictValid$ Spell.IsCommander+YouOwn\n",
    );
    assert!(
        lotus.statics.contains("Filter::IsCommander")
            && lotus.statics.contains("Filter::OwnedByYou"),
        "{}",
        lotus.statics
    );

    let shrine = read(
        "Name:ColorlessShrine\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 2 | RestrictValid$ Spell.Colorless\n",
    );
    assert!(
        rendered(&shrine).contains("Filter::IsColorless"),
        "{}",
        rendered(&shrine)
    );

    let varmint = read(
        "Name:Varmint\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         A:AB$ Mana | Cost$ T PayLife<1> | Produced$ Any | Amount$ 2 | RestrictValid$ Spell.YouDontOwn\n",
    );
    assert!(
        rendered(&varmint).contains("Filter::Not(&Filter::OwnedByYou)"),
        "{}",
        rendered(&varmint)
    );

    let mv4 = read(
        "Name:Mv4Mana\nManaCost:2 U\nTypes:Creature\nPT:1/3\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | Amount$ 2 | RestrictValid$ Spell.cmcGE4\n",
    );
    assert!(
        rendered(&mv4).contains("Filter::CmcAtLeast(4)"),
        "{}",
        rendered(&mv4)
    );

    let devoid_land = read(
        "Name:DevoidLand\nTypes:Land\n\
         A:AB$ Mana | Cost$ T PayLife<1> | Produced$ Any | RestrictValid$ Spell.withDevoid\n",
    );
    assert!(
        rendered(&devoid_land).contains("Filter::HasKeyword(KeywordSet::DEVOID)"),
        "{}",
        rendered(&devoid_land)
    );

    // Non-object filter bases must remain strictly refused.
    for bad_base in [
        "Player",
        "Emblem",
        "Any",
        "You",
        "Opponent",
        "Targeted",
        "lowestCMC",
    ] {
        assert!(
            refused(&format!(
                "Name:BadBase\nManaCost:1 R\nTypes:Sorcery\n\
                 A:SP$ DestroyAll | ValidCards$ {bad_base}\n"
            )),
            "expected refusal for base `{bad_base}`"
        );
    }
}

#[test]
fn keyword_atoms_spaced_negated_and_enforced_bits() {
    assert_eq!(
        keyword_atom("withFirst Strike").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::FIRST_STRIKE)")
    );
    assert_eq!(
        keyword_atom("withDouble Strike").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::DOUBLE_STRIKE)")
    );
    assert_eq!(
        keyword_atom("withoutFirst Strike").as_deref(),
        Some("Filter::Not(&Filter::HasKeyword(KeywordSet::FIRST_STRIKE))")
    );
    assert_eq!(
        keyword_atom("!withDefender").as_deref(),
        Some("Filter::Not(&Filter::HasKeyword(KeywordSet::DEFENDER))")
    );
    assert_eq!(
        keyword_atom("hasKeywordFlash").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::FLASH)")
    );
    assert_eq!(
        keyword_atom("hasKeywordLandwalk:Island").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::ISLANDWALK)")
    );
    assert_eq!(
        keyword_atom("!hasKeywordLandwalk:Island").as_deref(),
        Some("Filter::Not(&Filter::HasKeyword(KeywordSet::ISLANDWALK))")
    );
    assert_eq!(
        keyword_atom("withAwaken").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::AWAKEN)")
    );
    assert_eq!(
        keyword_atom("withFreerunning").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::FREERUNNING)")
    );
    assert_eq!(
        keyword_atom("withUmbra Armor").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::UMBRA_ARMOR)")
    );
    assert_eq!(
        keyword_atom("withProvoke").as_deref(),
        Some("Filter::HasKeyword(KeywordSet::PROVOKE)")
    );
    // Unenforced / parameterized keywords with no bit stay refused.
    for unknown in [
        "withCascade",
        "withFlashback",
        "withMorph",
        "withToxic",
        "withCycling",
        "hasKeywordConvoke",
    ] {
        assert_eq!(keyword_atom(unknown), None, "{unknown}");
    }

    let leviathan = read(
        "Name:Stormtide\nManaCost:5 U U U\nTypes:Creature\nPT:8/8\n\
         K:Landwalk:Island\n\
         S:Mode$ CantAttack | ValidCard$ Creature.withoutFlying+!hasKeywordLandwalk:Island\n",
    );
    assert!(
        rendered(&leviathan).contains("Filter::Not(&Filter::HasKeyword(KeywordSet::ISLANDWALK))"),
        "{}",
        rendered(&leviathan)
    );
}

#[test]
fn wave24c_filter_atoms_and_oracle_scoping() {
    // Generalized source power comparison (`powerLTY`, `powerLTZ`, self-trigger `TriggeredCard$CardPower`).
    let lena = read(
        "Name:Lena\nManaCost:4 W W\nTypes:Legendary Creature\nPT:3/3\n\
         A:AB$ PumpAll | Cost$ Sac<1/CARDNAME> | ValidCards$ Creature.YouCtrl+powerLTY | KW$ Indestructible\n\
         SVar:Y:Count$CardPower\n",
    );
    assert!(
        rendered(&lena).contains("Filter::PowerLessThanSourcePower"),
        "{}",
        rendered(&lena)
    );

    let launderer = read(
        "Name:Launderer\nManaCost:2 B B\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigR\n\
         SVar:TrigR:DB$ ChangeZone | ValidTgts$ Creature.powerLEX+YouOwn+Other | Origin$ Graveyard | Destination$ Battlefield\n\
         SVar:X:TriggeredCard$CardPower\n",
    );
    assert!(
        rendered(&launderer).contains("Filter::PowerAtMostSourcePower"),
        "{}",
        rendered(&launderer)
    );

    // `IsNotChosenType`
    let fear = read(
        "Name:Crippling Fear\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ DBPumpOther\n\
         SVar:DBPumpOther:DB$ PumpAll | ValidCards$ Creature.IsNotChosenType | NumAtt$ -3 | NumDef$ -3\n",
    );
    assert!(
        rendered(&fear).contains("Filter::Not(&Filter::MatchesChosenTypeOfSource)"),
        "{}",
        rendered(&fear)
    );

    // `!named<Name>` and `;` comma-escaping
    let legionnaire = read(
        "Name:Akron Legionnaire\nManaCost:6 W W\nTypes:Creature\nPT:8/4\n\
         S:Mode$ CantAttack | ValidCard$ Creature.YouCtrl+nonArtifact+!namedAkron Legionnaire\n",
    );
    assert!(
        rendered(&legionnaire).contains("Filter::Not(&Filter::Named(\"Akron Legionnaire\"))"),
        "{}",
        rendered(&legionnaire)
    );
    assert_eq!(
        filter("Creature.!namedSquee; Goblin Nabob"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Named(\"Squee, Goblin Nabob\"))])"
    );

    // `!`-prefixed supertypes, types, colors, and subtypes
    assert_eq!(
        filter("Card.!Legendary"),
        "Filter::Not(&Filter::HasSupertype(SupertypeSet::LEGENDARY))"
    );
    assert_eq!(filter("Card.!Land"), "Filter::NONLAND");
    assert_eq!(
        filter("Creature.!Black+!Goblin"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))), Filter::Not(&Filter::HasSubtype(subtypes::creature::GOBLIN))])"
    );

    // `blockingSelf`, `DefendingPlayerCtrl`, `lowestCMC`, `ControllerControlsIsland`, `setATQ`/`setARN`, `AllColors`, `Damaged`, `sharesCreatureTypeWith Valid`
    assert_eq!(
        filter("Creature.blockingSelf"),
        "Filter::And(&[Filter::CREATURE, Filter::BlockingSource])"
    );
    assert_eq!(
        filter("Creature.DefendingPlayerCtrl"),
        "Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer])"
    );
    assert_eq!(
        filter("Permanent.nonLand+lowestCMC"),
        "Filter::LowestManaValueOnBattlefield(&Filter::NONLAND)"
    );
    assert_eq!(
        filter("Creature.ControllerControlsIsland"),
        "Filter::And(&[Filter::CREATURE, Filter::ControllerControls(&Filter::HasSubtype(subtypes::land::ISLAND))])"
    );
    assert_eq!(
        filter("Permanent.!token+setATQ"),
        "Filter::And(&[Filter::Not(&Filter::IsToken), Filter::NameOriginallyPrintedIn(OriginalExpansion::Antiquities)])"
    );
    assert_eq!(
        filter("Permanent.setARN"),
        "Filter::NameOriginallyPrintedIn(OriginalExpansion::ArabianNights)"
    );
    assert!(filter("Creature.!AllColors").contains("Color::Green"));
    assert_eq!(
        filter("Creature.Damaged"),
        "Filter::And(&[Filter::CREATURE, Filter::DealtDamageToSourceThisTurn])"
    );
    assert_eq!(
        filter("Card.Creature+!sharesCreatureTypeWith Valid Creature.YouCtrl"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::SharesCreatureTypeWith(&Filter::YOUR_CREATURE))])"
    );
}

#[test]
fn wave24c_source_color_and_spell_cast_atoms() {
    // `RedSource` on `DamageDealtOnce` and Oracle-scoped `RedSource` on Flame of Keld.
    let red_terror = read(
        "Name:The Red Terror\nManaCost:3 R\nTypes:Legendary Creature\nPT:4/3\n\
         T:Mode$ DamageDealtOnce | TriggerZones$ Battlefield | ValidSource$ Card.RedSource+YouCtrl | ValidTarget$ Player,Permanent | Execute$ TrigP\n\
         SVar:TrigP:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1\n",
    );
    assert!(
        red_terror.statics.contains("Color::Red")
            && red_terror.statics.contains("Filter::ControlledByYou"),
        "{}",
        red_terror.statics
    );

    let flame = read(
        "Name:The Flame of Keld\nManaCost:1 R\nTypes:Enchantment Saga\n\
         K:Chapter:3:TrigDiscard,TrigDraw,TrigEffect\n\
         SVar:TrigDiscard:DB$ Discard | Mode$ Hand | Defined$ You\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 2\n\
         SVar:TrigEffect:DB$ Effect | ReplacementEffects$ FlameEvt\n\
         SVar:FlameEvt:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Card.RedSource | ValidTarget$ Permanent,Player | ReplaceWith$ DmgPlus2\n\
         SVar:DmgPlus2:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/Plus.2\n\
         Oracle:I — Discard your hand.\\nII — Draw two cards.\\nIII — If a red source you control would deal damage to a permanent or player this turn, it deals that much damage plus 2 to that permanent or player instead.\n",
    );
    assert!(
        flame.statics.contains("Color::Red") && flame.statics.contains("Filter::ControlledByYou"),
        "{}",
        flame.statics
    );

    // `!wasCastFromTheirHand` on `SpellCast` (Ghostly Pilferer).
    let pilferer = read(
        "Name:Pilferer\nManaCost:1 U\nTypes:Creature\nPT:2/1\n\
         T:Mode$ SpellCast | ValidCard$ Card.!wasCastFromTheirHand | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigD\n\
         SVar:TrigD:DB$ Draw\n",
    );
    assert!(
        pilferer
            .statics
            .contains("Filter::Not(&Filter::CastFromHand)"),
        "{}",
        pilferer.statics
    );
}

#[test]
fn choose_color_animate_or_colors_attached_and_subability() {
    let dream_coat = read(
        "Name:Dream Coat\nManaCost:U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         A:AB$ ChooseColor | Cost$ 0 | Defined$ You | OrColors$ True | ActivationLimit$ 1 | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | Colors$ ChosenColor | OverwriteColors$ True | Defined$ Enchanted | Duration$ Permanent\n",
    );
    assert!(
        dream_coat.abilities.join("").contains(
            "Effect::ChooseColors { subject: ColorSubject::Attached, max: 5, duration: Duration::Indefinitely }"
        ),
        "{:?}",
        dream_coat.abilities
    );

    let quickchange = read(
        "Name:Quickchange\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChooseColor | Defined$ You | OrColors$ True | SubAbility$ Animate\n\
         SVar:Animate:DB$ Animate | ValidTgts$ Creature | Colors$ ChosenColor | OverwriteColors$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         Oracle:Target creature becomes the color or colors of your choice until end of turn.\\nDraw a card.\n",
    );
    let qc = quickchange.abilities.join("");
    assert!(
        qc.contains(
            "Effect::ChooseColors { subject: ColorSubject::Target, max: 5, duration: Duration::UntilEndOfTurn }, Effect::draw(1)"
        ),
        "{qc}"
    );
    assert!(qc.contains("TargetSpec::Object(&Filter::CREATURE)"), "{qc}");

    // Swirling Spriggan: Oracle restricts to "Target creature you control".
    let spriggan = read(
        "Name:Swirling Spriggan\nManaCost:2 G G\nTypes:Creature\nPT:3/3\n\
         A:AB$ ChooseColor | Cost$ GU GU | Defined$ You | OrColors$ True | SubAbility$ Animate\n\
         SVar:Animate:DB$ Animate | ValidTgts$ Creature | Colors$ ChosenColor | OverwriteColors$ True\n\
         Oracle:{G/U}{G/U}: Target creature you control becomes the color or colors of your choice until end of turn.\n",
    );
    assert!(
        spriggan
            .abilities
            .join("")
            .contains("TargetSpec::Object(&Filter::YOUR_CREATURE)"),
        "{:?}",
        spriggan.abilities
    );

    // Prevention `ChooseColor` does not accept `OrColors$ True`.
    assert!(refused(
        "Name:BadOrColorPrevent\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChooseColor | Defined$ You | OrColors$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrev\n\
         SVar:RPrev:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Card.ChosenColor | Prevent$ True\n"
    ));
}

#[test]
fn strict_refusals_for_ambiguous_atoms_and_non_paper_scripts() {
    for bad in [
        // `YouCtrl$GreatestCardToughness` has no max-toughness `Filter`/`Amount`.
        "Name:BadGreatest\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardToughness\n",
        // `AddKeyword$ Flash` on stack static stays refused (not a stack-live keyword).
        "Name:BadWasCastStatic\nManaCost:3 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Instant.wasCast | AffectedZone$ Stack | AddKeyword$ Flash\n",
        // Drannith Magistrate: `CantBeCast` evaluates pre-cast where `CastHistory` is default.
        "Name:BadDrannith\nManaCost:1 W\nTypes:Creature\nPT:1/3\n\
         S:Mode$ CantBeCast | ValidCard$ Card.!wasCastFromTheirHand | Caster$ Opponent\n",
        // `IsNotChosenType` without any chosen creature/card type.
        "Name:BadNoChosenType\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Creature.IsNotChosenType\n",
        // `powerLTY` with non-source-power SVar or non-self `TriggeredCard$CardPower`.
        "Name:BadPowerVar\nManaCost:2 G\nTypes:Creature\nPT:3/3\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Creature.powerLTY\n\
         SVar:Y:Count$Valid Creature.YouCtrl\n",
        "Name:BadOtherTrigPower\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ E\n\
         SVar:E:DB$ Destroy | ValidTgts$ Creature.powerLTY\n\
         SVar:Y:TriggeredCard$CardPower\n",
        // `lowestCMC` in a card zone (not battlefield).
        "Name:BadLowestInGrave\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouOwn+lowestCMC\n",
        // Alchemy, Un-Set/Acorn, and Planechase scripts stay strictly refused.
        "Name:BadAlchemySeek\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Seek | Type$ Creature\n",
        "Name:BadAlchemyConjure\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ MakeCard | Conjure$ True | Name$ Lightning Bolt | Zone$ Hand\n",
        "Name:BadUnSticker\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         K:StickerSheet\n",
        "Name:BadUnSilver\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Permanent.BorderColorSilver\n",
        "Name:BadPlanechase\nTypes:Plane\n\
         T:Mode$ PlaneswalkedTo | Execute$ E\n\
         SVar:E:DB$ Draw\n",
    ] {
        assert!(refused(bad), "expected refusal:\n{bad}");
    }
}
