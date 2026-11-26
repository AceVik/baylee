use super::*;

#[test]
fn name_card_therapy_mise_foreshadow_brain_pry_unlock() {
    // 1. Cabal Therapy: NameCard(Card.nonLand) -> RevealDiscardAll(Card.NamedCard)
    let therapy = read(
        "Name:Cabal Therapy\nManaCost:B\nTypes:Sorcery\n\
         K:Flashback:Sac<1/Creature>\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand | ValidDescription$ nonland | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | ValidTgts$ Player | Mode$ RevealDiscardAll | DiscardValid$ Card.NamedCard",
    );
    assert!(
        therapy.abilities.iter().any(|a| {
            a.contains("excluded_types: TypeSet::LAND")
                && a.contains("Effect::RevealHand { who: PlayerRel::Chosen }")
                && a.contains("Effect::DiscardMatching { who: PlayerRel::Chosen")
                && a.contains("Filter::NamedBySource")
        }),
        "{:?}",
        therapy.abilities
    );

    // 2. Cabal Therapist: Main1 trigger -> ImmediateTrigger(Sac<1/Creature>) -> NameCard -> RevealDiscardAll -> Cleanup
    let therapist = read(
        "Name:Cabal Therapist\nManaCost:B\nTypes:Creature Horror\nPT:1/1\nK:Menace\n\
         T:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ DBImmediate\n\
         SVar:DBImmediate:AB$ ImmediateTrigger | Cost$ Sac<1/Creature> | Execute$ NameCard\n\
         SVar:NameCard:DB$ NameCard | Defined$ You | ValidCards$ Card.nonLand | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | ValidTgts$ Player | Mode$ RevealDiscardAll | DiscardValid$ Card.NamedCard | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearNamedCard$ True",
    );
    assert!(
        therapist.abilities.iter().any(|a| {
            a.contains("Effect::PlayerMayPayCostThen")
                && a.contains("CostPart::Sacrifice(&Filter::YOUR_CREATURE)")
                && a.contains("Effect::ReflexiveTrigger")
                && a.contains("Effect::ChooseCardName")
                && a.contains("excluded_types: TypeSet::LAND")
        }),
        "{:?}",
        therapist.abilities
    );

    // 3. Foreshadow: NameCard -> Mill(Opponent, 1) -> IfCondition(ThisWay::Milled(&Named), draw 1) -> AtNextTurnsUpkeep(draw 1)
    let foreshadow = read(
        "Name:Foreshadow\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ NameCard | SubAbility$ DBMill\n\
         SVar:DBMill:DB$ Mill | ValidTgts$ Opponent | NumCards$ 1 | RememberMilled$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card.NamedCard | SubAbility$ DelTrigSlowtrip\n\
         SVar:DelTrigSlowtrip:DB$ DelayedTrigger | NextTurn$ True | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ DrawSlowtrip | SubAbility$ DBCleanup\n\
         SVar:DrawSlowtrip:DB$ Draw\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearNamedCard$ True",
    );
    assert!(
        foreshadow.abilities[0].contains("ThisWay::Milled(&")
            && foreshadow.abilities[0].contains("Effect::AtNextTurnsUpkeep")
            && foreshadow.abilities[0].contains("TargetSpec::AnyOpponent"),
        "{}",
        foreshadow.abilities[0]
    );

    // 4. Brain Pry: NameCard -> RevealHand -> ChooseCardEx(Hand, Named, 1..=1, DiscardDefined) -> IfCondition(ThisWay::Discarded == 0, draw 1)
    let brain_pry = read(
        "Name:Brain Pry\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand | SubAbility$ RevealHand\n\
         SVar:RevealHand:DB$ RevealHand | RememberRevealed$ True | ValidTgts$ Player | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ Targeted | NumCards$ 1 | Mode$ TgtChoose | DiscardValid$ Card.NamedCard | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card.NamedCard | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearNamedCard$ True",
    );
    assert!(
        brain_pry.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && brain_pry.abilities[0]
                .contains("Effect::DiscardDefined { cards: DefinedObjects::Found }")
            && brain_pry.abilities[0].contains("ThisWay::Discarded"),
        "{}",
        brain_pry.abilities[0]
    );
}

