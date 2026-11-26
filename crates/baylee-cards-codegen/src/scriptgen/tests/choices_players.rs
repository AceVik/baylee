use super::{CardBody, TokenLookup, cats, parse, read, refusal_reason, refused, transcode};

fn read_tok(text: &str, tok: (&str, &str)) -> CardBody {
    let parsed = parse(text);
    let lookup = TokenLookup::held(&[tok]);
    transcode(&parsed, &cats(), Some(&lookup)).unwrap_or_else(|| {
        panic!(
            "should read in full, refused: {:?}",
            refusal_reason(&parsed, &cats(), Some(&lookup))
        )
    })
}

#[test]
fn choices_players_untap_during_untap_step_variants() {
    let seedborn = read(
        "Name:Seedborn Muse\nManaCost:3 G G\nTypes:Creature Spirit\nPT:2/4\n\
         S:Mode$ UntapOtherPlayer | ValidCard$ Permanent.YouCtrl",
    );
    assert_eq!(
        seedborn.abilities,
        [
            "static_ability!(Filter::ControlledByYou, Modifier::UntapDuringUntapStep { who: PlayerRel::EachOtherPlayer })"
        ]
    );

    let unwinding = read(
        "Name:Unwinding Clock\nManaCost:4\nTypes:Artifact\n\
         S:Mode$ UntapOtherPlayer | ValidCard$ Artifact.YouCtrl",
    );
    assert!(
        unwinding.abilities[0]
            .contains("Modifier::UntapDuringUntapStep { who: PlayerRel::EachOtherPlayer }"),
        "{:?}",
        unwinding.abilities
    );

    let urban = read(
        "Name:Urban Burgeoning\nManaCost:G\nTypes:Enchantment Aura\n\
         K:Enchant:Land\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | AddStaticAbility$ BurgeoningUntap\n\
         SVar:BurgeoningUntap:Mode$ UntapOtherPlayer | ValidCard$ Card.Self\n\
         SVar:NonStackingAttachEffect:True",
    );
    assert!(
        urban.abilities.iter().any(|a| a.contains(
            "Modifier::GrantStatic { filter: &Filter::This, modifier: &Modifier::UntapDuringUntapStep { who: PlayerRel::EachOtherPlayer } }"
        )),
        "{:?} / {}",
        urban.abilities,
        urban.statics
    );

    let teferi = read(
        "Name:Teferi, Who Slows the Sunset\nManaCost:2 W U\nTypes:Legendary Planeswalker Teferi\nLoyalty:4\n\
         A:AB$ Effect | Cost$ SubCounter<7/LOYALTY> | Planeswalker$ True | Ultimate$ True | \
         Name$ Emblem — Teferi, Who Slows the Sunset | Image$ emblem_teferi | \
         StaticAbilities$ UntapOppStep | Stackable$ False | Duration$ Permanent\n\
         SVar:UntapOppStep:Mode$ UntapOtherPlayer | ValidCard$ Permanent.YouCtrl | ValidPlayer$ Opponent",
    );
    assert!(
        teferi.abilities[0].contains("Effect::CreateEmblem")
            && teferi.abilities[0]
                .contains("Modifier::UntapDuringUntapStep { who: PlayerRel::EachOpponent }"),
        "{:?}",
        teferi.abilities
    );
}

