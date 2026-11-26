use super::*;

fn tokens_w17b() -> TokenLookup {
    TokenLookup::held(&[(
        "gu_0_0_fractal",
        "Name:Fractal\nTypes:Creature Fractal\nColors:green,blue\nPT:0/0",
    )])
}

fn cats_w17b() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Angel".into(),
        "Advisor".into(),
        "Cleric".into(),
        "Demon".into(),
        "Druid".into(),
        "Elemental".into(),
        "Elf".into(),
        "Fractal".into(),
        "Human".into(),
        "Scout".into(),
        "Soldier".into(),
        "Sponge".into(),
        "Turtle".into(),
        "Warlock".into(),
    ]);
    c.artifact.push("Book".into());
    c.planeswalker.push("Jace".into());
    c.normalize();
    c
}

fn read_w17b(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = cats_w17b();
    let t = tokens_w17b();
    transcode(&parsed, &c, Some(&t)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, Some(&t))
        )
    })
}

fn refused_w17b(text: &str) -> bool {
    let c = cats_w17b();
    let t = tokens_w17b();
    transcode(&parse(text), &c, Some(&t)).is_none()
}

#[test]
fn wave17b_empower_spells_triggers_excess_and_exile_count() {
    let proteges = read_w17b(
        "Name:Protege's Awakening\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ Empower | Type$ Jace | Num$ 6 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    assert!(
        proteges.abilities[0].contains(
            "Effect::Empower { token: &generated_tokens::JACE_PLANESWALKER_BLUE, amount: Amount::Fixed(6) }"
        ) && proteges.abilities[0].contains("Effect::draw(1)"),
        "{}",
        proteges.abilities[0]
    );

    let violent = read_w17b(
        "Name:Violent Echoes\nManaCost:2 R R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ 6 | ExcessSVar$ Excess | \
         ExcessSVarCondition$ Card.targetedBy | SubAbility$ DBEmpower\n\
         SVar:DBEmpower:DB$ Empower | Type$ Jace | Num$ Excess | ConditionCheckSVar$ Excess",
    );
    assert!(
        violent.abilities[0]
            .contains("Effect::Empower { token: &generated_tokens::JACE_PLANESWALKER_BLUE"),
        "{}",
        violent.abilities[0]
    );

    let overwrite = read_w17b(
        "Name:Overwrite the Multiverse\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Exile | \
         RememberChanged$ True | SubAbility$ DBEmpower\n\
         SVar:DBEmpower:DB$ Empower | Type$ Jace | Num$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        overwrite.abilities[0].contains(
            "Effect::Empower { token: &generated_tokens::JACE_PLANESWALKER_BLUE, amount: Amount::ThisWay(ThisWay::ExiledFromBattlefield(&Filter::Any)) }"
        ),
        "{}",
        overwrite.abilities[0]
    );

    // Non-Jace Empower type and opponent Defined$ stay refused.
    assert!(refused_w17b(
        "Name:BadEmpowerType\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Empower | Type$ Chandra | Num$ 2"
    ));
    assert!(refused_w17b(
        "Name:BadEmpowerWho\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Empower | Defined$ Opponent | Type$ Jace | Num$ 2"
    ));
}

