use super::*;

#[test]
fn hand_and_draw_draw_unlocks_and_refusals() {
    // 1. `OptionalDecider$ True` for single-seat non-You (`Harbor Guardian`).
    let harbor = read(
        "Name:Harbor Guardian\nManaCost:2 W U\nTypes:Creature Gargoyle\nPT:3/4\n\
         K:Reach\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ TriggeredDefendingPlayer | NumCards$ 1 | OptionalDecider$ True",
    );
    assert!(
        harbor.abilities[0].contains(
            "Effect::MayDoFor { player: PlayerRel::DefendingPlayer, \
             effects: &[Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::DefendingPlayer }] }"
        ),
        "{:?}",
        harbor.abilities
    );

    // 2. `OptionalDecider$ True` for multi-seat `Defined$ Opponent`.
    let opt_opp = read(
        "Name:OptEachOpp\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ Opponent | NumCards$ 1 | OptionalDecider$ True",
    );
    assert!(
        opt_opp.abilities[0].contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachOpponent, \
             effects: &[Effect::MayDoFor { player: PlayerRel::EventPlayer, \
             effects: &[Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EventPlayer }] }] }"
        ),
        "{:?}",
        opt_opp.abilities
    );

    // 3. `LandPlayed` `TriggeredCardController` (`Horn of Greed`).
    let horn = read(
        "Name:Horn of Greed\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ LandPlayed | ValidCard$ Land | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ TriggeredCardController | NumCards$ 1",
    );
    assert!(
        horn.abilities[0].contains("Trigger::PlaysLand(PlayerRel::EachPlayer)")
            && horn.abilities[0].contains(
                "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EventPlayer }"
            ),
        "{:?}",
        horn.abilities
    );

    // 4. Trailing Slot-2 `Draw | Defined$ TargetedPlayer` (`Cease // Desist` front half).
    let cease = read(
        "Name:Cease\nManaCost:1 BG\nTypes:Instant\n\
         A:SP$ ChangeZone | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ True | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | ValidTgts$ Player | LifeAmount$ 2 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ TargetedPlayer\n\
         AlternateMode:Split\n\
         ALTERNATE\n\
         Name:Desist\nManaCost:4 GW GW\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Artifact,Enchantment",
    );
    assert!(
        cease.abilities[0].contains(
            "Effect::WithSecondTarget(&[Effect::GainLifeFor { amount: Amount::Fixed(2), who: PlayerRel::Chosen }]), \
             Effect::WithSecondTarget(&[Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::Chosen }])"
        ),
        "{:?}",
        cease.abilities
    );

    // 5. `RemoveCounter | CounterNum$ All | RememberRemoved$ True` -> `Draw | NumCards$ Count$RememberedSize` (`Give // Take` back half).
    let take = read(
        "Name:Take\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ RemoveCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ All | RememberRemoved$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$RememberedSize",
    );
    assert!(
        take.abilities[0].contains("Effect::RemoveAllCountersFilter { filter: &Filter::Reference, kind: CounterKind::P1P1 }")
            && take.abilities[0].contains("Effect::DrawCards { amount: Amount::ThisWay(ThisWay::CountersRemovedFromTargets) }"),
        "{:?}",
        take.abilities
    );

    // Refusals: `RememberRemoved$ True` followed by `IsRemembered` (wrong consumer) and unsupported `TriggeredCardController` decider.
    assert!(refused(
        "Name:BadRem\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ RemoveCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ All | RememberRemoved$ True | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Defined$ Remembered | Origin$ Battlefield | Destination$ Hand"
    ));
    assert!(refused(
        "Name:BadDecider\nManaCost:U\nTypes:Artifact\n\
         T:Mode$ Drawn | ValidCard$ Card | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | OptionalDecider$ TriggeredCardController"
    ));
}

