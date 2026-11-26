//! Wave 19 post-engine tests for `OppOwn`, `DistinctNames`,
//! `EnteredCurrentZoneThisTurn`, `ExiledByYou`, Hand-zone grants, off-board
//! madness, `Mayhem`/`Beam me up`, and `ZonePlayGrant` companion riders.

use super::*;

#[test]
fn opp_own_and_entered_this_turn_filters() {
    assert_eq!(
        filter("Card.OppOwn"),
        "Filter::OwnedBy(PlayerRel::EachOpponent)"
    );
    assert_eq!(
        filter("Permanent.YouOwn+ThisTurnEnteredFrom_Battlefield"),
        "Filter::And(&[Filter::OwnedByYou, Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield))])"
    );
    assert_eq!(
        filter("Creature.!token+ThisTurnEnteredFrom_Exile"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken), Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Exile))])"
    );
    assert_eq!(
        filter("Card.!ThisTurnEnteredFrom_Graveyard"),
        "Filter::Not(&Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Graveyard)))"
    );

    let faiths_reward = read(
        "Name:Faith's Reward\nManaCost:3 W\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | \
         Defined$ ValidGraveyard Permanent.YouOwn+ThisTurnEnteredFrom_Battlefield",
    );
    let joined = faiths_reward.abilities.join("\n");
    assert!(
        joined.contains("Effect::YourGraveyardToBattlefield"),
        "{joined}"
    );
    assert!(
        faiths_reward
            .statics
            .contains("EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield))"),
        "{}",
        faiths_reward.statics
    );
}

#[test]
fn distinct_names_amounts() {
    let bf = read(
        "Name:DistinctBf\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$DifferentCardNames",
    );
    let joined_bf = bf.abilities.join("\n");
    assert!(
        joined_bf.contains("Amount::DistinctNames { zone: ZoneSel::Battlefield, filter:")
            && joined_bf.contains("Filter::YOUR_CREATURE"),
        "{joined_bf}"
    );

    let gy = read(
        "Name:DistinctGy\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn$DifferentCardNames",
    );
    let joined_gy = gy.abilities.join("\n");
    assert!(
        joined_gy.contains("Amount::DistinctNames {")
            && joined_gy.contains("zone: ZoneSel::GraveyardOf(PlayerRel::You)"),
        "{joined_gy}"
    );

    // Refuses unsupported suffix on DistinctNames or opponent single-graveyard probe.
    assert!(refused(
        "Name:BadSingleOppGy\nManaCost:U\nTypes:Sorcery\n\
         Oracle:Draw a card for each different card name in target opponent's graveyard.\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard Card.OppOwn$DifferentCardNames"
    ));
}

#[test]
fn hand_zone_and_offboard_madness_grants() {
    let telepathy = read(
        "Name:Telepathy\nManaCost:U\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedZone$ Hand | Affected$ Card.OppOwn | MayLookAt$ Player",
    );
    assert_eq!(
        telepathy.abilities,
        ["static_ability!(Filter::Any, Modifier::RevealOpponentsHands)"]
    );

    let aminatou = read(
        "Name:Aminatou, Veil Piercer\nManaCost:1 W U B\nTypes:Legendary Creature Human Wizard\nPT:2/4\n\
         S:Mode$ Continuous | Affected$ Card.Enchantment+YouOwn | AffectedZone$ Hand | AddKeyword$ Miracle:ManaCost:4",
    );
    assert_eq!(
        aminatou.abilities,
        [
            "static_ability!(Filter::And(&[Filter::ENCHANTMENT, Filter::OwnedByYou]), Modifier::GrantMiracle(MiracleGrant::ManaCostLessGeneric(4)))"
        ]
    );

    let lorehold = read(
        "Name:Lorehold, the Historian\nManaCost:3 R W\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.Instant+YouOwn,Card.Sorcery+YouOwn | AffectedZone$ Hand | AddKeyword$ Miracle:2",
    );
    assert!(
        lorehold.abilities[0]
            .contains("Modifier::GrantMiracle(MiracleGrant::Fixed(mana!(\"{2}\")))"),
        "{:?}",
        lorehold.abilities
    );

    let dream = read(
        "Name:Dream Devourer\nManaCost:1 B\nTypes:Creature Demon Cleric\nPT:0/3\n\
         S:Mode$ Continuous | Affected$ Card.nonLand+YouOwn+withoutForetell | AffectedZone$ Hand | AddKeyword$ Foretell",
    );
    assert_eq!(
        dream.abilities,
        [
            "static_ability!(Filter::And(&[Filter::NONLAND, Filter::OwnedByYou]), Modifier::GrantForetell { reduce_generic: 2 })"
        ]
    );

    let tannuk = read(
        "Name:Tannuk, Steadfast Second\nManaCost:2 R R\nTypes:Legendary Creature Kavu Pilot\nPT:3/5\n\
         S:Mode$ Continuous | Affected$ Artifact.YouOwn,Creature.Red+YouOwn | EffectZone$ Battlefield | AffectedZone$ Hand | AddKeyword$ Warp:2 R",
    );
    assert!(
        tannuk.abilities[0].contains("Modifier::GrantWarp(cost!(\"{2}{R}\"))"),
        "{:?}",
        tannuk.abilities
    );

    let ashling_hand = read(
        "Name:AshlingGrant\nManaCost:2 R\nTypes:Creature Elemental\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Permanent.Wizard+YouOwn | AffectedZone$ Hand | AddKeyword$ Evoke:4",
    );
    assert!(
        ashling_hand.abilities[0].contains("Modifier::GrantEvoke(cost!(\"{4}\"))"),
        "{:?}",
        ashling_hand.abilities
    );

    let gorger = read(
        "Name:Falkenrath Gorger\nManaCost:R\nTypes:Creature Vampire Berserker\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn+Goblin | AffectedZone$ Hand,Library,Graveyard,Exile,Command,Stack | AddKeyword$ Madness:ManaCost",
    );
    assert!(
        gorger.abilities[0].contains("Modifier::GrantMadnessManaCost")
            && gorger.abilities[0].contains("Filter::InZone(ZoneRef::NotBattlefield)"),
        "{:?}",
        gorger.abilities
    );

    // Hand Cycling/Ninjutsu and bare `withoutForetell` remain honestly refused.
    assert!(refused(
        "Name:BadHandCycling\nManaCost:1 R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn | AffectedZone$ Hand | AddKeyword$ Cycling:R"
    ));
    assert!(refused(
        "Name:BadWithoutForetell\nManaCost:1 B\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.nonLand+YouOwn+withoutForetell | AddKeyword$ Flying"
    ));
}