#[test]
fn name_card_petra_and_hamlet_unlock() {
    // 5. Petra Sphinx: ChooseCardName(Chosen) -> RevealTopByName(Chosen, Hand, Graveyard)
    let petra = read(
        "Name:Petra Sphinx\nManaCost:2 W W W\nTypes:Creature Sphinx\nPT:3/4\n\
         A:AB$ NameCard | Cost$ T | ValidTgts$ Player | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | DigNum$ 1 | Defined$ Targeted | ChangeNum$ All | ChangeValid$ Card.NamedCard | DestinationZone2$ Graveyard | Reveal$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearNamedCard$ True",
    );
    assert!(
        petra.abilities[0].contains("Effect::ChooseCardName { player: PlayerRel::Chosen")
            && petra.abilities[0].contains("Effect::RevealTopByName { player: PlayerRel::Chosen, matched: SearchDest::Hand, otherwise: SearchDest::Graveyard }"),
        "{}",
        petra.abilities[0]
    );

    // 6. Petrified Hamlet: ETB trigger NameCard(Land, remember: true) + static readers
    let hamlet = read(
        "Name:Petrified Hamlet\nManaCost:no cost\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigNameCard\n\
         SVar:TrigNameCard:DB$ NameCard | Defined$ You | ValidCards$ Land\n\
         S:Mode$ CantBeActivated | ValidCard$ Card.NamedCard | ValidSA$ Activated.!ManaAbility\n\
         S:Mode$ Continuous | Affected$ Land.NamedCard | AddAbility$ ColorlessMana\n\
         SVar:ColorlessMana:AB$ Mana | Cost$ T | Produced$ C\n\
         A:AB$ Mana | Cost$ T | Produced$ C",
    );
    assert!(
        hamlet
            .abilities
            .iter()
            .any(|a| { a.contains("types: TypeSet::LAND") && a.contains("remember: true") })
            && hamlet
                .abilities
                .iter()
                .any(|a| a.contains("Modifier::ChosenNameCantActivate"))
            && hamlet
                .abilities
                .iter()
                .any(|a| a.contains("Modifier::GrantActivated")),
        "{:?} / {}",
        hamlet.abilities,
        hamlet.statics
    );
}