#[test]
fn hand_and_draw_discard_reveal_subset_you_choose() {
    // 1. Fixed subset reveal (`Blackmail`, `Noggin Whack`).
    let blackmail = read(
        "Name:Blackmail\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | RevealNumber$ 3 | NumCards$ 1\n\
         Oracle:Target player reveals three cards from their hand and you choose one of them. That player discards that card.",
    );
    assert!(
        blackmail.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::RevealFromHand { who: PlayerRel::Chosen, filter: &Filter::Any, min: u8::MAX, max: Amount::Fixed(3), if_revealed: &[] }]")
            && blackmail.abilities[0].contains("zone: ChooseZone::Remembered")
            && blackmail.abilities[0].contains("min: 1, max: Amount::Fixed(1)")
            && blackmail.abilities[0].contains("Effect::DiscardDefined { cards: DefinedObjects::Found }"),
        "{:?}",
        blackmail.abilities
    );

    let noggin = read(
        "Name:Noggin Whack\nManaCost:2 B B\nTypes:Kindred Sorcery Rogue\n\
         K:Prowl:1 B\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | RevealNumber$ 3 | NumCards$ 2\n\
         Oracle:Prowl {1}{B}\\nTarget player reveals three cards from their hand. You choose two of them. That player discards those cards.",
    );
    assert!(
        noggin.abilities[0].contains("min: 2, max: Amount::Fixed(2)"),
        "{:?}",
        noggin.abilities
    );

    // 2. Dynamic `RevealNumber$` (`Mire's Toll`, `Hollow Specter` with ResolutionChosenNumber).
    let mires = read(
        "Name:Mire's Toll\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | RevealNumber$ X | NumCards$ 1\n\
         SVar:X:Count$Valid Mountain.YouCtrl\n\
         Oracle:Target player reveals a number of cards from their hand equal to the number of Mountains you control. You choose one of them. That player discards that card.",
    );
    assert!(
        mires.abilities[0].contains("max: Amount::CountOf")
            && mires.abilities[0].contains("zone: ChooseZone::Remembered"),
        "{:?}",
        mires.abilities
    );

    let specter = read(
        "Name:Hollow Specter\nManaCost:1 B B\nTypes:Creature Specter\nPT:2/2\nK:Flying\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | OptionalDecider$ You | Execute$ TrigDiscard\n\
         SVar:TrigDiscard:AB$ Discard | Cost$ X | Defined$ TriggeredTarget | NumCards$ 1 | RevealNumber$ X | Mode$ RevealYouChoose | DiscardValid$ Card\n\
         SVar:X:Count$xPaid\n\
         Oracle:Flying\\nWhenever Hollow Specter deals combat damage to a player, you may pay {X}. If you do, that player reveals X cards from their hand and you choose one of them. That player discards that card.",
    );
    assert!(
        specter.abilities[0].contains("max: Amount::ResolutionChosenNumber")
            && specter.abilities[0].contains("who: PlayerRel::DamagedPlayer"),
        "{:?}",
        specter.abilities
    );
}

