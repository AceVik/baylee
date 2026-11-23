//! Wave 27a unit tests: Wave 15a `SVar` / `Count$` / `CheckSVar$` /
//! `ConditionCheckSVar$` lowerings and honest refusals.

use super::*;

fn cats_w27a() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Advisor".into(),
            "Assassin".into(),
            "Bird".into(),
            "Cleric".into(),
            "Dwarf".into(),
            "Elk".into(),
            "Goblin".into(),
            "Hero".into(),
            "Human".into(),
            "Knight".into(),
            "Mouse".into(),
            "Mutant".into(),
            "Ogre".into(),
            "Phoenix".into(),
            "Phyrexian".into(),
            "Scientist".into(),
            "Spirit".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Wolf".into(),
            "Werewolf".into(),
        ],
        artifact: vec!["Clue".into(), "Spacecraft".into(), "Treasure".into()],
        land: vec![
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

fn read_w27a(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = cats_w27a();
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

fn refused_w27a(text: &str) -> bool {
    transcode(&parse(text), &cats_w27a(), None).is_none()
}

#[test]
fn wave27a_entered_this_turn_battlefield_graveyard_and_exile() {
    // Akal Pakal, First Among Equals: `Count$ThisTurnEntered_Battlefield_Artifact.YouCtrl`.
    let akal = read_w27a(
        "Name:Akal Pakal, First Among Equals\nManaCost:2 U\nTypes:Legendary Creature Human Advisor\nPT:1/5\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 2 | ChangeNum$ 1 | DestinationZone2$ Graveyard | NoReveal$ True\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Artifact.YouCtrl",
    );
    assert!(
        akal.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::ARTIFACT, from: None, \
             to: ZoneRef::Battlefield, owner_or_ctrl: PlayerRel::You, exclude_source: false }"
        ),
        "Akal Pakal: {:?}",
        akal.abilities
    );

    // Armory Mice (Celebration): `Count$ThisTurnEntered_Battlefield_Permanent.nonLand+YouCtrl`, `GE2`.
    let armory = read_w27a(
        "Name:Armory Mice\nManaCost:1 W\nTypes:Creature Mouse\nPT:3/1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddToughness$ 2 | CheckSVar$ Celebration | SVarCompare$ GE2\n\
         SVar:Celebration:Count$ThisTurnEntered_Battlefield_Permanent.nonLand+YouCtrl",
    );
    assert!(
        armory.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::NONLAND, from: None, \
             to: ZoneRef::Battlefield, owner_or_ctrl: PlayerRel::You, exclude_source: false }"
        ),
        "Armory Mice: {:?}",
        armory.abilities
    );

    // Bellowing Elk: `StrictlyOther` sets `exclude_source: true`.
    let elk = read_w27a(
        "Name:Bellowing Elk\nManaCost:3 G\nTypes:Creature Elk\nPT:4/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Trample & Indestructible | CheckSVar$ X\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Creature.YouCtrl+StrictlyOther",
    );
    assert!(
        elk.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::CREATURE, from: None, \
             to: ZoneRef::Battlefield, owner_or_ctrl: PlayerRel::You, exclude_source: true }"
        ),
        "Bellowing Elk: {:?}",
        elk.abilities
    );

    // Asmira, Holy Avenger: `Graveyard_from_Battlefield_Creature.YouOwn` -> `owner_or_ctrl: You`.
    let asmira = read_w27a(
        "Name:Asmira, Holy Avenger\nManaCost:2 G W\nTypes:Legendary Creature Human Cleric\nPT:2/3\n\
         K:Flying\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature.YouOwn",
    );
    assert!(
        asmira.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::CREATURE, from: Some(ZoneRef::Battlefield), \
             to: ZoneRef::Graveyard, owner_or_ctrl: PlayerRel::You, exclude_source: false }"
        ),
        "Asmira: {:?}",
        asmira.abilities
    );
}