#[test]
fn choices_players_combat_taxes_and_restrictions() {
    let propaganda = read(
        "Name:Propaganda\nManaCost:2 U\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You | Cost$ 2",
    );
    assert_eq!(
        propaganda.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttackManaTax { target: AttackTaxTarget::You, amount: Amount::Fixed(2) })"
        ]
    );

    let oath = read(
        "Name:Onakke Oathkeeper\nManaCost:1 W\nTypes:Creature Ogre Spirit\nPT:0/4\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ Planeswalker.YouCtrl | Cost$ 1",
    );
    assert_eq!(
        oath.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttackManaTax { target: AttackTaxTarget::PlaneswalkersYouControl, amount: Amount::Fixed(1) })"
        ]
    );

    let annex = read(
        "Name:Norn's Annex\nManaCost:3 WP WP\nTypes:Artifact\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You,Planeswalker.YouCtrl | Cost$ WP",
    );
    assert_eq!(
        annex.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttackPhyrexianTax { target: AttackTaxTarget::YouOrYourPlaneswalkers, color: ManaColor::White })"
        ]
    );

    let sphere = read(
        "Name:Sphere of Safety\nManaCost:4 W\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You,Planeswalker.YouCtrl | Cost$ X\n\
         SVar:X:Count$Valid Enchantment.YouCtrl",
    );
    assert!(
        sphere.abilities[0].contains(
            "Modifier::AttackManaTax { target: AttackTaxTarget::YouOrYourPlaneswalkers, amount: Amount::CountOf"
        ),
        "{:?}",
        sphere.abilities
    );

    let restraint = read(
        "Name:Collective Restraint\nManaCost:3 U\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You | Cost$ X\n\
         SVar:X:Count$Domain",
    );
    assert_eq!(
        restraint.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttackManaTax { target: AttackTaxTarget::You, amount: Amount::Domain(PlayerRel::You) })"
        ]
    );

    let nils = read(
        "Name:Nils, Discipline Enforcer\nManaCost:2 W\nTypes:Legendary Creature Human Cleric\nPT:2/2\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature.HasCounters | Target$ You,Planeswalker.YouCtrl | Cost$ X | RememberingAttacker$ True\n\
         SVar:X:Remembered$CardCounters.ALL",
    );
    assert!(
        nils.abilities[0].contains(
            "Modifier::AttackManaTax { target: AttackTaxTarget::YouOrYourPlaneswalkers, amount: Amount::TotalCountersOnSubject }"
        ),
        "{:?}",
        nils.abilities
    );

    let floodtide = read(
        "Name:Floodtide Serpent\nManaCost:4 U\nTypes:Creature Serpent\nPT:4/4\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature.Self | Cost$ Return<1/Enchantment>",
    );
    assert_eq!(
        floodtide.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::This]), Modifier::CantAttackUnlessYouReturn { count: 1, filter: &Filter::ENCHANTMENT })"
        ]
    );
}

#[test]
fn choices_players_attack_block_restrictions_and_dynamic_taxes() {
    let hollow = read(
        "Name:Hollow Warrior\nManaCost:4\nTypes:Artifact Creature Golem Warrior\nPT:4/4\n\
         S:Mode$ CantAttackUnless | ValidCard$ Card.Self | Cost$ tapXType<1/Creature.!attacking>\n\
         S:Mode$ CantBlockUnless | ValidCard$ Card.Self | Cost$ tapXType<1/Creature.!blocking>",
    );
    assert_eq!(
        hollow.abilities,
        [
            "static_ability!(Filter::This, Modifier::CantAttackUnlessYouTap { count: 1, filter: &Filter::CREATURE })",
            "static_ability!(Filter::This, Modifier::CantBlockUnlessYouTap { count: 1, filter: &Filter::CREATURE })",
        ]
    );

    let cowed = read(
        "Name:Cowed by Wisdom\nManaCost:W\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature.AttachedBy | Cost$ Y\n\
         S:Mode$ CantBlockUnless | ValidCard$ Creature.AttachedBy | Cost$ Y\n\
         SVar:Y:Count$ValidHand Card.YouOwn",
    );
    assert!(
        cowed.abilities[1]
            .contains("Modifier::AttackManaTax { target: AttackTaxTarget::AnyDefender")
            && cowed.abilities[2].contains("Modifier::BlockManaTax {"),
        "{:?}",
        cowed.abilities
    );

    let whipgrass = read(
        "Name:Whipgrass Entangler\nManaCost:2 W\nTypes:Creature Human Cleric\nPT:1/3\n\
         A:AB$ Animate | Cost$ 1 W | ValidTgts$ Creature | staticAbilities$ WhipgrassCantAttack,WhipgrassCantBlock\n\
         SVar:WhipgrassCantAttack:Mode$ CantAttackUnless | ValidCard$ Card.Self | Cost$ WhipgrassClericNum\n\
         SVar:WhipgrassCantBlock:Mode$ CantBlockUnless | ValidCard$ Card.Self | Cost$ WhipgrassClericNum\n\
         SVar:WhipgrassClericNum:Count$Valid Goblin",
    );
    assert!(
        whipgrass.abilities[0].contains("Modifier::GrantStatic")
            && whipgrass.abilities[0].contains("AttackManaTax")
            && whipgrass.abilities[0].contains("BlockManaTax"),
        "{:?}",
        whipgrass.abilities
    );
}