#[test]
fn name_card_lobotomy_suite_unlock() {
    // 7. Lobotomy suite (Slaughter Games, Lost Legacy)
    let slaughter = read(
        "Name:Slaughter Games\nManaCost:2 B R\nTypes:Sorcery\n\
         R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand | SubAbility$ ExileYard\n\
         SVar:ExileYard:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player.Opponent | ChangeType$ Card.NamedCard | Chooser$ You | ChangeNum$ NumInYard | Hidden$ True | SubAbility$ ExileHand\n\
         SVar:ExileHand:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInHand | Chooser$ You | SubAbility$ ExileLib\n\
         SVar:ExileLib:DB$ ChangeZone | Origin$ Library | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInLib | Chooser$ You | Shuffle$ True\n\
         SVar:NumInLib:TargetedPlayer$CardsInLibrary\n\
         SVar:NumInHand:TargetedPlayer$CardsInHand\n\
         SVar:NumInYard:TargetedPlayer$CardsInGraveyard\n\
         Oracle:This spell can't be countered.\\nChoose a nonland card name. Search target opponent's graveyard, hand, and library for any number of cards with that name and exile them. Then that player shuffles.",
    );
    assert!(
        slaughter.abilities.iter().any(|a| {
            a.contains("Effect::LookAtChosenHand")
                && a.contains(
                    "Effect::SearchLibraryOf { library: PlayerRel::Chosen, owner_searches: false",
                )
                && a.contains("TargetSpec::AnyOpponent")
        }),
        "{:?}",
        slaughter.abilities
    );

    let cranial = read(
        "Name:Cranial Extraction\nManaCost:3 B\nTypes:Sorcery Arcane\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand | SubAbility$ ExileYard\n\
         SVar:ExileYard:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player | ChangeType$ Card.NamedCard | Chooser$ You | ChangeNum$ NumInYard | Hidden$ True | SubAbility$ ExileHand\n\
         SVar:ExileHand:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInHand | Chooser$ You | SubAbility$ ExileLib\n\
         SVar:ExileLib:DB$ ChangeZone | Origin$ Library | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInLib | Chooser$ You | Shuffle$ True\n\
         SVar:NumInLib:TargetedPlayer$CardsInLibrary\n\
         SVar:NumInHand:TargetedPlayer$CardsInHand\n\
         SVar:NumInYard:TargetedPlayer$CardsInGraveyard\n\
         Oracle:Choose a nonland card name. Search target player's graveyard, hand, and library for all cards with that name and exile them. Then that player shuffles.",
    );
    assert!(
        cranial.abilities[0].contains(
            "zone: ChooseZone::Graveyard(PlayerRel::Chosen), filter: &Filter::NamedBySource, min: 0, max: Amount::Fixed(0), reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::Exile"
        ) && cranial.abilities[0].contains(
            "zone: ChooseZone::Hand(PlayerRel::Chosen), filter: &Filter::NamedBySource, min: 0, max: Amount::Fixed(0), reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::Exile"
        ),
        "{}",
        cranial.abilities[0]
    );

    let lost_legacy = read(
        "Name:Lost Legacy\nManaCost:1 B B\nTypes:Sorcery\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand+nonArtifact | SubAbility$ ExileHand\n\
         SVar:ExileHand:DB$ ChangeZone | ValidTgts$ Player | Origin$ Hand | Destination$ Exile | DefinedPlayer$ TargetedPlayer | ChangeType$ Card.NamedCard | ChangeNum$ NumInHand | Chooser$ You | SubAbility$ ExileLib | RememberChanged$ True\n\
         SVar:ExileLib:DB$ ChangeZone | Origin$ Library | Destination$ Exile | DefinedPlayer$ TargetedPlayer | ChangeType$ Card.NamedCard | ChangeNum$ NumInLib | Chooser$ You | Shuffle$ True | SubAbility$ ExileYard\n\
         SVar:ExileYard:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | DefinedPlayer$ TargetedPlayer | ChangeType$ Card.NamedCard | ChangeNum$ NumInYard | Chooser$ You | Hidden$ True | SubAbility$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ TargetedPlayer | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$RememberedSize\n\
         SVar:NumInHand:TargetedPlayer$CardsInHand\n\
         SVar:NumInLib:TargetedPlayer$CardsInLibrary\n\
         SVar:NumInYard:TargetedPlayer$CardsInGraveyard\n\
         Oracle:Choose a nonartifact, nonland card name. Search target player's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.",
    );
    assert!(
        lost_legacy.abilities[0].contains("Effect::CaptureAffected")
            && lost_legacy.abilities[0]
                .contains("excluded_types: TypeSet::LAND.union(TypeSet::ARTIFACT)")
            && lost_legacy.abilities[0].contains(
                "zone: ChooseZone::Hand(PlayerRel::Chosen), filter: &Filter::NamedBySource, min: 0, max: Amount::Fixed(u8::MAX as u32), reveal_chosen: false, movement: ChooseMove::Exile, not_chosen_movement: ChooseMove::None"
            )
            && lost_legacy.abilities[0].contains(
                "Effect::DrawCardsFor { who: PlayerRel::Chosen, amount: Amount::RememberedCount }"
            ),
        "{}",
        lost_legacy.abilities[0]
    );
}