#[test]
fn wave27a_entered_this_turn_departures_and_sums() {
    // Brimaz / Liliana's Standard Bearer: `Graveyard_from_Battlefield_<filter>.YouCtrl`
    // checks controller on the departed battlefield snapshot with `owner_or_ctrl: EachPlayer`.
    let died_ctrl = read_w27a(
        "Name:Standard Bearer Shape\nManaCost:2 B\nTypes:Creature Human Knight\nPT:3/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature.YouCtrl",
    );
    assert!(
        died_ctrl.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::YOUR_CREATURE, \
             from: Some(ZoneRef::Battlefield), to: ZoneRef::Graveyard, \
             owner_or_ctrl: PlayerRel::EachPlayer, exclude_source: false }"
        ),
        "Standard Bearer: {:?}",
        died_ctrl.abilities
    );

    // Vren shape: `Exile_from_Battlefield_Creature.OppCtrl`.
    let exiled_opp = read_w27a(
        "Name:Exile Tracker\nManaCost:2 U B\nTypes:Creature Wizard\nPT:3/4\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ThisTurnEntered_Exile_from_Battlefield_Creature.OppCtrl",
    );
    assert!(
        exiled_opp.abilities[0].contains(
            "Amount::EnteredThisTurn { filter: &Filter::OPPONENT_CREATURE, \
             from: Some(ZoneRef::Battlefield), to: ZoneRef::Exile, \
             owner_or_ctrl: PlayerRel::EachPlayer, exclude_source: false }"
        ),
        "Exile Tracker: {:?}",
        exiled_opp.abilities
    );

    // Welcome the Dead shape: graveyard from hand + graveyard from library sum.
    let milled_discarded = read_w27a(
        "Name:Grave Sum\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | Defined$ Opponent | LifeAmount$ X\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Hand_Card.YouOwn/Plus.Y\n\
         SVar:Y:Count$ThisTurnEntered_Graveyard_from_Library_Card.YouOwn",
    );
    assert!(
        milled_discarded.abilities[0].contains("Amount::Sum")
            && milled_discarded.abilities[0].contains("from: Some(ZoneRef::Hand)")
            && milled_discarded.abilities[0].contains("from: Some(ZoneRef::Library)"),
        "Grave Sum: {:?}",
        milled_discarded.abilities
    );

    // Earth Rumble Wrestlers: `SVar$Y/Plus.Z` combining battlefield count + land entered this turn.
    let wrestlers = read_w27a(
        "Name:Earth Rumble Wrestlers\nManaCost:3 RG\nTypes:Creature Human Warrior\nPT:3/4\n\
         K:Reach\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | AddKeyword$ Trample | CheckSVar$ X\n\
         SVar:Y:Count$Valid Creature.Land+YouCtrl\n\
         SVar:Z:Count$ThisTurnEntered_Battlefield_Land.YouCtrl\n\
         SVar:X:SVar$Y/Plus.Z",
    );
    assert!(
        wrestlers.abilities[0].contains("Amount::Sum")
            && wrestlers.abilities[0].contains("Amount::EnteredThisTurn"),
        "Earth Rumble Wrestlers: {:?}",
        wrestlers.abilities
    );
}