#[test]
fn choices_players_enter_modifiers_and_reveal_cost() {
    let archon = read(
        "Name:Guardian Archon\nManaCost:4 W W\nTypes:Creature Archon\nPT:5/5\n\
         K:ETBReplacement:Other:ChooseP\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Opponent | Secretly$ True\n\
         A:AB$ Pump | Cost$ RevealChosen<Player> | ValidTgts$ Permanent.YouCtrl | \
         KW$ Protection:Player.PlayerUID_ChosenPlayerUID:ChosenPlayerName | DefinedKW$ ChosenPlayer | \
         GameActivationLimit$ 1 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ You | KW$ Protection:Player.PlayerUID_ChosenPlayerUID:ChosenPlayerName | DefinedKW$ ChosenPlayer",
    );
    assert_eq!(
        archon.enter_modifiers,
        ["EnterModifier::SecretlyChooseOpponent"]
    );
    assert!(
        archon.abilities[0].contains("RevealChosenPlayer")
            && archon.abilities[0].contains("Modifier::ProtectionFrom")
            && archon.abilities[0].contains("Modifier::PlayerProtectionFrom"),
        "{:?}",
        archon.abilities
    );

    let lockbox = read(
        "Name:Ominous Lockbox\nManaCost:1 U\nTypes:Artifact Clue\n\
         K:ETBReplacement:Other:ChooseNum\n\
         SVar:ChooseNum:DB$ ChooseNumber | Secretly$ True | Min$ 1 | Max$ 20 | KeepSecret$ True\n\
         T:Mode$ SpellCast | ValidCard$ Card.cmcEQCount$ChosenNumber | TriggerZones$ Battlefield | ValidActivatingPlayer$ Opponent | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert_eq!(
        lockbox.enter_modifiers,
        ["EnterModifier::SecretlyChooseNumber { min: 1, max: 20 }"]
    );
    assert!(
        lockbox.statics.contains("Filter::CmcEqualsChosenNumber"),
        "{}",
        lockbox.statics
    );

    let oppressor = read(
        "Name:Callous Oppressor\nManaCost:1 U U\nTypes:Creature Octopus\nPT:1/2\n\
         K:ETBReplacement:Other:DBChooseOpp\n\
         SVar:DBChooseOpp:DB$ ChoosePlayer | Choices$ Player.Opponent | SubAbility$ DBChooseType\n\
         SVar:DBChooseType:DB$ ChooseType | Defined$ Player.Chosen | Type$ Creature\n\
         A:AB$ GainControl | Cost$ T | ValidTgts$ Creature.IsNotChosenType | LoseControl$ Untap,LeavesPlay",
    );
    assert_eq!(
        oppressor.enter_modifiers,
        ["EnterModifier::OpponentChoosesCreatureType"]
    );
    assert!(
        oppressor
            .statics
            .contains("Filter::Not(&Filter::MatchesChosenTypeOfSource)"),
        "{}",
        oppressor.statics
    );

    let favored = read(
        "Name:Favored Enemy\nManaCost:G\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseOpp\n\
         SVar:ChooseOpp:DB$ ChoosePlayer | Choices$ Opponent | SubAbility$ DBChooseType\n\
         SVar:DBChooseType:DB$ ChooseType | Type$ Creature | MostPrevalentInDefinedZone$ ChosenPlayer_Library | Note$ True | SubAbility$ DBCleanupChosen\n\
         SVar:DBCleanupChosen:DB$ Cleanup | ClearChosenPlayer$ True\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.OppCtrl+NotedType | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert_eq!(
        favored.enter_modifiers,
        ["EnterModifier::NoteMostPrevalentCreatureTypeInOpponentLibrary"]
    );
    assert!(
        favored
            .statics
            .contains("Filter::MatchesChosenTypeOfSource"),
        "{}",
        favored.statics
    );

    let two_players = read(
        "Name:TwoPlayerEtbProbe\nManaCost:4 R\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseP\n\
         SVar:ChooseP:DB$ ChoosePlayer | Defined$ You | Choices$ Player | RememberChosen$ True | SubAbility$ ChoosePTwo\n\
         SVar:ChoosePTwo:DB$ ChoosePlayer | Defined$ You | Choices$ NonChosenPlayer",
    );
    assert_eq!(
        two_players.enter_modifiers,
        ["EnterModifier::ChooseTwoPlayers"]
    );

    let rand_etb = read(
        "Name:HaktosEtbProbe\nManaCost:R R W W\nTypes:Creature\nPT:6/1\n\
         K:ETBReplacement:Other:ChooseNum\n\
         SVar:ChooseNum:DB$ ChooseNumber | Min$ 2 | Max$ 4 | Defined$ You | Random$ True",
    );
    assert_eq!(
        rand_etb.enter_modifiers,
        ["EnterModifier::ChooseNumberAtRandom { min: 2, max: 4 }"]
    );
}

