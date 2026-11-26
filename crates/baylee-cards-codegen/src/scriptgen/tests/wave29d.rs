#![allow(clippy::too_many_lines)]

use super::*;

#[test]
fn wave29d_moved_cant_enter_from_zones() {
    let cage = read(
        "Name:Grafdigger's Cage\nManaCost:1\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Graveyard,Library | Destination$ Battlefield | ValidLKI$ Creature.Other | Prevent$ True | Layer$ CantHappen\n\
         S:Mode$ CantBeCast | Origin$ Graveyard,Library",
    );
    assert_eq!(cage.abilities.len(), 2, "{:?}", cage.abilities);
    assert!(
        cage.abilities[0].contains("Modifier::CantEnterBattlefield(&CANT_ENTER")
            && cage.statics.contains("Filter::InZone(ZoneRef::Graveyard)")
            && cage.statics.contains("Filter::InZone(ZoneRef::Library)"),
        "{cage:?}"
    );

    let jailer = read(
        "Name:Soulless Jailer\nManaCost:2\nTypes:Artifact Creature Goblin\nPT:0/4\n\
         R:Event$ Moved | Layer$ CantHappen | ActiveZones$ Battlefield | Origin$ Graveyard | Destination$ Battlefield | ValidLKI$ Permanent | Prevent$ True\n\
         S:Mode$ CantBeCast | ValidCard$ Card.nonCreature | Origin$ Graveyard,Exile",
    );
    assert_eq!(jailer.abilities.len(), 2, "{:?}", jailer.abilities);

    let runestone = read(
        "Name:Weathered Runestone\nManaCost:2\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Graveyard,Library | Destination$ Battlefield | ValidLKI$ Permanent.nonLand | Prevent$ True | Layer$ CantHappen\n\
         S:Mode$ CantBeCast | Origin$ Graveyard,Library",
    );
    assert_eq!(runestone.abilities.len(), 2, "{:?}", runestone.abilities);

    // Non-LKI `ValidCard$` and unsupported origin zones remain refused.
    assert!(refused(
        "Name:NonLkiCage\nManaCost:1\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Graveyard | Destination$ Battlefield | ValidCard$ Creature | Prevent$ True | Layer$ CantHappen"
    ));
    assert!(refused(
        "Name:StackCage\nManaCost:1\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Stack | Destination$ Battlefield | ValidLKI$ Creature | Prevent$ True | Layer$ CantHappen"
    ));
}