#[test]
fn wave27a_spells_cast_matching_and_distinct_card_types_cast() {
    // Alicia Masters shape: `Count$ThisTurnCast_Card.nonCreature+YouCtrl`.
    let alicia = read_w27a(
        "Name:Alicia Shape\nManaCost:1 R\nTypes:Legendary Creature Human\nPT:0/4\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$ThisTurnCast_Card.nonCreature+YouCtrl",
    );
    assert!(
        alicia.abilities[0].contains(
            "Amount::SpellsCastMatchingThisTurn { filter: &Filter::NONCREATURE, caster: PlayerRel::You }"
        ),
        "Alicia: {:?}",
        alicia.abilities
    );

    // Arclight Phoenix: `Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl` with `GE3`.
    let phoenix = read_w27a(
        "Name:Arclight Phoenix\nManaCost:3 R\nTypes:Creature Phoenix\nPT:3/2\n\
         K:Flying\nK:Haste\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Graveyard | \
         CheckSVar$ X | SVarCompare$ GE3 | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert!(
        phoenix.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::SpellsCastMatchingThisTurn { \
             filter: &Filter::INSTANT_OR_SORCERY, caster: PlayerRel::You }, 3)"
        ),
        "Arclight Phoenix: {:?}",
        phoenix.abilities
    );

    // Canyon Crab: `Count$ThisTurnCast_Card.wasCastFromYourHandByYou` with `LT1`.
    let crab = read_w27a(
        "Name:Canyon Crab\nManaCost:1 U\nTypes:Creature\nPT:0/5\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ LT1 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Mode$ TgtChoose\n\
         SVar:X:Count$ThisTurnCast_Card.wasCastFromYourHandByYou",
    );
    assert!(
        crab.statics.contains(
            "Amount::SpellsCastMatchingThisTurn { filter: &Filter::CastFromHand, caster: PlayerRel::You }"
        ) && crab.abilities[0].contains("Condition::Not(&CHECK"),
        "Canyon Crab: {:?} / {}",
        crab.abilities,
        crab.statics
    );

    // Eshki Dragonclaw: `LimitMax.1` sum of creature cast + noncreature cast (`GE2`).
    let eshki = read_w27a(
        "Name:Eshki Dragonclaw\nManaCost:1 G U R\nTypes:Legendary Creature Human Warrior\nPT:4/4\n\
         K:Vigilance\nK:Trample\nK:Ward:1\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ Z | SVarCompare$ GE2 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2\n\
         SVar:X:Count$ThisTurnCast_Creature.YouCtrl/LimitMax.1\n\
         SVar:Y:Count$ThisTurnCast_Card.nonCreature+YouCtrl/LimitMax.1\n\
         SVar:Z:SVar$X/Plus.Y",
    );
    assert!(
        eshki.abilities[1].contains("Condition::All(&[")
            && eshki.abilities[1].contains("filter: &Filter::CREATURE, caster: PlayerRel::You")
            && eshki.abilities[1].contains("filter: &Filter::NONCREATURE, caster: PlayerRel::You"),
        "Eshki: {:?}",
        eshki.abilities
    );

    // April O'Neil, Hacktivist: `Count$ThisTurnCast_Card.YouCtrl$CardTypes`.
    let april = read_w27a(
        "Name:April O'Neil, Hacktivist\nManaCost:3 U\nTypes:Legendary Creature Human Scientist\nPT:1/5\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl$CardTypes",
    );
    assert!(
        april.abilities[0].contains("Amount::DistinctCardTypesCastThisTurn(PlayerRel::You)"),
        "April O'Neil: {:?}",
        april.abilities
    );

    // Opponent and all-player distinct card types cast.
    let opp_types = read_w27a(
        "Name:Opp Type Counter\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ThisTurnCast_Card.OppCtrl$CardTypes",
    );
    assert!(
        opp_types.abilities[0]
            .contains("Amount::DistinctCardTypesCastThisTurn(PlayerRel::EachOpponent)"),
        "{:?}",
        opp_types.abilities
    );

    // Opponent colored spell cast in `ConditionCheckSVar$` (Veil of Summer shape).
    let opp_spell_cond = read_w27a(
        "Name:Veil Shape\nManaCost:G\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ X\n\
         SVar:X:Count$ThisTurnCast_Card.OppCtrl+Blue,Card.OppCtrl+Black",
    );
    assert!(
        opp_spell_cond.abilities[0].contains("Amount::SpellsCastMatchingThisTurn")
            && opp_spell_cond.abilities[0].contains("caster: PlayerRel::EachOpponent"),
        "{:?}",
        opp_spell_cond.abilities
    );
}