#[test]
fn wave17b_different_card_power_coven_and_choose_distinct_powers() {
    let golden = read_w17b(
        "Name:Golden Ratio\nManaCost:1 G U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$DifferentCardPower",
    );
    assert!(
        golden.abilities[0].contains(
            "Effect::DrawCards { amount: Amount::DistinctPowersAmong(&Filter::YOUR_CREATURE) }"
        ),
        "{}",
        golden.abilities[0]
    );

    let augur = read_w17b(
        "Name:Augur of Autumn\nManaCost:1 G G\nTypes:Creature Human Druid\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You\n\
         S:Mode$ Continuous | Affected$ Land.TopLibrary+YouCtrl | AffectedZone$ Library | MayPlay$ True\n\
         S:Mode$ Continuous | CheckSVar$ X | SVarCompare$ GE3 | Affected$ Creature.TopLibrary+nonLand+YouCtrl | AffectedZone$ Library | MayPlay$ True\n\
         SVar:X:Count$Valid Creature.YouCtrl$DifferentCardPower",
    );
    assert!(
        augur
            .abilities
            .iter()
            .any(|a| a.contains("Amount::DistinctPowersAmong(&Filter::YOUR_CREATURE)")),
        "{:?}",
        augur.abilities
    );

    let vanguard = read_w17b(
        "Name:Sigarda's Vanguard\nManaCost:4 W\nTypes:Creature Angel\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChoose\n\
         SVar:TrigChoose:DB$ ChooseCard | MinAmount$ 0 | Amount$ Max | Choices$ Creature | WithDifferentPowers$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ChosenCard | KW$ Double Strike | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True\n\
         SVar:Max:Count$Valid Creature$DifferentCardPower",
    );
    assert!(
        vanguard.abilities[0].contains(
            "Effect::ChooseDistinctPowersOnBattlefield { chooser: PlayerRel::You, filter: &Filter::CREATURE, min: 0, max: Amount::DistinctPowersAmong(&Filter::CREATURE)"
        ),
        "{}",
        vanguard.abilities[0]
    );

    // Non-battlefield WithDifferentPowers$ stays refused.
    assert!(refused_w17b(
        "Name:BadGraveyardPowers\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Graveyard | MinAmount$ 0 | Amount$ 2 | Choices$ Creature.YouOwn | WithDifferentPowers$ True | SubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ ChosenCard"
    ));
}

#[test]
fn wave17b_etb_counter_dynamic_amounts_and_custom_counters() {
    let journal = read_w17b(
        "Name:Investigator's Journal\nManaCost:2\nTypes:Artifact Book Clue\n\
         K:etbCounter:SUSPECT:X:no Condition:desc\n\
         SVar:X:PlayerCountPlayers$HighestValid Creature.YouCtrl\n\
         A:AB$ Draw | Cost$ 2 T SubCounter<1/SUSPECT> | NumCards$ 1",
    );
    assert_eq!(
        journal.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: counters::SUSPECT, amount: Amount::HighestControlledCount { among: PlayerRel::EachPlayer, filter: &Filter::CREATURE } }"
        ]
    );

    let sponge = read_w17b(
        "Name:Thought Sponge\nManaCost:3 U\nTypes:Creature Sponge\nPT:1/1\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:PlayerCountOpponents$HighestCardsDrawn",
    );
    assert_eq!(
        sponge.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::HighestCardsDrawnThisTurn(PlayerRel::EachOpponent) }"
        ]
    );

    let inferno = read_w17b(
        "Name:Inferno Project\nManaCost:6 R\nTypes:Creature Elemental\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:Count$ValidGraveyard Instant.YouOwn,Sorcery.YouOwn$CardManaCost",
    );
    assert!(
        inferno.enter_modifiers[0]
            .contains("Amount::TotalManaValueInZone { zone: ZoneSel::GraveyardYou"),
        "{:?}",
        inferno.enter_modifiers
    );

    let embodiment = read_w17b(
        "Name:Embodiment of Agonies\nManaCost:1 B B\nTypes:Creature Demon\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:Count$ExactManaCostGraveyard Card.nonLand+YouOwn",
    );
    assert_eq!(
        embodiment.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::DistinctExactManaCostsInZone { zone: ZoneSel::GraveyardYou, filter: &Filter::NONLAND } }"
        ]
    );

    let magistrate = read_w17b(
        "Name:Magnanimous Magistrate\nManaCost:5 W\nTypes:Creature Human Advisor\nPT:3/4\n\
         K:etbCounter:REPR:5",
    );
    assert_eq!(
        magistrate.enter_modifiers,
        ["EnterModifier::WithCounters { kind: counters::REPRIEVE, amount: Amount::Fixed(5) }"]
    );

    let divinity = read_w17b(
        "Name:Myojin Probe\nManaCost:5 W W W\nTypes:Legendary Creature\nPT:4/6\n\
         K:etbCounter:DIVINITY:1:CheckSVar$ FromHand:desc\n\
         SVar:FromHand:Count$wasCastFromYourHandByYou.1.0",
    );
    assert!(
        divinity.enter_modifiers[0].contains("kind: counters::DIVINITY"),
        "{:?}",
        divinity.enter_modifiers
    );

    let swarmlord = read_w17b(
        "Name:The Swarmlord\nManaCost:3 G U R\nTypes:Legendary Creature\nPT:5/5\n\
         K:etbCounter:P1P1:X::desc\n\
         SVar:X:Count$TotalCommanderCastFromCommandZone/Twice",
    );
    assert!(
        swarmlord.enter_modifiers[0].contains("Amount::CommanderCasts(PlayerRel::You)")
            && swarmlord.enter_modifiers[0].contains("factor: 2"),
        "{:?}",
        swarmlord.enter_modifiers
    );
}