#[test]
fn hand_and_draw_reveal_hand_unlocks_and_refusals() {
    // 1. `Inquisition`: public RevealHand + Remembered$Valid count -> DealDamage.
    let inquisition = read(
        "Name:Inquisition\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Player | RememberRevealed$ True | SubAbility$ Damage\n\
         SVar:Damage:DB$ DealDamage | Defined$ Targeted | NumDmg$ X\n\
         SVar:X:Remembered$Valid Card.White\n\
         Oracle:Target player reveals their hand. Inquisition deals damage to that player equal to the number of white cards in their hand.",
    );
    assert!(
        inquisition.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && inquisition.abilities[0].contains("Effect::DealDamage { amount: Amount::CountOf { filter: &HAND_COUNT1, zone: ZoneSel::HandOf(PlayerRel::Chosen) }"),
        "{:?}",
        inquisition.abilities
    );

    // 2. `Revealing Eye` back half: optional nonland discard + ChosenCard-gated draw.
    let eye = read(
        "Name:Revealing Eye\nManaCost:no cost\nColors:black\nTypes:Creature Eye Horror\nPT:3/4\nK:Menace\n\
         T:Mode$ Transformed | ValidCard$ Card.Self | Execute$ TrigReveal\n\
         SVar:TrigReveal:DB$ RevealHand | ValidTgts$ Opponent | RememberRevealed$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | ChoiceZone$ Hand | Amount$ 1 | Choices$ Card.nonLand+IsRemembered | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | DefinedCards$ ChosenCard | Defined$ Targeted | Mode$ Defined | ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ EQ1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Targeted | NumCards$ 1 | ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ EQ1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True\n\
         Oracle:Menace\\nWhen this creature transforms into Revealing Eye, target opponent reveals their hand. You may choose a nonland card from it. If you do, that player discards that card, then draws a card.",
    );
    assert!(
        eye.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && eye.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && eye.abilities[0].contains("then: &[Effect::DiscardDefined { cards: DefinedObjects::Found }, Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::Chosen }]"),
        "{:?}",
        eye.abilities
    );

    // 3. `Specter's Shriek`: optional nonland exile + nonblack exiled gate -> exile a card from your hand.
    let shriek = read(
        "Name:Specter's Shriek\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent | RememberRevealed$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | ChoiceZone$ Hand | Choices$ Card.nonLand+IsRemembered | SubAbility$ DBExile | MinAmount$ 0\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | Defined$ ChosenCard | Mandatory$ True | SubAbility$ DBExile2\n\
         SVar:DBExile2:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ConditionDefined$ ChosenCard | ConditionPresent$ Card.nonBlack | ConditionCompare$ GE1 | Mandatory$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True\n\
         Oracle:Target opponent reveals their hand. You may choose a nonland card from it. If you do, that player exiles that card. If a nonblack card is exiled this way, exile a card from your hand.",
    );
    assert!(
        shriek.abilities[0].contains("movement: ChooseMove::Exile")
            && shriek.abilities[0].contains("Effect::IfEventObjectMatches")
            && shriek.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && shriek.statics.contains("Filter::InZone(ZoneRef::Exile)"),
        "{:?}\n{}",
        shriek.abilities,
        shriek.statics
    );

    // 4. `Talara's Bane`: gain toughness life + discard chosen card.
    let talara = read(
        "Name:Talara's Bane\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent | RememberRevealed$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | ChoiceZone$ Hand | Mandatory$ True | Choices$ Creature.Green+IsRemembered,Creature.White+IsRemembered | SubAbility$ DBImprint\n\
         SVar:DBImprint:DB$ Bookkeeping | Defined$ ChosenCard | ImprintCards$ ChosenCard | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | SubAbility$ DBDiscard | ConditionDefined$ Imprinted | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:DBDiscard:DB$ Discard | DefinedCards$ Imprinted | Defined$ Targeted | Mode$ Defined | SubAbility$ DBCleanup | ConditionDefined$ Imprinted | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:Imprinted$CardToughness\n\
         Oracle:Target opponent reveals their hand. You choose a green or white creature card from it. You gain life equal to that creature card's toughness, then that player discards that card.",
    );
    assert!(
        talara.abilities[0].contains("Effect::GainLife { amount: Amount::RememberedToughness }")
            && talara.abilities[0].contains(
                "Effect::DiscardDefined { cards: DefinedObjects::Spec(TargetSpec::EventObject) }"
            ),
        "{:?}",
        talara.abilities
    );

    // Refusals: Nightsnare false-branch ("if you don't") remains refused rather than approximated.
    assert!(refused(
        "Name:Nightsnare\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent | RememberRevealed$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | ChoiceZone$ Hand | Amount$ 1 | Choices$ Card.nonLand+IsRemembered | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ Count$ChosenSize | TrueSubAbility$ DBDiscard | FalseSubAbility$ DBDiscard2\n\
         SVar:DBDiscard:DB$ Discard | DefinedCards$ ChosenCard | Defined$ Targeted | Mode$ Defined | SubAbility$ DBCleanup\n\
         SVar:DBDiscard2:DB$ Discard | Defined$ Targeted | NumCards$ 2 | Mode$ TgtChoose | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True\n\
         Oracle:Target opponent reveals their hand. You may choose a nonland card from it. If you do, that player discards that card. If you don't, that player discards two cards."
    ));
}