#[test]
fn wave27a_attacked_distinct_mana_values_distinct_counters_and_highest_hand() {
    // Avengers Assemble!: `EnteredThisTurn` + `CreaturesAttackedThisTurn` summed.
    let avengers = read_w27a(
        "Name:Avengers Assemble!\nManaCost:4 W\nTypes:Enchantment\n\
         K:Flash\n\
         S:Mode$ Continuous | Affected$ Creature.Hero+YouCtrl | AddPower$ 2 | AddToughness$ 2\n\
         T:Mode$ Phase | Phase$ End of Turn | CheckSVar$ X | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Hero.YouCtrl/Plus.Y\n\
         SVar:Y:Count$CreaturesAttackedThisTurn Hero.YouCtrl",
    );
    assert!(
        avengers.abilities[1].contains("Amount::EnteredThisTurn")
            && avengers.abilities[1].contains("Amount::CreaturesAttackedThisTurn"),
        "Avengers Assemble!: {:?}",
        avengers.abilities
    );

    // Thaumaton Torpedo: `ReduceCost$ X` via `Count$Compare Y GE1.3.0` + `CreaturesAttackedThisTurn`.
    let torpedo = read_w27a(
        "Name:Thaumaton Torpedo\nManaCost:1\nTypes:Artifact\n\
         A:AB$ Destroy | Cost$ 6 T Sac<1/CARDNAME> | ValidTgts$ Permanent.nonLand | ReduceCost$ X\n\
         SVar:X:Count$Compare Y GE1.3.0\n\
         SVar:Y:Count$CreaturesAttackedThisTurn Spacecraft.YouCtrl",
    );
    assert!(
        torpedo.abilities[0].contains("CostReduction::If")
            && torpedo.abilities[0].contains("Amount::CreaturesAttackedThisTurn"),
        "Thaumaton Torpedo: {:?}",
        torpedo.abilities
    );

    // Charging Cinderhorn shape: `PlayerCountPlayers$AttackersDeclared` `EQ0`.
    let cinderhorn = read_w27a(
        "Name:Charging Cinderhorn Shape\nManaCost:3 R\nTypes:Creature\nPT:4/2\nK:Haste\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | Execute$ TrigPutCounter | \
         CheckSVar$ AttackedThisTurn | SVarCompare$ EQ0\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ CHARGE | CounterNum$ 1 | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:AttackedThisTurn:PlayerCountPlayers$AttackersDeclared\n\
         SVar:X:Count$CardCounters.CHARGE",
    );
    assert!(
        cinderhorn.statics.contains(
            "Amount::CreaturesAttackedThisTurn { filter: &Filter::Any, controller: PlayerRel::EachPlayer }"
        ) && cinderhorn.abilities[0].contains("Condition::Not(&CHECK"),
        "Charging Cinderhorn: {:?} / {}",
        cinderhorn.abilities,
        cinderhorn.statics
    );

    // Aven Heartstabber: `Count$ValidGraveyard Card.YouOwn$DifferentCardManaCost`, `GE5`.
    let heartstabber = read_w27a(
        "Name:Aven Heartstabber\nManaCost:U B\nTypes:Creature Bird Assassin\nPT:1/1\nK:Flying\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddToughness$ 2 | \
         AddKeyword$ Deathtouch | CheckSVar$ X | SVarCompare$ GE5\n\
         SVar:X:Count$ValidGraveyard Card.YouOwn$DifferentCardManaCost\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 2 | Defined$ You | SubAbility$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You",
    );
    assert!(
        heartstabber.abilities[0].contains(
            "Amount::DistinctManaValuesInZone { zone: ZoneSel::GraveyardOf(PlayerRel::You), filter: &Filter::Any }"
        ),
        "Aven Heartstabber: {:?}",
        heartstabber.abilities
    );

    // Distinct mana values in linked exile (`ExiledWith$DifferentCardManaCost`) and scaled graveyard.
    let linked_mv = read_w27a(
        "Name:Gateway Shape\nManaCost:2\nTypes:Artifact\n\
         A:AB$ GainLife | Cost$ T | LifeAmount$ 5 | ConditionCheckSVar$ Y | ConditionSVarCompare$ GE5\n\
         SVar:Y:ExiledWith$DifferentCardManaCost",
    );
    assert!(
        linked_mv.abilities[0].contains(
            "Amount::DistinctManaValuesInZone { zone: ZoneSel::LinkedExile, filter: &Filter::Any }"
        ),
        "{:?}",
        linked_mv.abilities
    );

    let scaled_mv = read_w27a(
        "Name:Scaled MV\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$ValidGraveyard Instant.YouOwn,Sorcery.YouOwn$DifferentCardManaCost/Times.2",
    );
    assert!(
        scaled_mv.abilities[0].contains("Amount::Times")
            && scaled_mv.abilities[0].contains("Amount::DistinctManaValuesInZone"),
        "{:?}",
        scaled_mv.abilities
    );

    // Blitzball Stadium inline `NumCards$ Count$DifferentCounterKinds_Card.Self` + SVar form.
    let blitzball = read_w27a(
        "Name:Blitzball Drawer\nManaCost:2 U\nTypes:Artifact\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ Count$DifferentCounterKinds_Card.Self",
    );
    assert!(
        blitzball.abilities[0].contains("Amount::DistinctCounterKindsOnSource"),
        "Blitzball: {:?}",
        blitzball.abilities
    );
}