#[test]
fn choices_players_effects_secret_join_forces_and_conditions() {
    let searchlight = read(
        "Name:Spectral Searchlight\nManaCost:3\nTypes:Artifact\n\
         A:AB$ ChoosePlayer | Cost$ T | Defined$ You | Choices$ Player | SubAbility$ DBMana\n\
         SVar:DBMana:DB$ Mana | Produced$ Any | Defined$ ChosenPlayer",
    );
    assert!(
        searchlight.abilities[0].contains("Effect::ChoosePlayerEx")
            && searchlight.abilities[0].contains("Effect::AddManaChoiceFor"),
        "{:?}",
        searchlight.abilities
    );

    let hellkite = read(
        "Name:Territorial Hellkite\nManaCost:2 R R\nTypes:Creature Dragon\nPT:6/5\n\
         K:Flying\nK:Haste\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigChoose | TriggerZones$ Battlefield\n\
         SVar:TrigChoose:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent+!IsRemembered | Random$ True | ChooseSubAbility$ DBPump | CantChooseSubAbility$ DBTap\n\
         SVar:DBPump:DB$ Effect | Defined$ Self | StaticAbilities$ AttackChosen | Duration$ UntilEndOfCombat\n\
         SVar:AttackChosen:Mode$ MustAttack | ValidCreature$ Card.EffectSource | MustAttack$ ChosenPlayer\n\
         SVar:DBTap:DB$ Tap | Defined$ Self | SubAbility$ DBCleanup\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigClearRem | Static$ True\n\
         SVar:TrigClearRem:DB$ Cleanup | ClearRemembered$ True | SubAbility$ TrigRemember\n\
         SVar:TrigRemember:DB$ Bookkeeping | RememberObjects$ TriggeredDefendingPlayer\n\
         T:Mode$ Phase | Phase$ EndCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ DBCleanup | IsPresent$ Card.!attackedThisTurn+Self | Static$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenPlayer$ True | ClearRemembered$ True",
    );
    assert_eq!(hellkite.abilities.len(), 1);
    assert!(
        hellkite.abilities[0].contains("PlayerFilter::OpponentNotAttackedBySourceLastCombat")
            && hellkite.abilities[0].contains("random: true")
            && hellkite.abilities[0].contains("Effect::TapSelf"),
        "{:?}",
        hellkite.abilities
    );

    let wheel = read(
        "Name:Wheel of Misfortune\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ ChooseNumber | Defined$ Player | Secretly$ True | Highest$ DBLoseLife | NotLowest$ DBDiscard\n\
         SVar:DBLoseLife:DB$ DealDamage | Defined$ Remembered | NumDmg$ Count$ChosenNumber\n\
         SVar:DBDiscard:DB$ Discard | Defined$ Remembered | Mode$ Hand | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Remembered | NumCards$ 7",
    );
    assert!(
        wheel.abilities[0].contains(
            "Effect::SecretChooseNumbers(SecretNumberChoice { players: PlayerRel::EachPlayer"
        ),
        "{:?}",
        wheel.abilities
    );

    let ogre = read(
        "Name:Menacing Ogre\nManaCost:3 R R\nTypes:Creature Ogre\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChooseNumber\n\
         SVar:TrigChooseNumber:DB$ ChooseNumber | Defined$ Player | Secretly$ True | Highest$ DBLoseLife | RememberHighest$ True | SubAbility$ DBPutCounter\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Remembered | LifeAmount$ X\n\
         SVar:X:Count$ChosenNumber\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2 | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        ogre.abilities[0].contains("remember_highest: true")
            && ogre.abilities[0].contains("Condition::YouAreRememberedPlayer"),
        "{:?}",
        ogre.abilities
    );

    let trap = read(
        "Name:The Toymaker's Trap\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigChooseNumber\n\
         SVar:TrigChooseNumber:DB$ ChooseNumber | Secretly$ True | Min$ 1 | Max$ 5 | RemoveChoices$ Remembered | Guesser$ Opponent | GuessWrong$ DBLoseLife | GuessCorrect$ DBSac | RememberChosen$ True\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ RememberedPlayer | LifeAmount$ Count$ChosenNumber | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw\n\
         SVar:DBSac:DB$ Sacrifice\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | Static$ True | ValidCard$ Card.Self | Execute$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        trap.abilities[0]
            .contains("Effect::SecretNumberGuess { min: 1, max: 5, track_used_on_source: true"),
        "{:?}",
        trap.abilities
    );

    let safe_probe = read(
        "Name:SecretOpponentMatchProbe\nManaCost:2\nTypes:Artifact\n\
         A:AB$ TargetOnly | Cost$ 1 T | ValidTgts$ Opponent | SubAbility$ ChooseNumber\n\
         SVar:ChooseNumber:DB$ ChooseNumber | Defined$ TargetedAndYou | Min$ 1 | Max$ 3 | MatchedAbility$ DBDraw | UnmatchedAbility$ DBLose | Secretly$ True\n\
         SVar:DBDraw:DB$ Draw | Defined$ You\n\
         SVar:DBLose:DB$ LoseLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        safe_probe.abilities[0].contains("players: PlayerRel::YouAndTargetOpponent"),
        "{:?}",
        safe_probe.abilities
    );
}