#[test]
fn hand_and_draw_peek_and_reveal_unlocks_and_refusals() {
    // 1. Parley `Mana` (`Selvala, Explorer Returned`).
    let selvala = read(
        "Name:Selvala, Explorer Returned\nManaCost:1 G W\nTypes:Legendary Creature Elf Scout\nPT:2/4\n\
         A:AB$ PeekAndReveal | Cost$ T | Defined$ Player | RememberRevealed$ True | SubAbility$ DBMana\n\
         SVar:DBMana:DB$ Mana | Produced$ G | Amount$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Player | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.nonLand",
    );
    assert!(
        selvala.abilities[0].contains("Effect::AddManaAmountFor { who: PlayerRel::You, color: ManaColor::Green, amount: Amount::ThisWayBy { what: ThisWay::Revealed(")
            && selvala.abilities[0].contains("Effect::GainLife { amount: Amount::ThisWayBy { what: ThisWay::Revealed(")
            && selvala.abilities[0].contains("Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EachPlayer }"),
        "{:?}",
        selvala.abilities
    );

    // 2. `Prophecy`: `RevealTop` with `ConditionDefined$ Remembered` + trailing non-remembered `Shuffle -> DelayedTrigger`.
    let prophecy = read(
        "Name:Prophecy\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ PeekAndReveal | ValidTgts$ Opponent | NoPeek$ True | RememberRevealed$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card.Land | ConditionCompare$ GE1 | SubAbility$ DBShuffle\n\
         SVar:DBShuffle:DB$ Shuffle | Defined$ ParentTarget | SubAbility$ DelTrigSlowtrip\n\
         SVar:DelTrigSlowtrip:DB$ DelayedTrigger | NextTurn$ True | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ DrawSlowtrip\n\
         SVar:DrawSlowtrip:DB$ Draw",
    );
    assert!(
        prophecy.abilities[0]
            .contains("Effect::RevealTop { player: PlayerRel::Chosen, count: Amount::Fixed(1)")
            && prophecy.abilities[0].contains("if_matches: &[Effect::gain_life(1)]")
            && prophecy.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::Chosen }")
            && prophecy.abilities[0]
                .contains("Effect::AtNextTurnsUpkeep { effects: &[Effect::draw(1)] }"),
        "{:?}",
        prophecy.abilities
    );

    // 3. `Cerebral Eruption`: CaptureAffected([RevealTop]) + Damage + ReturnToHand(ThisObject).
    let eruption = read(
        "Name:Cerebral Eruption\nManaCost:2 R R\nTypes:Sorcery\n\
         A:SP$ PeekAndReveal | ValidTgts$ Opponent | NoPeek$ True | RememberRevealed$ True | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DamageAll | ValidCards$ Creature.TargetedPlayerCtrl | ValidPlayers$ Targeted | NumDmg$ X | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Parent | Origin$ Stack | Destination$ Hand | ConditionDefined$ Remembered | ConditionPresent$ Land | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        eruption.abilities[0].contains(
            "Effect::CaptureAffected { first: &[Effect::RevealTop { player: PlayerRel::Chosen"
        ) && eruption.abilities[0]
            .contains("Effect::DealDamageEach { amount: Amount::RememberedManaValue")
            && eruption.abilities[0]
                .contains("Effect::ReturnToHand { target: TargetSpec::ThisObject }"),
        "{:?}",
        eruption.abilities
    );

    // 4. `Thran Tome`: CaptureAffected([RevealTop(3)]) -> ChooseCardEx(Chosen, Remembered -> Graveyard) + draw(2).
    let tome = read(
        "Name:Thran Tome\nManaCost:4\nTypes:Artifact Book\n\
         A:AB$ PeekAndReveal | Cost$ 5 T | PeekAmount$ 3 | NoPeek$ True | RememberRevealed$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | ValidTgts$ Opponent | Choices$ Card.IsRemembered | ChoiceZone$ Library | Amount$ 1 | SubAbility$ DBDig | Mandatory$ True\n\
         SVar:DBDig:DB$ ChangeZone | Origin$ Library | Destination$ Graveyard | Defined$ ChosenCard | Shuffle$ False | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 2 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        tome.abilities[0].contains("target = Some(TargetSpec::AnyOpponent)")
            && tome.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::RevealTop { player: PlayerRel::You, count: Amount::Fixed(3)")
            && tome.abilities[0].contains("chooser: PlayerRel::Chosen, zone: ChooseZone::Remembered")
            && tome.abilities[0].contains("movement: ChooseMove::Graveyard")
            && tome.abilities[0].contains("Effect::draw(2)"),
        "{:?}",
        tome.abilities
    );

    // 5. `Getaway Barrel`: CaptureAffected([RevealTop(13)]) -> ChooseCardConstrained(AtRandom -> Battlefield) + BottomOfLibraryRandom.
    let barrel = read(
        "Name:Getaway Barrel\nManaCost:3 R\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReveal\n\
         SVar:TrigReveal:DB$ PeekAndReveal | Defined$ You | PeekAmount$ 13 | RememberRevealed$ True | SubAbility$ DBRandomPick\n\
         SVar:DBRandomPick:DB$ ChooseCard | Defined$ You | Amount$ 1 | Choices$ Card.Creature+IsRemembered | ChoiceZone$ Library | AtRandom$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | Defined$ ChosenCard | SubAbility$ DBRestRandomOrder\n\
         SVar:DBRestRandomOrder:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True | ClearRemembered$ True",
    );
    assert!(
        barrel.abilities[0].contains("Effect::ChooseCardConstrained")
            && barrel.abilities[0].contains("at_random: true")
            && barrel.abilities[0].contains("movement: ChooseMove::Battlefield { tapped: false }")
            && barrel.abilities[0].contains("movement: ChooseMove::BottomOfLibraryRandom"),
        "{:?}",
        barrel.abilities
    );

    // Refusal: `Planar Genesis` (`NoReveal$ True | RememberPeeked$ True` + optional land-vs-hand fork) must stay refused.
    assert!(refused(
        "Name:Planar Genesis\nManaCost:G U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | Defined$ You | PeekAmount$ 4 | NoReveal$ True | RememberPeeked$ True | SubAbility$ PickOne\n\
         SVar:PickOne:DB$ ChooseCard | Defined$ You | Amount$ 1 | Choices$ Card.IsRemembered | Mandatory$ True | ChoiceZone$ Library | SubAbility$ DBChangeZoneBattlefield\n\
         SVar:DBChangeZoneBattlefield:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | Defined$ ChosenCard | ConditionDefined$ Remembered | ConditionPresent$ Card.Land+ChosenCard | ConditionCompare$ EQ1 | Tapped$ True | Optional$ True | SubAbility$ DBChangeZoneHand\n\
         SVar:DBChangeZoneHand:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Library | Destination$ Hand | ConditionDefined$ Remembered | ConditionPresent$ Card.ChosenCard+inZoneBattlefield | ConditionCompare$ EQ0 | SubAbility$ DBRestRandomOrder\n\
         SVar:DBRestRandomOrder:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True | ClearRemembered$ True"
    ));
}