#[test]
fn wave27a_highest_opponent_hand_size_and_amount_compare() {
    // Akuta, Born of Ash: `Phase.CheckSVar` comparing `Count$ValidHand Card.YouOwn` vs `HighestOpponentHandSize`.
    let akuta = read_w27a(
        "Name:Akuta, Born of Ash\nManaCost:2 B B\nTypes:Legendary Creature Spirit\nPT:3/2\nK:Haste\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | CheckSVar$ X | SVarCompare$ GTY | \
         TriggerZones$ Graveyard | Execute$ TrigReturn\n\
         SVar:TrigReturn:AB$ ChangeZone | Cost$ Sac<1/Swamp> | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:PlayerCountOpponents$HighestCardsInHand",
    );
    assert!(
        akuta.abilities[0].contains("Amount::HandSize(PlayerRel::You)")
            && akuta.abilities[0].contains("Amount::HighestOpponentHandSize"),
        "Akuta: {:?}",
        akuta.abilities
    );

    // Okina Nightwatch & Pulse of the Grid: `Continuous.CheckSVar` and `ConditionCheckSVar$`.
    let okina = read_w27a(
        "Name:Okina Nightwatch\nManaCost:4 G\nTypes:Creature Human\nPT:4/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 3 | AddToughness$ 3 | CheckSVar$ X | SVarCompare$ GTY\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:PlayerCountOpponents$HighestCardsInHand",
    );
    assert!(
        okina.abilities[0].contains("Amount::HighestOpponentHandSize"),
        "Okina: {:?}",
        okina.abilities
    );

    let pulse = read_w27a(
        "Name:Pulse of the Grid\nManaCost:1 U U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 2 | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Mode$ TgtChoose | SubAbility$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ Parent | Origin$ Stack | Destination$ Hand | \
         ConditionCheckSVar$ X | ConditionSVarCompare$ GTY\n\
         SVar:Y:Count$CardsInYourHand\n\
         SVar:X:PlayerCountOpponents$HighestCardsInHand",
    );
    assert!(
        pulse.abilities[0].contains("Amount::HighestOpponentHandSize")
            && pulse.abilities[0].contains("Amount::HandSize(PlayerRel::You)"),
        "Pulse of the Grid: {:?}",
        pulse.abilities
    );
}

#[test]
fn wave27a_counters_placed_on_source_if_condition_and_landwalk_preservation() {
    // Beast, Erudite Aerialist: `Count$CountersAddedThisTurn P1P1 You Card.Self` (`by_you: true`).
    let beast = read_w27a(
        "Name:Beast, Erudite Aerialist\nManaCost:3 GU\nTypes:Legendary Creature Mutant Scientist Hero\nPT:3/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ X\n\
         SVar:X:Count$CountersAddedThisTurn P1P1 You Card.Self\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert!(
        beast.abilities[0].contains(
            "Condition::CountersPlacedOnSourceThisTurn { kind: CounterKind::P1P1, by_you: true }"
        ),
        "Beast: {:?}",
        beast.abilities
    );

    // Any-player counter placement (`by_you: false`) + negated `EQ0` + basic landwalk preserved.
    let walker = read_w27a(
        "Name:Swamp Counter Watcher\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         K:Landwalk:Swamp\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ EQ0 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         SVar:X:Count$CountersAddedThisTurn P1P1 Player Card.Self",
    );
    assert!(
        walker
            .keywords
            .contains(&"KeywordSet::SWAMPWALK".to_string()),
        "landwalk must be preserved: {:?}",
        walker.keywords
    );
    assert!(
        walker.abilities[0].contains(
            "Condition::Not(&Condition::CountersPlacedOnSourceThisTurn { kind: CounterKind::P1P1, by_you: false })"
        ),
        "{:?}",
        walker.abilities
    );

    // Agency Coroner: `Count$Compare Y EQ1.2.1` + `Sacrificed$Valid Card.IsSuspected` -> `Amount::IfCondition`.
    let coroner = read_w27a(
        "Name:Agency Coroner\nManaCost:4 B\nTypes:Creature Ogre Cleric\nPT:3/6\n\
         A:AB$ Draw | Cost$ 2 B Sac<1/Creature.Other/another creature> | NumCards$ X\n\
         SVar:Y:Sacrificed$Valid Card.IsSuspected\n\
         SVar:X:Count$Compare Y EQ1.2.1",
    );
    assert!(
        coroner.abilities[0].contains(
            "Amount::IfCondition { condition: &Condition::SacrificedForCost(&Filter::Suspected), \
             then: &Amount::Fixed(2), otherwise: &Amount::Fixed(1) }"
        ),
        "Agency Coroner: {:?}",
        coroner.abilities
    );

    // Resolution `/LimitMax.4` cap via `Amount::IfCondition`.
    let capped = read_w27a(
        "Name:Capped Draw\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Artifact.YouCtrl/LimitMax.4",
    );
    assert!(
        capped.abilities[0].contains("Amount::IfCondition")
            && capped.abilities[0].contains("Amount::EnteredThisTurn")
            && capped.abilities[0].contains("Amount::Fixed(4)"),
        "Capped Draw: {:?}",
        capped.abilities
    );
}