#[test]
fn wave29d_reveal_random_and_name_card_chains() {
    let favor = read(
        "Name:Planeswalker's Favor\nManaCost:2 G\nTypes:Enchantment\n\
         A:AB$ Reveal | Cost$ 3 G | Random$ True | RememberRevealed$ True | ValidTgts$ Opponent | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        favor.abilities[0].contains(
            "Effect::RevealRandomFromHand { who: PlayerRel::FirstTargetPlayerOrController, count: Amount::Fixed(1) }"
        ) && favor.abilities[0].contains("Effect::WithSecondTarget(&[")
            && favor.abilities[0].contains("Amount::ThisWay(ThisWay::RevealedManaValue)"),
        "{}",
        favor.abilities[0]
    );

    let scorn = read(
        "Name:Planeswalker's Scorn\nManaCost:2 B\nTypes:Enchantment\n\
         A:AB$ Reveal | Cost$ 3 B | Random$ True | RememberRevealed$ True | ValidTgts$ Opponent | SorcerySpeed$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        scorn.abilities[0]
            .contains("Amount::Negated(&Amount::ThisWay(ThisWay::RevealedManaValue))"),
        "{}",
        scorn.abilities[0]
    );

    let fury = read(
        "Name:Planeswalker's Fury\nManaCost:2 R\nTypes:Enchantment\n\
         A:AB$ Reveal | Cost$ 3 R | Random$ True | RememberRevealed$ True | ValidTgts$ Opponent | SorcerySpeed$ True | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ Targeted | NumDmg$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        fury.abilities[0].contains(
            "Effect::DealDamage { amount: Amount::ThisWay(ThisWay::RevealedManaValue), target: TargetSpec::Player(PlayerRel::Chosen) }"
        ),
        "{}",
        fury.abilities[0]
    );

    let mirth = read(
        "Name:Planeswalker's Mirth\nManaCost:2 W\nTypes:Enchantment\n\
         A:AB$ Reveal | Cost$ 3 W | Random$ True | RememberRevealed$ True | ValidTgts$ Opponent | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        mirth.abilities[0]
            .contains("Effect::GainLife { amount: Amount::ThisWay(ThisWay::RevealedManaValue) }"),
        "{}",
        mirth.abilities[0]
    );

    let friendly_fire = read(
        "Name:Friendly Fire\nManaCost:3 R\nTypes:Instant\n\
         A:SP$ Reveal | ValidTgts$ Creature | Defined$ TargetedController | Random$ True | RememberRevealed$ True | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DamageAll | ValidPlayers$ TargetedController | ValidCards$ Targeted.Self | NumDmg$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        friendly_fire.abilities[0]
            .contains("target: TargetSpec::Player(PlayerRel::ControllerOfTarget)")
            && friendly_fire.abilities[0].contains("Amount::ThisWay(ThisWay::RevealedManaValue)"),
        "{}",
        friendly_fire.abilities[0]
    );

    let torturer = read(
        "Name:Hired Torturer\nManaCost:2 B\nTypes:Creature Wizard\nPT:2/3\n\
         K:Defender\n\
         A:AB$ LoseLife | Cost$ 3 B T | ValidTgts$ Opponent | LifeAmount$ 2 | SubAbility$ DBReveal\n\
         SVar:DBReveal:DB$ Reveal | Defined$ Targeted | Random$ True | NumCards$ 1",
    );
    assert!(
        torturer.abilities[0].contains(
            "Effect::RevealRandomFromHand { who: PlayerRel::Chosen, count: Amount::Fixed(1) }"
        ),
        "{}",
        torturer.abilities[0]
    );

    let dementia = read(
        "Name:Mindprobe Mage\nManaCost:3 U B\nTypes:Creature Wizard\nPT:3/3\n\
         A:AB$ NameCard | Cost$ T | Defined$ You | SubAbility$ DBReveal\n\
         SVar:DBReveal:DB$ Reveal | ValidTgts$ Opponent | Random$ True | RememberRevealed$ True | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ Targeted | Mode$ TgtChoose | DiscardValid$ Card.NamedCard+IsRemembered | NumCards$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearNamedCard$ True",
    );
    assert!(
        dementia.abilities[0]
            .contains("Effect::RevealRandomDiscardNamed { player: PlayerRel::Chosen, count: Amount::Fixed(1) }"),
        "{}",
        dementia.abilities[0]
    );

    // Vacuous self random reveal without `RememberRevealed$` stays refused.
    assert!(refused(
        "Name:SelfRandReveal\nManaCost:U\nTypes:Instant\n\
         A:SP$ Reveal | Defined$ You | Random$ True | NumCards$ 1"
    ));
}