#[test]
fn choices_players_join_forces_plague_and_conditions() {
    let minds_aglow = read(
        "Name:Minds Aglow\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | StartingWith$ You | RepeatSubAbility$ DBPay | SubAbility$ DBDraw\n\
         SVar:DBPay:DB$ ChooseNumber | Defined$ Player.IsRemembered | ChooseAnyNumber$ True | SubAbility$ DBStore\n\
         SVar:DBStore:DB$ StoreSVar | SVar$ JoinForcesAmount | Type$ CountSVar | Expression$ JoinForcesAmount/Plus.Y | UnlessCost$ Y | UnlessPayer$ Player.IsRemembered | UnlessSwitched$ True\n\
         SVar:DBDraw:DB$ Draw | Defined$ Player | NumCards$ JoinForcesAmount | SubAbility$ DBReset\n\
         SVar:DBReset:DB$ StoreSVar | SVar$ JoinForcesAmount | Type$ Number | Expression$ 0\n\
         SVar:Y:Count$ChosenNumber\n\
         SVar:JoinForcesAmount:Number$0",
    );
    assert!(
        minds_aglow.abilities[0].contains("RepeatDomain::EachPlayerStartingWithYou")
            && minds_aglow.abilities[0].contains("Effect::AccumulateChosenNumber")
            && minds_aglow.abilities[0].contains("Amount::ResolutionAccumulatedNumber"),
        "{:?}",
        minds_aglow.abilities
    );

    let plague = read_tok(
        "Name:Plague of Vermin\nManaCost:6 B\nTypes:Sorcery\n\
         A:SP$ Repeat | RepeatSubAbility$ DBResetCheck | RepeatCheckSVar$ NumPlayerGiveup | RepeatSVarCompare$ LTTotalPlayer | SubAbility$ DBRepeatToken\n\
         SVar:DBResetCheck:DB$ StoreSVar | SVar$ NumPlayerGiveup | Type$ Number | Expression$ 0 | SubAbility$ DBRepeatChoice\n\
         SVar:DBRepeatChoice:DB$ RepeatEach | StartingWith$ You | RepeatSubAbility$ DBChoice | RepeatPlayers$ Player\n\
         SVar:DBChoice:DB$ ChooseNumber | Defined$ Remembered | Max$ LifeTotal | ListTitle$ amount of life to pay | SubAbility$ DBCheckPaid\n\
         SVar:DBCheckPaid:DB$ StoreSVar | SVar$ NumPlayerGiveup | Type$ CountSVar | Expression$ NumPlayerGiveup/Plus.1 | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ DBStore\n\
         SVar:DBStore:DB$ StoreSVar | SVar$ EachPlayer | Type$ AdditiveForEach | Expression$ X | ConditionCheckSVar$ X | UnlessCost$ PayLife<X> | UnlessPayer$ Remembered | UnlessSwitched$ True | UnlessResolveSubs$ WhenNotPaid | SubAbility$ DBGiveUp\n\
         SVar:DBGiveUp:DB$ StoreSVar | SVar$ NumPlayerGiveup | Type$ CountSVar | Expression$ NumPlayerGiveup/Plus.1 | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1\n\
         SVar:DBRepeatToken:DB$ RepeatEach | ChangeZoneTable$ True | RepeatSubAbility$ DBToken | RepeatPlayers$ Player | SubAbility$ DBFinalReset\n\
         SVar:DBToken:DB$ Token | TokenAmount$ LifePaidAmount | TokenScript$ b_1_1_rat | TokenOwner$ Remembered\n\
         SVar:DBFinalReset:DB$ StoreSVar | SVar$ NumPlayerGiveup | Type$ Number | Expression$ 0 | SubAbility$ DBResetEach\n\
         SVar:DBResetEach:DB$ RepeatEach | RepeatSubAbility$ DBCleanup | RepeatPlayers$ Player\n\
         SVar:DBCleanup:DB$ StoreSVar | SVar$ EachPlayer | Type$ Number | Expression$ 0\n\
         SVar:NumPlayerGiveup:Number$0\n\
         SVar:TotalPlayer:PlayerCountPlayers$Amount\n\
         SVar:LifeTotal:PlayerCountRemembered$LifeTotal\n\
         SVar:LifePaidAmount:PlayerCountRemembered$SVarAmount\n\
         SVar:X:Count$ChosenNumber",
        (
            "b_1_1_rat",
            "Name:Rat\nTypes:Creature Goblin\nColors:black\nPT:1/1",
        ),
    );
    assert!(
        plague.abilities[0].contains("Effect::RepeatEachPlayerPayUntilNone { kind: VariablePayKind::Life, max_per_player: Amount::LifeTotal(PlayerRel::EventPlayer)"),
        "{:?}",
        plague.abilities
    );

    let bring_ending = read(
        "Name:Bring the Ending\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X | TargetType$ Spell | TgtZone$ Stack | ValidTgts$ Card | BranchConditionSVarCompare$ GE3 | TrueSubAbility$ Counter | FalseSubAbility$ CounterUnless\n\
         SVar:CounterUnless:DB$ Counter | Defined$ Targeted | UnlessCost$ 2\n\
         SVar:Counter:DB$ Counter | Defined$ Targeted\n\
         SVar:X:TargetedController$Counters.Poison",
    );
    assert!(
        bring_ending.abilities[0].contains("Condition::AmountAtLeast(Amount::PlayerCounters { player: PlayerRel::TargetController, kind: CounterKind::Poison }, 3)"),
        "{:?}",
        bring_ending.abilities
    );

    let radgull = read(
        "Name:Vexing Radgull\nManaCost:1 U\nTypes:Creature Bird Mutant\nPT:1/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ TrigBranch | CombatDamage$ True\n\
         SVar:TrigBranch:DB$ Branch | Defined$ TriggeredTarget | BranchConditionSVar$ X | BranchConditionSVarCompare$ EQ0 | TrueSubAbility$ DBRadiation | FalseSubAbility$ DBProliferate\n\
         SVar:DBRadiation:DB$ Radiation | Defined$ TriggeredTarget | Num$ 2\n\
         SVar:DBProliferate:DB$ Proliferate\n\
         SVar:X:TriggeredTarget$Counters.RAD",
    );
    assert!(
        radgull.abilities[0].contains(
            "Amount::PlayerCounters { player: PlayerRel::EventPlayer, kind: CounterKind::Rad }"
        ),
        "{:?}",
        radgull.abilities
    );

    let haunting = read(
        "Name:Haunting Voyage\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ ReturnAll | FalseSubAbility$ ReturnUpToTwo\n\
         SVar:ReturnUpToTwo:DB$ ChangeZone | ChangeType$ Creature.ChosenType+YouOwn | Origin$ Graveyard | Destination$ Battlefield | ChangeNum$ 2 | Hidden$ True\n\
         SVar:ReturnAll:DB$ ChangeZoneAll | ChangeType$ Creature.ChosenType+YouOwn | Origin$ Graveyard | Destination$ Battlefield\n\
         SVar:X:Count$Foretold.1.0\n\
         K:Foretell:5 B B\n\
         Oracle:Choose a creature type. Return up to two creature cards of that type from your graveyard to the battlefield. If this spell was foretold, return all creature cards of that type from your graveyard to the battlefield instead.",
    );
    assert!(
        haunting.abilities[0].contains("Condition::WasForetold"),
        "{:?}",
        haunting.abilities
    );

    let murgish = read_tok(
        "Name:Murgish Cemetery\nManaCost:4 B B\nTypes:Enchantment\n\
         A:AB$ ChooseNumber | Cost$ 3 B Discard<1/Card> | Min$ 2 | Max$ 6 | Defined$ You | Random$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ b_x_x_zombie | TokenPower$ X | TokenToughness$ X\n\
         SVar:X:Count$Random.2.6",
        (
            "b_x_x_zombie",
            "Name:Zombie\nTypes:Creature Goblin\nColors:black\nPT:*/*",
        ),
    );
    assert!(
        murgish.abilities[0].contains("Effect::ChooseNumberAtRandom { min: Amount::Fixed(2), max: Amount::Fixed(6), then: &[Effect::CreateTokenSized"),
        "{:?}",
        murgish.abilities
    );
}

