use super::*;

fn w27c_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Elf".into(),
        "Vampire".into(),
        "Demon".into(),
        "Sphinx".into(),
        "Giant".into(),
        "Ooze".into(),
        "Robot".into(),
        "Soldier".into(),
        "Dragon".into(),
        "Nightmare".into(),
    ]);
    c.artifact
        .extend(["Food".into(), "Equipment".into(), "Treasure".into()]);
    c.land.extend(["Plains".into(), "Desert".into()]);
    c.normalize();
    c
}

fn w27c_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "r_1_1_goblin",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
        ),
        (
            "w_1_1_soldier",
            "Name:Soldier\nTypes:Creature Soldier\nColors:white\nPT:1/1",
        ),
        (
            "g_7_7_giant",
            "Name:Giant\nTypes:Creature Giant\nColors:green\nPT:7/7",
        ),
        (
            "c_a_food_sac",
            "Name:Food\nTypes:Artifact Food\nA:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME/this token> | LifeAmount$ 3",
        ),
        (
            "g_x_x_ooze",
            "Name:Ooze\nTypes:Creature Ooze\nColors:green\nPT:*/*",
        ),
        (
            "c_4_4_a_robot_noblock",
            "Name:Robot\nTypes:Artifact Creature Robot\nPT:4/4\nS:Mode$ CantBlock | ValidCard$ Card.Self",
        ),
    ])
}

fn read_w27c(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = w27c_cats();
    let toks = w27c_tokens();
    transcode(&parsed, &c, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "expected Wave 27c script to transcode, refused: {:?}",
            refusal_reason(&parsed, &c, Some(&toks))
        )
    })
}

fn refused_w27c(text: &str) -> bool {
    let parsed = parse(text);
    let c = w27c_cats();
    let toks = w27c_tokens();
    transcode(&parsed, &c, Some(&toks)).is_none()
}

// ---------------------------------------------------------------------------
// 1. Arjun, Bringer of the Last Gift, Act of Authority
// ---------------------------------------------------------------------------