#[test]
fn wave29d_triggers_and_stubs() {
    let confessor = read(
        "Name:Confessor\nManaCost:W\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ Discarded | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        confessor.abilities[0]
            .contains("Trigger::Discarded { who: PlayerRel::EachPlayer, filter: &Filter::Any }"),
        "{}",
        confessor.abilities[0]
    );

    let auramancers = read(
        "Name:Lost Auramancers\nManaCost:2 W W\nTypes:Creature Wizard\nPT:3/3\n\
         K:Vanishing:3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+counters_EQ0_TIME | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Enchantment | ChangeNum$ 1 | Optional$ True",
    );
    assert!(
        auramancers
            .abilities
            .iter()
            .any(|a| a.contains("Trigger::Dies(&") && a.contains("Effect::SearchLibrary")),
        "{:?}",
        auramancers.abilities
    );

    let death_match = read(
        "Name:Death Match\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ TrigPump | OptionalDecider$ TriggeredCardController\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ -3 | NumDef$ -3 | IsCurse$ True | TargetingPlayer$ TriggeredCardController",
    );
    assert!(
        death_match.abilities[0]
            .contains("Effect::MayDoFor { player: PlayerRel::ControllerOfEvent")
            && death_match.abilities[0].contains(".chosen_by_event_player()"),
        "{}",
        death_match.abilities[0]
    );

    let pandemonium = read(
        "Name:Pandemonium\nManaCost:3 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ TrigDamage | OptionalDecider$ TriggeredCardController\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | DamageSource$ TriggeredCard | TargetingPlayer$ TriggeredCardController\n\
         SVar:X:TriggeredCard$CardPower",
    );
    assert!(
        pandemonium.abilities[0].contains(
            "Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::AnyTarget }"
        ) && pandemonium.abilities[0].contains(".chosen_by_event_player()"),
        "{}",
        pandemonium.abilities[0]
    );

    let lieges = read(
        "Name:Oath of Lieges\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ LiegesSearch | TriggerZones$ Battlefield\n\
         SVar:LiegesSearch:DB$ ChangeZone | TargetingPlayer$ TriggeredPlayer | ValidTgts$ Player.OpponentOf TriggeredPlayer+withMoreLandsThanTriggeredPlayer | DefinedPlayer$ TriggeredPlayer | Chooser$ TriggeredPlayer | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | Optional$ True | ShuffleNonMandatory$ True",
    );
    assert!(
        lieges.abilities[0].contains(
            "TargetSpec::OpponentWithMoreThanActivePlayer(&Filter::LAND)).chosen_by_event_player()"
        ) && lieges.abilities[0].contains("Find::BATTLEFIELD"),
        "{}",
        lieges.abilities[0]
    );

    let terrain = read(
        "Name:Illusionary Terrain\nManaCost:U U\nTypes:Enchantment\n\
         K:Cumulative upkeep:2\n\
         K:ETBReplacement:Other:ChooseType1\n\
         SVar:ChooseType1:DB$ ChooseType | Defined$ You | Type$ Basic Land | SubAbility$ ChooseType2\n\
         SVar:ChooseType2:DB$ ChooseType | Defined$ You | Type$ Basic Land | ChooseType2$ True\n\
         S:Mode$ Continuous | Affected$ Land.Basic+ChosenType | AddType$ ChosenType2 | RemoveLandTypes$ True",
    );
    assert!(
        terrain
            .enter_modifiers
            .contains(&"EnterModifier::ChooseTwoBasicLandTypes".to_string())
            && terrain.abilities.iter().any(|a| {
                a.contains("Filter::MatchesChosenTypeOfSource")
                    && a.contains("Modifier::SetLandTypeToSecondChosen")
            }),
        "{terrain:?}"
    );

    // `ChooseType.SubAbility` without `ChooseType2$ True` stays refused.
    assert!(refused(
        "Name:BadTerrain\nManaCost:U U\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseType1\n\
         SVar:ChooseType1:DB$ ChooseType | Defined$ You | Type$ Basic Land | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw"
    ));

    let hydromorph = read(
        "Name:Hydromorph Guardian\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/2\n\
         A:AB$ Counter | Cost$ U Sac<1/CARDNAME> | TargetType$ Spell | ValidTgts$ Card | TargetValidTargeting$ Creature.YouCtrl+inRealZoneBattlefield",
    );
    assert!(
        hydromorph
            .statics
            .contains("Filter::TargetsYouControl(&Filter::CREATURE)")
            && hydromorph.abilities[0].contains("Effect::CounterTarget"),
        "{hydromorph:?}"
    );

    let elfhame = read(
        "Name:Elfhame Sanctuary\nManaCost:1 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | SubAbility$ DBSkipDraw | ShuffleNonMandatory$ True\n\
         SVar:DBSkipDraw:DB$ SkipPhase | Defined$ You | Step$ Draw | Duration$ EndOfTurn",
    );
    assert!(
        elfhame.abilities[0].contains("Modifier::SkipDrawStep { who: PlayerRel::You }")
            && elfhame.abilities[0].contains("Duration::UntilEndOfTurn"),
        "{}",
        elfhame.abilities[0]
    );

    // Non-`YouCtrl` `TargetValidTargeting$` and across-turn `SkipPhase` stay refused.
    assert!(refused(
        "Name:ConfoundProbe\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | TargetValidTargeting$ Creature.inRealZoneBattlefield"
    ));
    assert!(refused(
        "Name:FatigueProbe\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ SkipPhase | ValidTgts$ Player | Step$ Draw"
    ));
}