#[test]
fn choices_players_honest_refusal_guards() {
    for bad in [
        // SetChosenNumber$ on AttackTax/BlockTax must stay refused (Dynamic ChosenNumber not stored on ContinuousEffect).
        "Name:War Tax\nManaCost:2 U\nTypes:Enchantment\n\
         A:AB$ Effect | Cost$ X U | StaticAbilities$ AttackTax | SetChosenNumber$ X\n\
         SVar:AttackTax:Mode$ CantAttackUnless | ValidCard$ Creature | Cost$ XChosen\n\
         SVar:X:Count$xPaid\n\
         SVar:XChosen:Count$ChosenNumber",
        "Name:War Cadence\nManaCost:2 R\nTypes:Enchantment\n\
         A:AB$ Effect | Cost$ X R | StaticAbilities$ CadenceStaticAb | Stackable$ False | SetChosenNumber$ X\n\
         SVar:CadenceStaticAb:Mode$ CantBlockUnless | Cost$ XChosen\n\
         SVar:X:Count$xPaid\n\
         SVar:XChosen:Count$ChosenNumber",
        // Plane / Ongoing Scheme EffectZone$ Command UntapOtherPlayer stays refused.
        "Name:Horizon Boughs\nManaCost:no cost\nTypes:Plane Pyrulea\n\
         S:Mode$ UntapOtherPlayer | ValidCard$ Permanent | EffectZone$ Command",
        // RememberingAttacker$ True with wrong SVar must stay refused.
        "Name:BadNils\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature.HasCounters | Target$ You,Planeswalker.YouCtrl | Cost$ X | RememberingAttacker$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
        // Opponent's LoseGame is not in DSL (`Effect::LoseGame` is always `You`).
        "Name:BadOppLoseGame\nManaCost:W\nTypes:Creature\nPT:1/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseGame | Defined$ TriggeredTarget",
        // Ticket counters (Un-set) stay refused.
        "Name:BadTicketCounter\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X | TargetType$ Spell | TgtZone$ Stack | ValidTgts$ Card | BranchConditionSVarCompare$ GE3 | TrueSubAbility$ C\n\
         SVar:C:DB$ Counter | Defined$ Targeted\n\
         SVar:X:TargetedController$Counters.TICKET",
    ] {
        let parsed = parse(bad);
        assert!(refused(bad), "must refuse:\n{bad}");
        assert!(
            refusal_reason(&parsed, &super::cats(), None).is_some(),
            "missing refusal reason for:\n{bad}"
        );
    }
}