#[test]
fn hand_and_draw_choose_card_unlocks_and_refusals() {
    // 1. Targeted `ChooseCard` without `Choices$` (`Cryptek` & `Mandate of Abaddon`).
    let cryptek = read(
        "Name:Cryptek\nManaCost:3 B\nTypes:Artifact Creature Necron Wizard\nPT:3/3\n\
         A:AB$ ChooseCard | Cost$ 1 B T | ValidTgts$ Artifact.Creature+YouCtrl+Other | SubAbility$ DBDelayedTrigger\n\
         SVar:DBDelayedTrigger:DB$ DelayedTrigger | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ TriggeredCard | Tapped$ True | GainControl$ True",
    );
    assert!(
        cryptek.abilities[0].contains("Trigger::Dies")
            && cryptek.abilities[0].contains("Effect::GraveyardToBattlefieldModified"),
        "{:?}",
        cryptek.abilities
    );

    let together = read(
        "Name:Together Forever\nManaCost:W W\nTypes:Enchantment\n\
         A:AB$ ChooseCard | Cost$ 1 | ValidTgts$ Creature.HasCounters | SubAbility$ DBDelayedTrigger\n\
         SVar:DBDelayedTrigger:DB$ DelayedTrigger | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ TriggeredCard",
    );
    assert!(
        together.abilities[0].contains("Effect::RegisterDelayedTrigger")
            && together.abilities[0]
                .contains("Effect::GraveyardToHand { target: TargetSpec::EventObject }"),
        "{:?}",
        together.abilities
    );
    let mandate_reason = refusal_reason(
        &parse(
            "Name:Mandate of Abaddon\nManaCost:3 B\nTypes:Sorcery\n\
             A:SP$ ChooseCard | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDestroyAll\n\
             SVar:DBDestroyAll:DB$ DestroyAll | ValidCards$ Creature.powerLTX\n\
             SVar:X:Targeted$CardPower",
        ),
        &cats(),
        None,
    );
    assert!(
        mandate_reason
            .as_deref()
            .is_some_and(|r| !r.contains("ChooseCard")),
        "ChooseCard wrapper should chain past ChooseCard: {mandate_reason:?}"
    );

    // 2. `Watchers of the Dead`: RepeatEach + ChooseCard(Graveyard, RememberChosen) + ChangeZoneAll(!IsRemembered -> Exile).
    let watchers = read(
        "Name:Watchers of the Dead\nManaCost:2\nTypes:Artifact Creature Cat\nPT:2/2\n\
         A:AB$ RepeatEach | Cost$ Exile<1/CARDNAME> | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ ChooseTwoGrave | SubAbility$ ExileAllOthers\n\
         SVar:ChooseTwoGrave:DB$ ChooseCard | Defined$ Remembered | Amount$ 2 | Choices$ Card.RememberedPlayerCtrl | ChoiceZone$ Graveyard | RememberChosen$ True | Mandatory$ True\n\
         SVar:ExileAllOthers:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card.!IsRemembered",
    );
    assert!(
        watchers.abilities[0].contains("RepeatDomain::EachOpponent")
            && watchers.abilities[0]
                .contains("zone: ChooseZone::Graveyard(PlayerRel::EventPlayer)")
            && watchers.abilities[0].contains("min: 2, max: Amount::Fixed(2)")
            && watchers.abilities[0].contains("not_chosen_movement: ChooseMove::Exile"),
        "{:?}",
        watchers.abilities
    );

    // 3. `O-Kagachi Made Manifest`: ChooseCard(DefendingPlayer, Graveyard, RememberChosen) -> Hand -> Pump(Self, +Remembered$CardManaCost).
    let okagachi = read(
        "Name:O-Kagachi Made Manifest\nManaCost:no cost\nTypes:Enchantment Creature Dragon Spirit\nPT:6/6\n\
         K:Flying\nK:Trample\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChoose\n\
         SVar:TrigChoose:DB$ ChooseCard | Defined$ TriggeredDefendingPlayer | Choices$ Card.YouOwn+nonLand | Mandatory$ True | ChoiceZone$ Graveyard | RememberChosen$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Graveyard | Destination$ Hand | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Self | NumAtt$ +X | SubAbility$ DBCleanup\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True",
    );
    assert!(
        okagachi.abilities[0].contains("chooser: PlayerRel::DefendingPlayer")
            && okagachi.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::You)")
            && okagachi.abilities[0].contains("movement: ChooseMove::Hand")
            && okagachi.abilities[0].contains("Effect::PumpTargetSlot { slot: TargetSlot::This, power: Amount::RememberedManaValue"),
        "{:?}",
        okagachi.abilities
    );

    // Refusals: Alchemy `Perpetual` Pump on chosen hand card and cross-player simultaneous `DestroyAll`.
    assert!(refused(
        "Name:AlchemyPerp\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Choices$ Creature.YouOwn | ChoiceZone$ Hand | Mandatory$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ChosenCard | NumAtt$ +1 | NumDef$ +1 | Duration$ Perpetual"
    ));
    assert!(refused(
        "Name:CalamityLike\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ ChooseThree | SubAbility$ DestroyAll\n\
         SVar:ChooseThree:DB$ ChooseCard | Defined$ Remembered | Choices$ Permanent.nonLand+RememberedPlayerCtrl | ChoiceZone$ Battlefield | Amount$ 3 | Mandatory$ True | RememberChosen$ True\n\
         SVar:DestroyAll:DB$ DestroyAll | ValidCards$ Permanent.nonLand+!IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True"
    ));
}