#[test]
fn wave29d_prevention_stagger_and_statics() {
    let haze_frog = read(
        "Name:Haze Frog\nManaCost:3 G G\nTypes:Creature Goblin\nPT:2/1\n\
         K:Flash\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPrevent\n\
         SVar:TrigPrevent:DB$ Effect | ReplacementEffects$ RPrevent | RememberObjects$ Self\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidSource$ Creature.!IsRemembered",
    );
    assert!(
        haze_frog
            .abilities
            .iter()
            .any(|a| a.contains("Filter::ANOTHER_CREATURE")),
        "{:?}",
        haze_frog.abilities
    );

    let ajani_aid = read(
        "Name:Ajani's Aid\nManaCost:2 G W\nTypes:Enchantment\n\
         A:AB$ ChooseCard | Cost$ Sac<1/CARDNAME> | Choices$ Creature | Mandatory$ True | SubAbility$ DBPrevent\n\
         SVar:DBPrevent:DB$ Effect | ReplacementEffects$ PreventDmg | RememberObjects$ ChosenCard | SubAbility$ DBCleanup\n\
         SVar:PreventDmg:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Creature.IsRemembered | IsCombat$ True | Prevent$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        ajani_aid.abilities[0].contains(
            "Effect::PreventFromChosenSource { target: None, sources: &Filter::CREATURE, next_only: false, combat_only: true, all_but: 0, gain_life: false }"
        ),
        "{}",
        ajani_aid.abilities[0]
    );

    let neko_te = read(
        "Name:Neko-Te\nManaCost:3\nTypes:Artifact Equipment\n\
         K:Equip:2\n\
         T:Mode$ DamageDone | ValidSource$ Card.EquippedBy | ValidTarget$ Creature | TriggerZones$ Battlefield | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | Defined$ TriggeredTarget | SubAbility$ DBFreeze\n\
         SVar:DBFreeze:DB$ Effect | Duration$ LeavesPlay | RememberObjects$ TriggeredTarget | ReplacementEffects$ DontUntap\n\
         SVar:DontUntap:Event$ Untap | Layer$ CantHappen | ValidCard$ Creature.IsRemembered | ValidStepTurnToController$ You",
    );
    assert!(
        neko_te
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::DoesNotUntap")
                && a.contains("Duration::WhileSourceOnBattlefield")),
        "{:?}",
        neko_te.abilities
    );

    let lightning = read(
        "Name:Lightning, Army of One\nManaCost:1 R W\nTypes:Legendary Creature Wizard\nPT:3/2\n\
         K:First Strike\nK:Trample\nK:Lifelink\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ TrigEffect | CombatDamage$ True\n\
         SVar:TrigEffect:DB$ Effect | ReplacementEffects$ Stagger | RememberObjects$ TriggeredTarget | Duration$ UntilYourNextTurn\n\
         SVar:Stagger:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Card,Emblem | ValidTarget$ Player.IsRemembered,Permanent.ControlledBy Player.IsRemembered | ReplaceWith$ DmgTwice\n\
         SVar:DmgTwice:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ StaggerX\n\
         SVar:StaggerX:ReplaceCount$DamageAmount/Twice",
    );
    assert!(
        lightning.abilities.iter().any(|a| a.contains(
            "Modifier::MultiplyDamageFrom { sources: &Filter::Any, factor: 2, combat_only: false, noncombat_only: false, recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::DamagedPlayer) }, Duration::UntilYourNextTurn"
        )),
        "{:?}",
        lightning.abilities
    );

    let lunar = read(
        "Name:Lunar Convocation\nManaCost:W B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ Both | SVarCompare$ GE1 | Execute$ TrigTok\n\
         SVar:TrigTok:DB$ LoseLife | Defined$ You | LifeAmount$ 1\n\
         SVar:Both:PlayerCountPropertyYou$LifeLostThisTurn/Times.Gained\n\
         SVar:Gained:Count$LifeYouGainedThisTurn",
    );
    assert!(
        lunar.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 1)")
            && lunar.abilities[0]
                .contains("Condition::AmountAtLeast(Amount::LifeLostThisTurn(PlayerRel::You), 1)"),
        "{}",
        lunar.abilities[0]
    );

    let greven = read(
        "Name:GrevenProbe\nManaCost:3 B R\nTypes:Legendary Creature Wizard\nPT:5/5\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ X\n\
         SVar:X:Count$LifeYouLostThisTurn",
    );
    assert!(
        greven.abilities[0].contains("Amount::LifeLostThisTurn(PlayerRel::You)"),
        "{}",
        greven.abilities[0]
    );

    let energy_pt = read(
        "Name:DynamoProbe\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ Y\n\
         SVar:X:Count$YourCountersEnergy\n\
         SVar:Y:Count$CardCounters.CHARGE",
    );
    assert!(
        energy_pt.abilities.iter().any(|a| {
            a.contains("Amount::CountersOnYou(CounterKind::Energy)")
                && a.contains("Amount::CountersOnSource(CounterKind::Charge)")
        }),
        "{:?}",
        energy_pt.abilities
    );
    assert!(refused(
        "Name:UnmodeledExpProbe\nManaCost:R W B\nTypes:Creature Wizard\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X\n\
         SVar:X:Count$YourCountersExperience"
    ));

    let nerd_rage = read(
        "Name:Nerd Rage\nManaCost:2 U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 2\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | AddStaticAbility$ NoMaxHand\n\
         SVar:NoMaxHand:Mode$ Continuous | Affected$ You | SetMaxHandSize$ Unlimited",
    );
    assert!(
        nerd_rage.abilities.iter().any(|a| {
            a.contains("Modifier::GrantStatic { filter: &Filter::Any, modifier: &Modifier::NoMaxHandSize }")
        }),
        "{:?}",
        nerd_rage.abilities
    );
}