#[test]
fn capture_affected_arjun_bringer_and_act_of_authority() {
    // Arjun, the Shifting Flame: Hand -> Library bottom (chosen order) -> Draw RememberedCount.
    let arjun = read_w27c(
        "Name:Arjun, the Shifting Flame\nManaCost:4 U R\nTypes:Legendary Creature Sphinx Wizard\nPT:5/5\nK:Flying\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ You | Execute$ TrigChangeZone | TriggerZones$ Battlefield\n\
         SVar:TrigChangeZone:DB$ ChangeZoneAll | Origin$ Hand | Destination$ Library | ChangeType$ Card.YouOwn | RememberChanged$ True | LibraryPosition$ -1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ RemTossed | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:RemTossed:Remembered$Amount",
    );
    let a = &arjun.abilities[0];
    assert!(
        a.contains("Effect::ChooseCardEx")
            && a.contains("ChooseMove::BottomOfLibraryChosenOrder")
            && (a.contains("Amount::RememberedCount")
                || a.contains("Amount::ThisWay(ThisWay::PutIntoLibraryFromHand)")),
        "Arjun ability: {a}"
    );

    // Bringer of the Last Gift: SacrificeAll other creatures -> return creatures !IsRemembered.
    let bringer = read_w27c(
        "Name:Bringer of the Last Gift\nManaCost:6 B B\nTypes:Creature Vampire Demon\nPT:6/6\nK:Flying\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self+wasCastByYou | Destination$ Battlefield | Execute$ TrigSacAll\n\
         SVar:TrigSacAll:DB$ SacrificeAll | ValidCards$ Creature.StrictlyOther | RememberSacrificed$ True | SubAbility$ DBReturnAll\n\
         SVar:DBReturnAll:DB$ ChangeZoneAll | ChangeType$ Creature.!IsRemembered | Origin$ Graveyard | Destination$ Battlefield | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let b = &bringer.abilities[0];
    assert!(
        b.contains("Effect::CaptureAffected")
            && b.contains("Effect::SacrificeAll")
            && b.contains("Effect::EachPlayerGraveyardToBattlefield")
            && bringer.statics.contains("Filter::NotRemembered"),
        "Bringer ability: {b}, statics: {:?}",
        bringer.statics
    );

    // Act of Authority: Exile target artifact/enchantment -> RememberedController gains control of ThisObject.
    let act = read_w27c(
        "Name:Act of Authority\nManaCost:1 W W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ AuthorityExile | OptionalDecider$ You\n\
         SVar:AuthorityExile:DB$ ChangeZone | ValidTgts$ Artifact,Enchantment | Origin$ Battlefield | Destination$ Exile\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigExile | TriggerZones$ Battlefield | OptionalDecider$ You\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Artifact,Enchantment | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBGainControl\n\
         SVar:DBGainControl:DB$ GainControl | Defined$ Self | NewController$ RememberedController | ConditionCheckSVar$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    let upkeep = &act.abilities[1];
    assert!(
        upkeep.contains("Effect::CaptureAffected")
            && upkeep.contains("target: TargetSpec::ThisObject")
            && upkeep.contains("to: PlayerRel::RememberedController")
            && upkeep.contains("Amount::RememberedCount"),
        "Act of Authority upkeep: {upkeep}"
    );
}

// ---------------------------------------------------------------------------
// 2. Crovax, Giant Opportunity, Cruel Reality, LKI stats (MV / Power / Toughness)
// ---------------------------------------------------------------------------

#[test]
fn capture_affected_sacrifice_branches_and_lki_amounts() {
    // Crovax the Cursed: optional upkeep sacrifice -> +1/+1 counter if EQ1, -1/-1 if EQ0.
    let crovax = read_w27c(
        "Name:Crovax the Cursed\nManaCost:2 B B\nTypes:Legendary Creature Vampire\nPT:0/0\n\
         K:etbCounter:P1P1:4\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigSacrifice | TriggerZones$ Battlefield\n\
         SVar:TrigSacrifice:DB$ Sacrifice | Optional$ True | SacValid$ Creature | Amount$ 1 | RememberSacrificed$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1 | SubAbility$ DBSubCounter\n\
         SVar:DBSubCounter:DB$ RemoveCounter | CounterType$ P1P1 | CounterNum$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let c = &crovax.abilities[0];
    assert!(
        c.contains("Effect::CaptureAffected")
            && c.contains("Effect::MayDo")
            && c.contains("Amount::RememberedCount")
            && c.contains("AmountCmp::Equal, right: &Amount::Fixed(1)")
            && c.contains("AmountCmp::Equal, right: &Amount::Fixed(0)"),
        "Crovax: {c}"
    );

    // Giant Opportunity: StrictAmount$ True optional sacrifice of 2 Foods -> 7/7 Giant or 3 Foods.
    let giant = read_w27c(
        "Name:Giant Opportunity\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Sacrifice | SacValid$ Food | Defined$ You | Amount$ 2 | Optional$ True | StrictAmount$ True | RememberSacrificed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ g_7_7_giant | TokenOwner$ You | ConditionDefined$ RememberedLKI | ConditionPresent$ Food | ConditionCompare$ EQ2 | SubAbility$ DBToken2\n\
         SVar:DBToken2:DB$ Token | TokenAmount$ 3 | TokenScript$ c_a_food_sac | TokenOwner$ You | ConditionDefined$ RememberedLKI | ConditionPresent$ Food | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        giant.abilities[0].contains("Effect::CaptureAffected")
            && giant.abilities[0].contains("Condition::ControlCount")
            && giant.abilities[0].contains("Effect::SacrificeAmountForPlayers")
            && giant.abilities[0].contains("right: &Amount::Fixed(2)")
            && giant.abilities[0].contains("right: &Amount::Fixed(0)"),
        "Giant Opportunity: {}",
        giant.abilities[0]
    );

    // Heed the Mists: Mill 1 -> Draw Remembered$CardManaCost.
    let heed = read_w27c(
        "Name:Heed the Mists\nManaCost:3 U U\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ You | NumCards$ 1 | RememberMilled$ True | SubAbility$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        heed.abilities[0].contains("Effect::CaptureAffected")
            && heed.abilities[0].contains("Amount::RememberedManaValue"),
        "Heed the Mists: {}",
        heed.abilities[0]
    );

    // First Responder shape: RememberLKI$ True + RememberedLKI$CardPower.
    let responder = read_w27c(
        "Name:First Responder\nManaCost:3 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Hidden$ True | Mandatory$ True | ChangeType$ Creature.StrictlyOther+YouCtrl | RememberLKI$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardPower",
    );
    assert!(
        responder.abilities[0].contains("Effect::CaptureAffected")
            && responder.abilities[0].contains("Amount::RememberedPower"),
        "First Responder: {}",
        responder.abilities[0]
    );
}

// ---------------------------------------------------------------------------
// 3. Mill/Dig/Token/ChangeZoneAll + nested CaptureAffected + ChooseZone::Remembered
// ---------------------------------------------------------------------------

#[test]
fn capture_affected_mill_dig_tokens_nested_and_single_object_filter_cond() {
    // Ainok Wayfarer: Mill 3 -> nested CaptureAffected (ChooseZone::Remembered Land -> Hand) -> +1/+1 if 0.
    let ainok = read_w27c(
        "Name:Ainok Wayfarer\nManaCost:1 G\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 3 | RememberMilled$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Hidden$ True | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Land.IsRemembered | SelectPrompt$ You may select a land card | RememberChanged$ True | ForgetOtherRemembered$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | CounterType$ P1P1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        ainok.abilities[0]
            .matches("Effect::CaptureAffected")
            .count(),
        2,
        "expected nested CaptureAffected: {}",
        ainok.abilities[0]
    );
    assert!(
        ainok.abilities[0].contains("zone: ChooseZone::Remembered")
            && ainok.abilities[0].contains("Amount::RememberedCount"),
        "Ainok Wayfarer: {}",
        ainok.abilities[0]
    );

    // Cantankerous Keepers: Mill 4 -> ForEachRemembered(Elf) ReturnToHand.
    let keepers = read_w27c(
        "Name:Cantankerous Keepers\nManaCost:5 G\nTypes:Creature Elf Soldier\nPT:4/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 4 | RememberMilled$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Elf.IsRemembered | Origin$ Graveyard,Exile | Destination$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        keepers.abilities[0].contains("Effect::CaptureAffected")
            && keepers.abilities[0].contains("Effect::ForEachRemembered")
            && keepers.abilities[0]
                .contains("Effect::ReturnToHand { target: TargetSpec::EventObject }"),
        "Cantankerous Keepers: {}",
        keepers.abilities[0]
    );

    // Search for Blex: Dig ChangeNum$ Any -> LoseLife Remembered$Valid Card/Times.3.
    let blex = read_w27c(
        "Name:Search for Blex\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 5 | ChangeNum$ Any | DestinationZone$ Hand | RememberChanged$ True | DestinationZone2$ Graveyard | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | LifeAmount$ X | Defined$ You | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card/Times.3",
    );
    assert!(
        blex.abilities[0].contains("Effect::CaptureAffected")
            && blex.abilities[0]
                .contains("Amount::Times { base: &Amount::RememberedCount, factor: 3 }"),
        "Search for Blex: {}",
        blex.abilities[0]
    );

    // Martial Coup: Token X RememberTokens$ True -> DestroyAll Creature.!IsRemembered if X >= 5.
    let coup = read_w27c(
        "Name:Martial Coup\nManaCost:X W W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ X | TokenScript$ w_1_1_soldier | RememberTokens$ True | TokenOwner$ You | SubAbility$ CoupDestroy\n\
         SVar:CoupDestroy:DB$ DestroyAll | ValidCards$ Creature.!IsRemembered | ConditionCheckSVar$ X | ConditionSVarCompare$ GE5 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        coup.abilities[0].contains("Effect::CaptureAffected")
            && (coup.abilities[0].contains("Filter::NotRemembered")
                || coup.statics.contains("Filter::NotRemembered")),
        "Martial Coup: {}, statics: {:?}",
        coup.abilities[0],
        coup.statics
    );

    // Psychic Spiral: target declared inside `then` (`Mill | ValidTgts$ Player | NumCards$ Remembered$Amount`).
    let spiral = read_w27c(
        "Name:Psychic Spiral\nManaCost:4 U\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card.YouOwn | Origin$ Graveyard | Destination$ Library | Shuffle$ True | RememberChanged$ True | SubAbility$ DBMill\n\
         SVar:DBMill:DB$ Mill | ValidTgts$ Player | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        spiral.abilities[0].contains("Effect::CaptureAffected")
            && spiral.abilities[0].contains("Amount::RememberedCount")
            && spiral.abilities[0]
                .contains("targets = Some(TargetReq::one(TargetSpec::AnyPlayer))"),
        "Psychic Spiral: {}",
        spiral.abilities[0]
    );

    // Shredder's Technique shape: single-target Destroy(Creature,Enchantment) -> if Enchantment, lose 2 life.
    let shredder = read_w27c(
        "Name:Shredder Probe\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature,Enchantment | RememberDestroyed$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | LifeAmount$ 2 | ConditionDefined$ RememberedLKI | ConditionPresent$ Enchantment | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        shredder.abilities[0].contains("Effect::CaptureAffected")
            && shredder.abilities[0].contains("Effect::ForEachRemembered")
            && shredder.abilities[0].contains("Effect::LoseLife"),
        "Shredder Probe: {}",
        shredder.abilities[0]
    );
}

// ---------------------------------------------------------------------------
// 4. SharesCardTypeWithRemembered / SharesNameWithRemembered / Defined$ Remembered
// ---------------------------------------------------------------------------

#[test]
fn capture_affected_shares_filters_and_defined_remembered_ops() {
    let shares = read_w27c(
        "Name:Echo Purge\nManaCost:3 W B\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ You | NumCards$ 2 | RememberMilled$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Permanent.sharesCardTypeWith Remembered,Permanent.sharesNameWith Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        shares.abilities[0].contains("Effect::CaptureAffected")
            && shares
                .statics
                .contains("Filter::SharesCardTypeWithRemembered")
            && shares.statics.contains("Filter::SharesNameWithRemembered"),
        "Echo Purge: {}, statics: {:?}",
        shares.abilities[0],
        shares.statics
    );

    // Spell-scoped Defined$ Remembered Untap without Cleanup: ChangeZoneAll tapped -> Untap Remembered.
    let untap_rem = read_w27c(
        "Name:Awakening Probe\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Land.YouOwn | Tapped$ True | RememberChanged$ True | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Remembered | Condition$ Delirium",
    );
    assert!(
        untap_rem.abilities[0].contains("Effect::CaptureAffected")
            && untap_rem.abilities[0]
                .contains("Effect::UntapAll { filter: &Filter::IsRemembered }"),
        "Awakening Probe: {}",
        untap_rem.abilities[0]
    );

    // Basic landwalk keyword preserved alongside Wave 27c CaptureAffected.
    let walker = read_w27c(
        "Name:Forestwalk Scout\nManaCost:2 G\nTypes:Creature Elf\nPT:2/2\nK:Landwalk:Forest\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 1 | RememberMilled$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        walker
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::FORESTWALK")
            && walker.abilities[0].contains("Amount::RememberedManaValue"),
        "Forestwalk Scout: kw={:?}, ab={:?}",
        walker.keywords,
        walker.abilities
    );
}

// ---------------------------------------------------------------------------
// 5. 2-slot chains: Bazaar Trader, Assassin Gauntlet, Cybernetica Datasmith
// ---------------------------------------------------------------------------

#[test]
fn multi_target_two_slot_bazaar_trader_assassin_gauntlet_and_datasmith() {
    // Bazaar Trader: Slot 1 Player -> Slot 2 GainControl NewController$ ParentTarget -> FirstTargetPlayerOrController.
    let trader = read_w27c(
        "Name:Bazaar Trader\nManaCost:1 R\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ TargetOnly | Cost$ T | ValidTgts$ Player | SubAbility$ DBGainControl\n\
         SVar:DBGainControl:DB$ GainControl | ValidTgts$ Artifact.YouCtrl,Creature.YouCtrl,Land.YouCtrl | NewController$ ParentTarget",
    );
    let t = &trader.abilities[0];
    assert!(
        t.contains("Effect::WithSecondTarget(&[Effect::GainControlEx")
            && t.contains("to: PlayerRel::FirstTargetPlayerOrController")
            && t.contains("target = Some(TargetSpec::AnyPlayer)")
            && t.contains("second_targets = Some(TargetReq::one(TargetSpec::Object("),
        "Bazaar Trader: {t}"
    );

    // Assassin Gauntlet: Slot 1 up-to-1 Attach + Slot 2 Opponent TapAll.
    let gauntlet = read_w27c(
        "Name:Assassin Gauntlet\nManaCost:2 U\nTypes:Artifact Equipment\nK:Equip:2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigAttach\n\
         SVar:TrigAttach:DB$ Attach | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBTapAll\n\
         SVar:DBTapAll:DB$ TapAll | ValidTgts$ Opponent | ValidCards$ Creature",
    );
    let g = &gauntlet.abilities[1];
    assert!(
        g.contains("Effect::WithSecondTarget(&[Effect::TapAllOf { who: PlayerRel::Chosen")
            && g.contains("targets = Some(TargetReq::up_to_one(")
            && g.contains("second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"),
        "Assassin Gauntlet: {g}"
    );

    // Cybernetica Datasmith: Slot 1 Player Draw + Slot 2 Distinct Player Token.
    let datasmith = read_w27c(
        "Name:Cybernetica Datasmith\nManaCost:1 U B\nTypes:Artifact Creature\nPT:0/1\n\
         A:AB$ Draw | ValidTgts$ Player | Cost$ U T | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_4_4_a_robot_noblock | ValidTgts$ Player | TargetUnique$ True | TokenOwner$ ThisTargetedPlayer",
    );
    let d = &datasmith.abilities[0];
    assert!(
        d.contains("Effect::WithSecondTarget(&[Effect::CreateTokensFor")
            && d.contains("TargetConstraint::DistinctFromPriorTargetGroups"),
        "Cybernetica Datasmith: {d}"
    );
}

// ---------------------------------------------------------------------------
// 6. 3-slot independent target chains & Effect::WithAllTargets
// ---------------------------------------------------------------------------

#[test]
fn multi_target_three_slot_arm_the_cathars_boom_box_and_convert_to_slime() {
    // Arm the Cathars: 3-slot distinct Pump (+3/+3, +2/+2, +1/+1) + trailing Defined$ Targeted (WithAllTargets).
    let arm = read_w27c(
        "Name:Arm the Cathars\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | KW$ Vigilance | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | NumAtt$ +2 | NumDef$ +2 | KW$ Vigilance | TargetUnique$ True | SubAbility$ DBPump2\n\
         SVar:DBPump2:DB$ Pump | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | NumAtt$ +1 | NumDef$ +1 | KW$ Vigilance | TargetUnique$ True | SubAbility$ DBPump3\n\
         SVar:DBPump3:DB$ Pump | Defined$ Targeted | KW$ Vigilance",
    );
    let a = &arm.abilities[0];
    assert!(
        (a.contains("Effect::PumpTargetSlot { slot: TargetSlot::Second")
            || a.contains("Effect::WithSecondTarget(&["))
            && a.contains("Effect::WithThirdTarget(&[")
            && a.contains("Effect::WithAllTargets(&[")
            && a.contains("second_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE)).with_constraint(TargetConstraint::DistinctFromPriorTargetGroups))")
            && a.contains("third_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE)).with_constraint(TargetConstraint::DistinctFromPriorTargetGroups))"),
        "Arm the Cathars: {a}"
    );

    // Boom Box: 3-slot TargetOnly (Artifact, Creature, Land) -> Destroy | Defined$ Targeted (WithAllTargets).
    let boom = read_w27c(
        "Name:Boom Box\nManaCost:2\nTypes:Artifact\n\
         A:AB$ TargetOnly | Cost$ 6 T Sac<1/CARDNAME> | ValidTgts$ Artifact | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DestroyCreature\n\
         SVar:DestroyCreature:DB$ TargetOnly | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DestroyLand\n\
         SVar:DestroyLand:DB$ TargetOnly | ValidTgts$ Land | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ Targeted",
    );
    let b = &boom.abilities[0];
    assert!(
        b.contains("Effect::WithAllTargets(&[Effect::destroy(TargetSpec::Object(&Filter::Any))])")
            && b.contains(
                "targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::ARTIFACT)))"
            )
            && b.contains(
                "second_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE)))"
            )
            && b.contains(
                "third_targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::LAND)))"
            ),
        "Boom Box: {b}"
    );

    // Convert to Slime: 3-slot TargetOnly + Destroy(Defined$ Targeted, RememberDestroyed) + Token(RememberedManaValue)!
    let slime = read_w27c(
        "Name:Convert to Slime\nManaCost:3 B G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Artifact | SubAbility$ DBTarget1\n\
         SVar:DBTarget1:DB$ TargetOnly | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Creature | SubAbility$ DBTarget2\n\
         SVar:DBTarget2:DB$ TargetOnly | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Enchantment | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ Targeted | RememberDestroyed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | Condition$ Delirium | TokenScript$ g_x_x_ooze | TokenOwner$ You | TokenPower$ X | TokenToughness$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    let s = &slime.abilities[0];
    assert!(
        s.contains("Effect::CaptureAffected { first: &[Effect::WithAllTargets(&[Effect::destroy(TargetSpec::Object(&Filter::Any))])]")
            && s.contains("Amount::RememberedManaValue")
            && s.contains("third_targets = Some("),
        "Convert to Slime: {s}"
    );
}

#[test]
fn remembered_animate_delayed_zones_and_nested_optional_hand_choice() {
    let dance = read_w27c(
        "Name:Remembered Dance\nManaCost:4 B R\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl | RememberChanged$ True | ActivationPhases$ BeginCombat->EndCombat | SubAbility$ FirstAnimate\n\
         SVar:FirstAnimate:DB$ Animate | Keywords$ Haste | Duration$ Permanent | Defined$ Remembered | AtEOT$ Hand | SubAbility$ SecondMove\n\
         SVar:SecondMove:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | Optional$ You | ChangeType$ Creature | ChangeNum$ 1 | RememberChanged$ True | ForgetOtherRemembered$ True | SubAbility$ SecondAnimate\n\
         SVar:SecondAnimate:DB$ Animate | Keywords$ Haste | Duration$ Permanent | Defined$ Remembered | AtEOT$ SacrificeCtrl | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    let ability = &dance.abilities[0];
    assert!(
        ability.contains("Effect::CaptureAffected")
            && ability.contains("Effect::ForEachRemembered")
            && ability.contains("Modifier::AddKeyword(KeywordSet::HASTE)")
            && ability.contains("Effect::ReturnToHand { target: TargetSpec::EventObject }")
            && ability.contains("Effect::SacrificeEvent")
            && ability.contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && ability.contains("min: 0, max: Amount::Fixed(1)")
            && ability.contains("condition = Some(Condition::DuringCombat)"),
        "{ability}"
    );
}

// ---------------------------------------------------------------------------
// 7. Honest refusals & Pure Paper-MTG guardrails
// ---------------------------------------------------------------------------

#[test]
fn wave27c_honest_refusals_and_pure_paper_guardrails() {
    for bad in [
        // Alchemy / Digital: Perpetual, MakeCard/Conjure, Seek must stay refused.
        "Name:BadPerpetualRem\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Hand | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | PumpZone$ Hand | Defined$ Remembered | NumAtt$ +1 | Duration$ Perpetual | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:BadConjureRem\nManaCost:4 B\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBConjure\n\
         SVar:DBConjure:DB$ MakeCard | Conjure$ True | ConditionDefined$ Remembered | ConditionPresent$ Card | Name$ Skeleton | Zone$ Graveyard | Amount$ 3 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:BadSeekRem\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBSeek\n\
         SVar:DBSeek:DB$ Seek | Type$ Card.nonLand | Num$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
        // Cross-ability persistent memory (no SubAbility$ / ForgetOnMoved$ / DB$ Effect).
        "Name:BadPersistentExile\nManaCost:2 U U\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Artifact | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True",
        "Name:BadForgetOnMoved\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | ForgetOnMoved$ Exile | StaticAbilities$ ST | SubAbility$ DBCleanup\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        // Permanent trigger omitting Cleanup | ClearRemembered$ True leaks across resolutions.
        "Name:BadUnclosedTriggerRem\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 1 | RememberMilled$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | LifeAmount$ X\n\
         SVar:X:Remembered$CardManaCost",
        // Nested RememberChanged$ True WITHOUT ForgetOtherRemembered$ True (append semantics).
        "Name:BadAppendRemembered\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 2 | RememberMilled$ True | SubAbility$ DBMill2\n\
         SVar:DBMill2:DB$ Mill | NumCards$ 2 | RememberMilled$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
        // RememberControlled$ without a SubAbility$ must stay refused.
        "Name:BadRememberControlled\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Opponent | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainControl | ValidTgts$ Permanent.YouCtrl | NewController$ ParentTarget | RememberControlled$ True",
        // RememberedOwner (not a PlayerRel variant).
        "Name:BadRememberedOwner\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature | RememberDestroyed$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ RememberedOwner | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        // Multi-object first with narrowed ConditionPresent$ (cannot count filtered subset with RememberedCount).
        "Name:BadNarrowedMultiCond\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ SacrificeAll | ValidCards$ Permanent.YouCtrl | RememberSacrificed$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Land | ConditionCompare$ GE2 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        // Multi-player first + multi-player then reading Remembered$Amount (conflates per-player counts).
        "Name:BadMultiPlayerCount\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ Player | Mode$ Hand | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Player | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
        // 4 independent target slots (Decimate) must stay refused.
        "Name:Decimate\nManaCost:2 R G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Artifact | SubAbility$ S2\n\
         SVar:S2:DB$ TargetOnly | ValidTgts$ Creature | SubAbility$ S3\n\
         SVar:S3:DB$ TargetOnly | ValidTgts$ Enchantment | SubAbility$ S4\n\
         SVar:S4:DB$ TargetOnly | ValidTgts$ Land | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ Targeted",
        // 3-target Charm mode must stay refused (SpellMode has no third_targets).
        "Name:BadThreeTargetCharm\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ Charm | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | LifeAmount$ 3\n\
         SVar:M2:DB$ Pump | ValidTgts$ Creature | NumAtt$ +1 | SubAbility$ P2\n\
         SVar:P2:DB$ Pump | ValidTgts$ Creature | NumAtt$ +1 | SubAbility$ P3\n\
         SVar:P3:DB$ Pump | ValidTgts$ Creature | NumAtt$ +1",
    ] {
        assert!(refused_w27c(bad), "must refuse: {bad}");
    }
}

#[test]
fn capture_random_discard_return_with_any_player_life_payment() {
    let card = read_w27c(
        "Name:Synthetic Rift\nManaCost:1 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigDiscard | TriggerZones$ Battlefield\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ You | Mode$ Random | RememberDiscarded$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Graveyard | Destination$ Battlefield | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | ConditionCompare$ EQ1 | UnlessCost$ PayLife<5> | UnlessPayer$ Player | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let ability = &card.abilities[0];
    assert!(
        ability.contains("Effect::CaptureAffected")
            && ability.contains("Effect::DiscardRandom")
            && ability.contains("Effect::AnyPlayerMayPayLifeOr { life: 5")
            && ability.contains("Effect::ForEachRemembered")
            && ability.contains("TargetSpec::EventObject"),
        "{ability}"
    );
    let condition = ability
        .find("Effect::ForEachRemembered { filter: &Filter::CREATURE")
        .unwrap_or_else(|| panic!("missing remembered creature condition: {ability}"));
    let payment = ability
        .find("Effect::AnyPlayerMayPayLifeOr")
        .expect("payment question");
    let movement = ability.find("Effect::MoveAndModify").expect("zone change");
    assert!(
        condition < payment && payment < movement,
        "the creature condition must gate the payment question: {ability}"
    );
}