#[test]
fn graveyard_mayhem_beam_me_up_and_zone_play_riders() {
    let green_goblin = read(
        "Name:Green Goblin\nManaCost:1 U B R\nTypes:Legendary Creature Goblin\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Card.YouOwn+nonLand | AffectedZone$ Graveyard | AddKeyword$ Mayhem:CardManaCost",
    );
    assert_eq!(
        green_goblin.abilities,
        [
            "static_ability!(Filter::And(&[Filter::OwnedByYou, Filter::NONLAND]), Modifier::GrantsMayhemManaCost)"
        ]
    );

    let scotty = read(
        "Name:Scotty, Miracle Worker\nManaCost:1 U U\nTypes:Legendary Creature Human Artificer\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Instant.YouOwn,Sorcery.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Beam me up | Condition$ PlayerTurn | Phases$ Main1,Main2\n\
         S:Mode$ CantBeBeamedUp | ValidCard$ Card.Self | ValidSA$ Spell.KeywordSource | Secondary$ True",
    );
    assert_eq!(scotty.abilities.len(), 1);
    assert!(
        scotty.abilities[0]
            .contains("Modifier::GrantsBeamMeUpManaCost { cant_return_source: true }")
            && scotty.abilities[0].contains("Condition::DuringStep(StepKind::FirstMain)"),
        "{:?}",
        scotty.abilities
    );
    assert!(refused(
        "Name:BareCantBeam\nManaCost:U\nTypes:Creature\nPT:1/1\n\
         S:Mode$ CantBeBeamedUp | ValidCard$ Card.Self | ValidSA$ Spell.KeywordSource | Secondary$ True"
    ));

    let maestros = read(
        "Name:Maestros Ascendancy\nManaCost:U B R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | Condition$ PlayerTurn | ReplaceGraveyard$ Exile | MayPlay$ True | RaiseCost$ Sac<1/Creature> | MayPlayLimit$ 1 | EffectZone$ Battlefield | AffectedZone$ Graveyard",
    );
    assert!(
        maestros.abilities[0].contains("Modifier::ZonePlayGrant")
            && maestros.abilities[0].contains("ZonePlayCastRider::ExileInsteadOfGraveyard")
            && maestros.abilities[0].contains("once_per_turn: true"),
        "{:?}",
        maestros.abilities
    );

    let noctis = read(
        "Name:Noctis, Prince of Lucis\nManaCost:1 W U B\nTypes:Legendary Creature\nPT:4/3\n\
         S:Mode$ Continuous | Affected$ Artifact.YouCtrl | MayPlay$ True | EffectZone$ Battlefield | AffectedZone$ Graveyard | RaiseCost$ PayLife<3>\n\
         T:Mode$ SpellCast | ValidCard$ Card.CastSa Spell.MayPlaySource | Execute$ TrigEffect | Static$ True | TriggerZones$ Battlefield\n\
         SVar:TrigEffect:DB$ Effect | ReplacementEffects$ ReMoved | RememberObjects$ TriggeredCard\n\
         SVar:ReMoved:Event$ Moved | Origin$ Stack | ValidCard$ Card.IsRemembered | Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBCounter\n\
         SVar:ETBCounter:DB$ PutCounter | ETB$ True | Defined$ ReplacedCard | CounterType$ FINALITY | CounterNum$ 1 | SubAbility$ RemoveSelf\n\
         SVar:RemoveSelf:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile",
    );
    assert_eq!(noctis.abilities.len(), 1);
    assert!(
        noctis.abilities[0].contains("ZonePlayCastRider::EntryFinality"),
        "{:?}",
        noctis.abilities
    );

    let serra = read(
        "Name:Serra Paragon\nManaCost:2 W W\nTypes:Creature Angel\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Permanent.YouOwn+cmcLE3,Land.YouOwn | ValidAfterStack$ Spell.cmcLE3 | Condition$ PlayerTurn | MayPlay$ True | MayPlayLimit$ 1 | EffectZone$ Battlefield | AffectedZone$ Graveyard\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidSA$ Spell.MayPlaySource | Static$ True | Execute$ DBAnimate | TriggerZones$ Battlefield | Secondary$ True\n\
         T:Mode$ LandPlayed | ValidCard$ Land | ValidSA$ SpellAbility.MayPlaySource | Static$ True | Execute$ DBAnimate | TriggerZones$ Battlefield | Secondary$ True\n\
         SVar:DBAnimate:DB$ Animate | Defined$ TriggeredCardLKICopy | Duration$ Permanent | Triggers$ TrigDieExile\n\
         SVar:TrigDieExile:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Exile | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 2",
    );
    assert_eq!(serra.abilities.len(), 1);
    assert!(
        serra.abilities[0].contains("ZonePlayCastRider::GrantDeathExileAndGainLife(2)")
            && serra.abilities[0].contains("allow_lands: true")
            && serra.abilities[0].contains("max_mana_value: Some(Amount::Fixed(3))"),
        "{:?}",
        serra.abilities
    );

    let glimpse = read(
        "Name:Glimpse the Cosmos\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 3 | ChangeNum$ 1 | NoReveal$ True\n\
         S:Mode$ Continuous | Affected$ Card.Self | EffectZone$ Graveyard | AffectedZone$ Graveyard | MayPlay$ True | MayPlayAltManaCost$ U\n\
         R:Event$ Moved | ValidLKI$ Card.CastSa Spell.MayPlaySource | Origin$ Stack | Destination$ Graveyard | ReplaceWith$ MoveExile\n\
         SVar:MoveExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Stack | Destination$ Exile",
    );
    assert_eq!(glimpse.abilities.len(), 2);
    assert!(
        glimpse.abilities[1].contains("ZonePlayCastRider::ExileInsteadOfGraveyard")
            && glimpse.abilities[1].contains("zone = ZoneRef::Graveyard"),
        "{:?}",
        glimpse.abilities
    );
}