#[test]
fn wave29d_unless_costs_and_choose_number() {
    let crab = read(
        "Name:Mirrorshell Crab\nManaCost:5 U U\nTypes:Artifact Creature Wizard\nPT:5/7\n\
         K:Ward:3\n\
         A:AB$ Counter | Cost$ 2 U Discard<1/CARDNAME> | ActivationZone$ Hand | TargetType$ Spell,Activated,Triggered | ValidTgts$ Card | UnlessCost$ 3",
    );
    assert!(
        crab.abilities
            .iter()
            .any(|a| a.contains("TargetSpec::SpellOrAbility(&Filter::Any)")
                && a.contains("Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, mana: Amount::Fixed(3), effect: &Effect::CounterTargetSpellOrAbility }")),
        "{:?}",
        crab.abilities
    );

    let prince = read(
        "Name:Prince of Thralls\nManaCost:4 U B B R\nTypes:Creature Wizard\nPT:7/7\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Permanent.OppCtrl | TriggerZones$ Battlefield | Execute$ TrigSteal\n\
         SVar:TrigSteal:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | UnlessCost$ PayLife<3> | UnlessPayer$ TriggeredCardController",
    );
    assert!(
        prince.abilities[0].contains(
            "Effect::PlayerMayPayLifeOr { player: PlayerRel::ControllerOfEvent, life: Amount::Fixed(3)"
        ),
        "{}",
        prince.abilities[0]
    );

    let spike = read(
        "Name:Aether Spike\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 2 | SubAbility$ DBChooseNumber\n\
         SVar:DBChooseNumber:DB$ ChooseNumber | Max$ Max | ListTitle$ amount of energy to pay | SubAbility$ DBPay\n\
         SVar:DBPay:DB$ Nothing | UnlessCost$ Mandatory PayEnergy<N> | UnlessPayer$ You | UnlessSwitched$ True | SubAbility$ DBCounter\n\
         SVar:DBCounter:DB$ Counter | TargetType$ Spell | TgtZone$ Stack | ValidTgts$ Card | UnlessCost$ N\n\
         SVar:Max:Count$YourCountersEnergy\n\
         SVar:N:Count$ChosenNumber",
    );
    assert!(
        spike.abilities[0].contains("Amount::ResolutionChosenNumber")
            && spike.abilities[0].contains("Effect::CounterTarget"),
        "{}",
        spike.abilities[0]
    );

    let plunge = read(
        "Name:PlungeProbe\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ ChooseNumber | Defined$ You | ListTitle$ amount of life to pay | Max$ LifeAmountX | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | DigNum$ Y | DestinationZone2$ Exile | UnlessCost$ PayLife<Y> | UnlessSwitched$ True | UnlessPayer$ You\n\
         SVar:LifeAmountX:Count$YourLifeTotal/LimitMin.0\n\
         SVar:Y:Count$ChosenNumber",
    );
    assert!(
        plunge.abilities[0].contains("max: Amount::LifeTotal(PlayerRel::You)")
            && plunge.abilities[0].contains("pay_cost: Some(VariablePayKind::Life)"),
        "{}",
        plunge.abilities[0]
    );

    let electrozoa = read(
        "Name:Electrozoa\nManaCost:2 U\nTypes:Creature Goblin\nPT:3/1\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEnergy\n\
         SVar:TrigEnergy:DB$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | Defined$ Self | UnlessCost$ PayEnergy<1> | UnlessPayer$ You",
    );
    assert!(
        electrozoa
            .abilities
            .iter()
            .any(|a| a.contains("Effect::MayPayEnergyOr { amount: 1, effect: &Effect::TapSelf }")),
        "{:?}",
        electrozoa.abilities
    );

    let runner = read(
        "Name:Lightning Runner\nManaCost:3 R R\nTypes:Creature Wizard\nPT:2/2\n\
         K:Double Strike\nK:Haste\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigEnergy\n\
         SVar:TrigEnergy:DB$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 2 | SubAbility$ DBExtra\n\
         SVar:DBExtra:DB$ UntapAll | ValidCards$ Creature.YouCtrl | UnlessCost$ PayEnergy<8> | UnlessPayer$ You | UnlessSwitched$ True",
    );
    assert!(
        runner.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::CountersOnYou(CounterKind::Energy), 8)")
            && runner.abilities[0].contains("pay_cost: Some(VariablePayKind::Energy)"),
        "{}",
        runner.abilities[0]
    );

    let withercrown = read(
        "Name:Withercrown\nManaCost:1 B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | SetPower$ 0\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ EnchantedController | TriggerZones$ Battlefield | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseLife | Defined$ EnchantedController | LifeAmount$ 1 | UnlessCost$ Sac<1/CARDNAME> | UnlessPayer$ EnchantedController",
    );
    assert!(
        withercrown.abilities.iter().any(|a| a.contains(
            "Effect::PlayerMayPayCostOr { player: PlayerRel::ControllerOfAttached, cost: &CostPart::Sacrifice(&Filter::This)"
        )),
        "{:?}",
        withercrown.abilities
    );

    let gallia = read(
        "Name:GalliaProbe\nManaCost:R G\nTypes:Legendary Creature Wizard\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 2 | Defined$ You | UnlessCost$ Discard<1/Random> | UnlessPayer$ You | UnlessSwitched$ True",
    );
    assert!(
        gallia.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), 1)")
            && gallia.abilities[0]
                .contains("Effect::DiscardRandom { who: PlayerRel::You, count: Amount::Fixed(1) }"),
        "{}",
        gallia.abilities[0]
    );

    // Honest refusal boundaries preserved:
    assert!(refused(
        "Name:BadEnergyTax\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ PayEnergy<2>"
    ));
    assert!(refused(
        "Name:BadUnswitchedRandomDiscard\nManaCost:1 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseLife | Defined$ You | LifeAmount$ 2 | UnlessCost$ Discard<1/Random> | UnlessPayer$ You"
    ));
    assert!(refused(
        "Name:BadLimitMinZeroDmg\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$YourLifeTotal/LimitMin.0"
    ));
}