#[test]
fn wave17b_each_from_source_all_counters_and_token_found() {
    let star_pupil = read_w17b(
        "Name:Star Pupil\nManaCost:W\nTypes:Creature Human Wizard\nPT:0/0\n\
         K:etbCounter:P1P1:1\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ EachFromSource | EachFromSource$ TriggeredCardLKICopy",
    );
    assert!(
        star_pupil.abilities[0].contains(
            "Effect::PutAllCountersFrom { donor: CounterDonor::Event, target: TargetSpec::Object(&Filter::YOUR_CREATURE) }"
        ),
        "{}",
        star_pupil.abilities[0]
    );

    let ozolith = read_w17b(
        "Name:The Ozolith\nManaCost:1\nTypes:Legendary Artifact\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | CounterType$ EachFromSource | EachFromSource$ TriggeredCardLKICopy",
    );
    assert!(
        ozolith.abilities[0].contains(
            "Effect::PutAllCountersFrom { donor: CounterDonor::Event, target: TargetSpec::ThisObject }"
        ),
        "{}",
        ozolith.abilities[0]
    );

    let augmenter = read_w17b(
        "Name:Ambitious Augmenter\nManaCost:G\nTypes:Creature Turtle Wizard\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+HasCounters | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ gu_0_0_fractal | TokenOwner$ You | RememberTokens$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Remembered | CounterType$ EachFromSource | EachFromSource$ TriggeredCardLKICopy | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        augmenter.abilities[0]
            .contains("Effect::PutAllCountersFromIntoFound { donor: CounterDonor::Event }"),
        "{}",
        augmenter.abilities[0]
    );
}

#[test]
fn wave17b_mayhem_keyword_and_mayhem_cost_paid_condition() {
    let quicksand = read_w17b(
        "Name:Sandman's Quicksand\nManaCost:1 B B\nTypes:Sorcery\n\
         K:Mayhem:3 B\n\
         A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -2 | NumDef$ -2 | IsCurse$ True | SubAbility$ DBPumpAll | \
         ConditionDefined$ Self | ConditionPresent$ Card.!CastSa Spell.Mayhem | ConditionCompare$ EQ1\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Creature.OppCtrl | NumAtt$ -2 | NumDef$ -2 | \
         ConditionDefined$ Self | ConditionPresent$ Card.CastSa Spell.Mayhem | IsCurse$ True",
    );
    assert_eq!(quicksand.mayhem.as_deref(), Some("Some(mana!(\"{3}{B}\"))"));
    assert_eq!(quicksand.keywords, ["KeywordSet::MAYHEM"]);
    assert!(
        quicksand.abilities[0].contains("Condition::Not(&Condition::MayhemCostPaid)")
            && quicksand.abilities[0].contains("Condition::MayhemCostPaid"),
        "{}",
        quicksand.abilities[0]
    );

    let hybrid_mayhem = read_w17b(
        "Name:Hybrid Mayhem Raider\nManaCost:3 B R\nTypes:Creature\nPT:4/4\n\
         K:Mayhem:2 BR",
    );
    assert_eq!(
        hybrid_mayhem.mayhem.as_deref(),
        Some("Some(mana!(\"{2}{B/R}\"))")
    );

    // Costed Mayhem on a Land MUST stay refused (`K:Mayhem` without cost is land-only).
    assert!(refused_w17b("Name:BadMayhemLand\nTypes:Land\nK:Mayhem:2 U"));
}