#[test]
fn choose_card_delayed_target_and_psychic_purge_unlocks_and_refusals() {
    // 1. Cryptek & Together Forever (`ChooseCard` with ValidTgts$ -> DelayedTrigger)
    let cryptek = read(
        "Name:Cryptek\nManaCost:3 B\nTypes:Artifact Creature Necron Wizard\nPT:3/3\n\
         A:AB$ ChooseCard | Cost$ 1 B T | ValidTgts$ Artifact.Creature+YouCtrl+Other | SubAbility$ DBDelayedTrigger\n\
         SVar:DBDelayedTrigger:DB$ DelayedTrigger | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ TriggeredCard | Tapped$ True | GainControl$ True",
    );
    assert!(
        cryptek.abilities[0].contains("Effect::RegisterDelayedTrigger")
            && cryptek.abilities[0].contains("TriggerRemember::FirstTarget")
            && cryptek.abilities[0].contains("Effect::GraveyardToBattlefieldModified")
            && cryptek.abilities[0].contains("owner_control: false")
            && cryptek.abilities[0].contains("tapped: true"),
        "{}",
        cryptek.abilities[0]
    );

    let together = read(
        "Name:Together Forever\nManaCost:W W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | Support$ 2\n\
         A:AB$ ChooseCard | Cost$ 1 | ValidTgts$ Creature.HasCounters | SubAbility$ DBDelayedTrigger\n\
         SVar:DBDelayedTrigger:DB$ DelayedTrigger | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ TriggeredCard",
    );
    assert!(
        together.abilities[1]
            .contains("Effect::GraveyardToHand { target: TargetSpec::EventObject }"),
        "{}",
        together.abilities[1]
    );

    // 2. Psychic Purge: TriggeredCauseController on self-Discarded by opponent -> PlayerRel::EventPlayer
    let purge = read(
        "Name:Psychic Purge\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1\n\
         T:Mode$ Discarded | ValidCard$ Card.Self | ValidCause$ SpellAbility.OppCtrl | Execute$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ TriggeredCauseController | LifeAmount$ 5",
    );
    assert!(
        purge.abilities[1].contains("Trigger::DiscardedThisByOpponent")
            && purge.abilities[1].contains("target: PlayerRel::EventPlayer"),
        "{}",
        purge.abilities[1]
    );

    // 3. Refusal guards:
    assert!(refused(
        "Name:Academic Probation\nManaCost:1 W\nTypes:Sorcery Lesson\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.nonLand | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ CantCast | Duration$ UntilYourNextTurn\n\
         SVar:CantCast:Mode$ CantBeCast | ValidCard$ Card.nonLand+NamedCard | Caster$ Opponent"
    ));
    assert!(refused(
        "Name:Day of the Moon\nManaCost:2 R\nTypes:Enchantment Saga\n\
         K:Chapter:3:DBChoose,DBChoose,DBChoose\n\
         SVar:DBChoose:DB$ NameCard | ValidCards$ Creature | SubAbility$ DBGoad\n\
         SVar:DBGoad:DB$ Goad | Defined$ Valid Creature.NamedCard"
    ));
    assert!(refused(
        "Name:Dispossess\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ NameCard | Defined$ You | ValidCards$ Card.Artifact | SubAbility$ ExileYard\n\
         SVar:ExileYard:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player | ChangeType$ Card.NamedCard | Chooser$ You | ChangeNum$ NumInYard | Hidden$ True | SubAbility$ ExileHand\n\
         SVar:ExileHand:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInHand | Chooser$ You | SubAbility$ ExileLib\n\
         SVar:ExileLib:DB$ ChangeZone | Origin$ Library | Destination$ Exile | DefinedPlayer$ Targeted | ChangeType$ Card.NamedCard | ChangeNum$ NumInLib | Chooser$ You | Shuffle$ True\n\
         SVar:NumInLib:TargetedPlayer$CardsInLibrary\n\
         SVar:NumInHand:TargetedPlayer$CardsInHand\n\
         SVar:NumInYard:TargetedPlayer$CardsInGraveyard\n\
         Oracle:Choose an artifact card name. Search target opponent's graveyard, hand, and library for any number of cards with the chosen name and exile them. Then that player shuffles."
    ));
    assert!(refused(
        "Name:Conundrum Sphinx\nManaCost:2 U U\nTypes:Creature Sphinx\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ EachName\n\
         SVar:EachName:DB$ NameCard | Defined$ Player"
    ));
    assert!(refused(
        "Name:Rushed Rebirth\nManaCost:B G\nTypes:Instant\n\
         A:SP$ ChooseCard | ValidTgts$ Creature | SubAbility$ DBDelayedTrigger\n\
         SVar:DBDelayedTrigger:DB$ DelayedTrigger | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Creature.cmcLTX | ChangeNum$ 1 | Tapped$ True\n\
         SVar:X:TriggerRemembered$CardManaCost"
    ));
    assert!(refused(
        "Name:BadCauseCtrl\nManaCost:B\nTypes:Enchantment\n\
         T:Mode$ Discarded | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseLife | Defined$ TriggeredCauseController | LifeAmount$ 2"
    ));
}