#[test]
fn wave27a_honest_refusals_and_non_paper_guards() {
    for bad in [
        // Hidden destination zone (`Hand`) is not tracked by `PerTurn::zone_changes`.
        "Name:Barrin Shape\nManaCost:1 U U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         SVar:X:Count$ThisTurnEntered_Hand_from_Battlefield_Permanent.YouOwn",
        // Hidden destination zone (`Library`).
        "Name:Library Enter\nManaCost:1 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:X:Count$ThisTurnEntered_Library_from_Battlefield_Permanent.YouOwn",
        // `tokenCreated` atom is unmodelled (`Bennie Bracks`).
        "Name:Bennie Shape\nManaCost:3 W\nTypes:Creature\nPT:3/2\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ D\n\
         SVar:D:DB$ Draw\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Card.tokenCreated+YouOwn",
        // `CountersAddedThisTurn` with `Any` counter kind (`Wakka`, `Fractal Tender`).
        "Name:Wakka Shape\nManaCost:2 G W\nTypes:Creature\nPT:4/4\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ D\n\
         SVar:D:DB$ Draw\n\
         SVar:X:Count$CountersAddedThisTurn Any Player Card.Self",
        // `CountersAddedThisTurn` targeting non-self (`Sigardian Paladin`, `Fairgrounds Trumpeter`).
        "Name:Sigardian Shape\nManaCost:2 G W\nTypes:Creature\nPT:4/4\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Trample | CheckSVar$ X\n\
         SVar:X:Count$CountersAddedThisTurn P1P1 You Creature",
        // `CountersAddedThisTurn` with count > 1 (`GE2` cannot lower to boolean condition).
        "Name:Two Counters Self\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | CheckSVar$ X | SVarCompare$ GE2\n\
         SVar:X:Count$CountersAddedThisTurn P1P1 You Card.Self",
        // `DifferentCounterKinds` on board permanents (only `Card.Self` is modelled).
        "Name:Hundred Battle Shape\nManaCost:3 B\nTypes:Creature\nPT:4/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | CheckSVar$ X | SVarCompare$ GE3\n\
         SVar:X:Count$DifferentCounterKinds_Creature.YouCtrl",
        // `PlayerCountPlayers$HighestCardsInHand` includes controller, not opponents only.
        "Name:All Players Max Hand\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:PlayerCountPlayers$HighestCardsInHand",
        // Filtered `$CardTypes` on `ThisTurnCast` (`DistinctCardTypesCastThisTurn` has no filter).
        "Name:Filtered Cast Types\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ThisTurnCast_Creature.YouCtrl$CardTypes",
        // `!CastSaSource` exclusion on `ThisTurnCast_` is not modelled on `SpellsCastMatchingThisTurn`.
        "Name:Other Red Spell\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ X\n\
         SVar:X:Count$ThisTurnCast_Card.Red+!CastSaSource+YouCtrl",
        // `EffectSource` on `CreaturesAttackedThisTurn` (`Goblin Researcher`).
        "Name:Effect Source Attacked\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Haste | CheckSVar$ X\n\
         SVar:X:Count$CreaturesAttackedThisTurn Creature.EffectSource",
        // Mixed `StrictlyOther` across comma-separated alternatives.
        "Name:Mixed Other\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | CheckSVar$ X\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Creature.YouCtrl+StrictlyOther,Artifact.YouCtrl",
        // Unmodelled `$GreatestCardManaCost` suffix on `ThisTurnCast_`.
        "Name:Greatest Cast MV\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl$GreatestCardManaCost",
        // Alchemy / digital effects alongside Wave 27a SVars must remain strictly refused.
        "Name:Alchemy MakeCard With Celebration\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GE2 | Execute$ TrigMake\n\
         SVar:TrigMake:DB$ MakeCard | Conjure$ True | Name$ Lightning Bolt | Zone$ Hand\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Permanent.nonLand+YouCtrl",
        "Name:Alchemy Draft With Distinct MV\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Draft | ConditionCheckSVar$ Count$ValidGraveyard Card.YouOwn$DifferentCardManaCost | \
         ConditionSVarCompare$ GE5 | Spellbook$ Corpse Churn",
        "Name:Unset Sticker With Hand Compare\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         K:StickerSheet\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | CheckSVar$ X | SVarCompare$ GTY\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:PlayerCountOpponents$HighestCardsInHand",
    ] {
        assert!(refused_w27a(bad), "must refuse honestly:\n{bad}");
    }
}