#[test]
fn exiled_by_you_and_mana_spending_permissions() {
    let draugr = read(
        "Name:Draugr Necromancer\nManaCost:3 B\nTypes:Snow Creature Zombie\nPT:4/4\n\
         S:Mode$ Continuous | Affected$ Card.OppOwn+counters_GE1_ICE+nonLand | AffectedZone$ Exile | MayPlay$ True | MayPlaySnowIgnoreColor$ True",
    );
    assert!(
        draugr.abilities[0].contains("ZonePlayManaSpending::SnowAsAnyColor")
            && draugr.abilities[0].contains("OwnedBy(PlayerRel::EachOpponent)")
            && draugr.abilities[0].contains("counters::ICE"),
        "{:?}",
        draugr.abilities
    );

    let any_type = read(
        "Name:AnyTypeProbe\nManaCost:2 U B\nTypes:Creature\nPT:4/2\n\
         Oracle:You may play lands and cast noncreature spells from among cards you exiled with ice counters on them, and mana of any type can be spent to cast those spells.\n\
         S:Mode$ Continuous | Affected$ Card.ExiledByYou+counters_GE1_ICE | AffectedZone$ Exile | MayPlay$ True | MayPlayIgnoreType$ True | ValidAfterStack$ Spell.nonCreature",
    );
    assert!(
        any_type.abilities[0].contains("AltCastZones::ExiledByYou")
            && any_type.abilities[0].contains("ZonePlayManaSpending::AnyType")
            && any_type.abilities[0].contains("Filter::Not(&Filter::CREATURE)")
            && any_type.abilities[0].contains("allow_lands: true"),
        "{:?}",
        any_type.abilities
    );

    // Haldan's unsupported FETCH counter and ambiguous MayPlayIgnoreType must remain refused.
    assert!(refused(
        "Name:Haldan, Avid Arcanist\nManaCost:2 U\nTypes:Legendary Creature\nPT:1/4\n\
         Oracle:You may play lands and cast noncreature spells from among cards you exiled that have fetch counters on them, and you may spend mana as though it were mana of any color to cast those spells.\n\
         S:Mode$ Continuous | Affected$ Card.ExiledByYou+counters_GE1_FETCH | AffectedZone$ Exile | MayPlay$ True | MayPlayIgnoreColor$ True | ValidAfterStack$ Spell.nonCreature"
    ));
    assert!(refused(
        "Name:AmbiguousIgnoreType\nManaCost:2 U B\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.ExiledByYou+counters_GE1_ICE | AffectedZone$ Exile | MayPlay$ True | MayPlayIgnoreType$ True"
    ));
}
