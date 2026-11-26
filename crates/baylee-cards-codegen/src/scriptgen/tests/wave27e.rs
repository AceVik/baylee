use super::*;

#[test]
fn wave27e_shuffle_into_library_instead_of_graveyard_and_refusals() {
    // Blightsteel Colossus / Darksteel Colossus / Progenitus / Nexus of Fate shape.
    let colossus = read(
        "Name:ColossusProbe\nManaCost:11\nTypes:Artifact Creature Golem\nPT:11/11\n\
         K:Trample\nK:Indestructible\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True",
    );
    assert_eq!(
        colossus.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard { \
             filter: &Filter::This, reveal: true })"
        ]
    );

    // Without Reveal$ True -> reveal: false.
    let no_reveal = read(
        "Name:QuietShuffleProbe\nManaCost:5\nTypes:Artifact\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Shuffle$ True",
    );
    assert_eq!(
        no_reveal.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard { \
             filter: &Filter::This, reveal: false })"
        ]
    );

    // Battlefield permanent replacing other cards moving to graveyard from anywhere.
    let external = read(
        "Name:LibrarySanctuaryProbe\nManaCost:3 W\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Creature.!token+YouOwn | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True",
    );
    assert!(
        external.abilities[0]
            .starts_with("AbilityDef::Replacement(ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard { filter: &SHUFFLE_GY"),
        "{:?}",
        external.abilities
    );
    assert!(external.abilities[0].ends_with(", reveal: true })"));

    // Honest refusals:
    // 1. Origin$ Battlefield ("would die" only, e.g. Saiba Syphoner) — rule is "from anywhere".
    assert!(refused(
        "Name:DieOnlyShuffle\nManaCost:2 U U\nTypes:Creature\nPT:2/2\n\
         R:Event$ Moved | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Graveyard | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Destination$ Library | Shuffle$ True | Defined$ ReplacedCard"
    ));
    // 2. Bottom/top of library without Shuffle$ True (Wheel of Sun and Moon / Nissa's Chosen / Gravebane Zombie).
    assert!(refused(
        "Name:BottomNotShuffle\nManaCost:G G\nTypes:Creature\nPT:2/3\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ DBBot\n\
         SVar:DBBot:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         LibraryPosition$ -1 | Defined$ ReplacedCard"
    ));
    // 3. Card.Self restricted to ActiveZones$ Battlefield (must work from all zones).
    assert!(refused(
        "Name:BfOnlySelfShuffle\nManaCost:5\nTypes:Creature\nPT:5/5\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True"
    ));
    // 4. Optional$ True or unexpected ReplacementResult$.
    assert!(refused(
        "Name:OptionalShuffle\nManaCost:5\nTypes:Creature\nPT:5/5\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | Optional$ True | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True"
    ));
    assert!(refused(
        "Name:UpdatedResultShuffle\nManaCost:5\nTypes:Creature\nPT:5/5\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplacementResult$ Updated | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True"
    ));
    // 5. SubAbility$ on DBShuffle.
    assert!(refused(
        "Name:ChainedShuffle\nManaCost:5\nTypes:Creature\nPT:5/5\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | \
         Defined$ ReplacedCard | Reveal$ True | Shuffle$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1"
    ));
}

#[test]
fn wave27e_enter_with_counters_from_source_and_put_each_counter_kind() {
    // Blue, Loyal Raptor replacement shape.
    let raptor = read(
        "Name:RaptorProbe\nManaCost:2 G U\nTypes:Legendary Creature Goblin\nPT:5/4\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Goblin.YouCtrl+Other | \
         Destination$ Battlefield | ReplaceWith$ EnterWithCounters | ReplacementResult$ Updated\n\
         SVar:EnterWithCounters:DB$ PutCounter | ETB$ True | CounterType$ EachFromSource | \
         EachFromSource$ Self | Defined$ ReplacedCard | CounterNum$ 1",
    );
    assert_eq!(raptor.abilities.len(), 1);
    assert!(
        raptor.abilities[0].starts_with(
            "AbilityDef::Replacement(ReplacementRule::EnterWithCountersFromSource { filter: &ENTER"
        ),
        "{:?}",
        raptor.abilities
    );
    assert!(
        raptor.statics.contains("Filter::Another")
            && raptor.statics.contains("Filter::ControlledByYou"),
        "{}",
        raptor.statics
    );

    // Effect::PutEachCounterKindFromSource — targeted creature.
    let targeted = read(
        "Name:ShareCountersProbe\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ PutCounter | Cost$ T | ValidTgts$ Creature.YouCtrl | \
         CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1",
    );
    assert!(
        targeted.abilities[0]
            .contains("Effect::PutEachCounterKindFromSource { target: TargetSpec::Object("),
        "{:?}",
        targeted.abilities
    );

    // Effect::PutEachCounterKindFromSource — triggered entering creature (EventObject).
    let triggered_etb = read(
        "Name:TriggerShareProbe\nManaCost:2 W U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Creature.Other+YouCtrl | Origin$ Any | \
         Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ TrigShare\n\
         SVar:TrigShare:DB$ PutCounter | Defined$ TriggeredCardLKICopy | \
         CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1",
    );
    assert!(
        triggered_etb.abilities[0]
            .contains("Effect::PutEachCounterKindFromSource { target: TargetSpec::EventObject }"),
        "{:?}",
        triggered_etb.abilities
    );

    // Multi-zone GreatestCardPower K:etbCounter (Ambitious Dragonborn shape).
    let dragonborn = read(
        "Name:DragonbornProbe\nManaCost:3 G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition:Enters with X +1/+1 counters.\n\
         SVar:X:Count$ValidGraveyard,Battlefield Creature.YouCtrl$GreatestCardPower",
    );
    assert_eq!(dragonborn.enter_modifiers.len(), 1);
    assert!(
        dragonborn.enter_modifiers[0]
            .contains("EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::GreatestPowerAmong(&COUNT"),
        "{:?}",
        dragonborn.enter_modifiers
    );
    assert!(
        dragonborn.statics.contains("ZoneRef::Battlefield")
            && dragonborn.statics.contains("ZoneRef::Graveyard")
            && dragonborn.statics.contains("Filter::YOUR_CREATURE")
            && dragonborn.statics.contains("Filter::OwnedByYou"),
        "{}",
        dragonborn.statics
    );

    // Effect::PutEachCounterKindFromSource — self target (`Defined$ Self`).
    let self_copy = read(
        "Name:SelfCounterProbe\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ EachFromSource | \
         EachFromSource$ Self | CounterNum$ 1",
    );
    assert!(
        self_copy.abilities[0]
            .contains("Effect::PutEachCounterKindFromSource { target: TargetSpec::ThisObject }"),
        "{:?}",
        self_copy.abilities
    );

    // Honest refusal on a spell (`A:SP$`) and on non-targeted `Defined$ Enchanted`.
    assert!(refused(
        "Name:SpellEachFromSource\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1"
    ));
    assert!(refused(
        "Name:AuraEachFromSource\nManaCost:1 G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ PutCounter | Cost$ 1 | Defined$ Enchanted | CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 1"
    ));
}

#[test]
fn wave27e_each_from_source_refusals() {
    // Honest refusals for EachFromSource:
    // 1. Missing CounterNum$ 1 (copies ALL counters, e.g. Star Pupil / Iron Apprentice / Denry Klin).
    assert!(refused(
        "Name:AllCountersCopy\nManaCost:W\nTypes:Creature\nPT:0/0\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ EachFromSource | EachFromSource$ Self"
    ));
    // 2. CounterNum$ 2 instead of 1.
    assert!(refused(
        "Name:TwoEachCounter\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ PutCounter | Cost$ T | ValidTgts$ Creature.YouCtrl | CounterType$ EachFromSource | EachFromSource$ Self | CounterNum$ 2"
    ));
    // 3. EachFromSource$ TriggeredCardLKICopy (not Self).
    assert!(refused(
        "Name:OtherSourceCounters\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ EachFromSource | EachFromSource$ TriggeredCardLKICopy | CounterNum$ 1"
    ));
    // 4. ETB replacement missing CounterNum$ 1 or wrong ReplacementResult$.
    assert!(refused(
        "Name:RaptorAllCounters\nManaCost:2 G U\nTypes:Creature\nPT:5/4\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Goblin.YouCtrl+Other | \
         Destination$ Battlefield | ReplaceWith$ EnterWithCounters | ReplacementResult$ Updated\n\
         SVar:EnterWithCounters:DB$ PutCounter | ETB$ True | CounterType$ EachFromSource | \
         EachFromSource$ Self | Defined$ ReplacedCard"
    ));
    assert!(refused(
        "Name:RaptorWrongResult\nManaCost:2 G U\nTypes:Creature\nPT:5/4\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Goblin.YouCtrl+Other | \
         Destination$ Battlefield | ReplaceWith$ EnterWithCounters | ReplacementResult$ Replaced\n\
         SVar:EnterWithCounters:DB$ PutCounter | ETB$ True | CounterType$ EachFromSource | \
         EachFromSource$ Self | Defined$ ReplacedCard | CounterNum$ 1"
    ));
}

#[test]
fn wave27e_modify_life_gain_and_refusals() {
    // Boon Reflection / Alhammarret's Archive / Rhox Faithmender shape (Twice -> Multiply(2)).
    let boon = read(
        "Name:BoonProbe\nManaCost:4 W\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ GainDouble | AILogic$ DoubleLife\n\
         SVar:GainDouble:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ X\n\
         SVar:X:ReplaceCount$LifeGained/Twice",
    );
    assert_eq!(
        boon.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ModifyLifeGain { who: PlayerRel::You, op: LifeGainOp::Multiply(2) })"
        ]
    );

    // Cleric Class / Honor Troll / Angel of Vitality shape (Plus.1 -> Add(1)).
    let cleric = read(
        "Name:ClericProbe\nManaCost:W\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ GainPlus\n\
         SVar:GainPlus:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ X\n\
         SVar:X:ReplaceCount$LifeGained/Plus.1",
    );
    assert_eq!(
        cleric.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ModifyLifeGain { who: PlayerRel::You, op: LifeGainOp::Add(1) })"
        ]
    );

    // Phial of Galadriel conditional life-gain doubler (CheckSVar$ Y | SVarCompare$ LE5).
    let phial = read(
        "Name:PhialProbe\nManaCost:3\nTypes:Artifact\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | CheckSVar$ Y | SVarCompare$ LE5 | \
         ValidPlayer$ You | ReplaceWith$ GainDouble | AILogic$ DoubleLife\n\
         SVar:GainDouble:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ X\n\
         SVar:X:ReplaceCount$LifeGained/Twice\n\
         SVar:Y:Count$YourLifeTotal",
    );
    assert_eq!(
        phial.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ModifyLifeGain { who: PlayerRel::You, op: LifeGainOp::Multiply(2) }, condition = Some(Condition::LifeAtMost(5)))"
        ]
    );

    // Arbitrary Times.3 / Plus.2 and opponent/all-player targets.
    let triple_all = read(
        "Name:TripleLifeProbe\nManaCost:5 W\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ Player | ReplaceWith$ GainTriple\n\
         SVar:GainTriple:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ ReplaceCount$LifeGained/Thrice",
    );
    assert_eq!(
        triple_all.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ModifyLifeGain { who: PlayerRel::EachPlayer, op: LifeGainOp::Multiply(3) })"
        ]
    );

    // Honest refusals:
    // 1. Tainted Remedy / Plague Drone (LoseLife instead of GainLife).
    assert!(refused(
        "Name:TaintedProbe\nManaCost:2 B\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ Opponent | ReplaceWith$ RLoseLife\n\
         SVar:RLoseLife:DB$ LoseLife | LifeAmount$ X | Defined$ ReplacedPlayer\n\
         SVar:X:ReplaceCount$LifeGained"
    ));
    // 2. Rain of Gore (ValidSource$ SpellAbility | SourceController$ True).
    assert!(refused(
        "Name:RainOfGoreProbe\nManaCost:B R\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidSource$ SpellAbility | SourceController$ True | ReplaceWith$ GainDouble\n\
         SVar:GainDouble:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ ReplaceCount$LifeGained/Twice"
    ));
    // 3. Unsupported op (Minus.1, HalfDown, Plus.0, Times.1).
    for bad_op in [
        "ReplaceCount$LifeGained/Minus.1",
        "ReplaceCount$LifeGained/HalfDown",
        "ReplaceCount$LifeGained/Plus.0",
        "ReplaceCount$LifeGained/Times.1",
    ] {
        assert!(
            refused(&format!(
                "Name:BadLifeOp\nManaCost:3 W\nTypes:Enchantment\n\
                 R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ Bad\n\
                 SVar:Bad:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ {bad_op}"
            )),
            "must refuse {bad_op}"
        );
    }
    // 4. Non-battlefield ActiveZones$.
    assert!(refused(
        "Name:CommandLifeDouble\nManaCost:3 W\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Command | ValidPlayer$ You | ReplaceWith$ GainDouble\n\
         SVar:GainDouble:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ ReplaceCount$LifeGained/Twice"
    ));
}

#[test]
fn wave27e_damage_subtract_cap_counter_riders_and_per_recipient_riders() {
    // 1. Benevolent Unicorn (DamageAmountReplacement::Subtract(1) from Spell sources).
    let unicorn = read(
        "Name:UnicornProbe\nManaCost:1 W\nTypes:Creature\nPT:1/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Spell | \
         ValidTarget$ Permanent,Player | ReplaceWith$ DmgMinus1\n\
         SVar:DmgMinus1:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/Minus.1",
    );
    assert_eq!(unicorn.abilities.len(), 1);
    assert!(
        unicorn.abilities[0].contains("Modifier::ReplaceDamage")
            && unicorn.abilities[0].contains("recipients: DamageRecipients::Any")
            && unicorn.abilities[0].contains("replacement: DamageAmountReplacement::Subtract(1)"),
        "{:?}",
        unicorn.abilities
    );
    assert!(unicorn.statics.contains("Filter::InZone(ZoneRef::Stack)"));

    // 2. Lashknife Barrier (Subtract(1) to Creature.YouCtrl).
    let barrier = read(
        "Name:BarrierProbe\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Creature.YouCtrl | ReplaceWith$ DmgMinus1\n\
         SVar:DmgMinus1:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/Minus.1",
    );
    assert!(
        barrier.abilities[0].contains("Modifier::ReplaceDamage")
            && barrier.abilities[0].contains("sources: &Filter::Any")
            && barrier.abilities[0]
                .contains("DamageRecipients::Permanents(&Filter::YOUR_CREATURE)")
            && barrier.abilities[0].contains("replacement: DamageAmountReplacement::Subtract(1)"),
        "{:?}",
        barrier.abilities
    );

    // 3. Divine Presence (generalized DamageAmountReplacement::Cap(3) across Permanent,Player).
    let presence = read(
        "Name:PresenceProbe\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Permanent,Player | \
         DamageAmount$ GE4 | ReplaceWith$ Dmg3\n\
         SVar:Dmg3:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ 3",
    );
    assert_eq!(
        presence.abilities,
        ["static_ability!(Filter::Any, Modifier::ReplaceDamage { \
             sources: &Filter::Any, recipients: DamageRecipients::Any, combat: None, \
             replacement: DamageAmountReplacement::Cap(3) })"]
    );

    // 4. Bloatfly Swarm (Modifier::PreventDamageRemovingCounters with Effect::Rad rider).
    let bloatfly = read(
        "Name:BloatflyProbe\nManaCost:3 B\nTypes:Creature\nPT:0/0\n\
         K:Flying\nK:etbCounter:P1P1:5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_P1P1 | \
         ReplaceWith$ Counters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Counters:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | \
         CounterNum$ Y | RememberAmount$ True | SubAbility$ DBRadiation\n\
         SVar:DBRadiation:DB$ Radiation | Defined$ Player | Num$ Z | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:ReplaceCount$DamageAmount\n\
         SVar:Z:Count$RememberedNumber",
    );
    assert_eq!(
        bloatfly.abilities,
        [
            "static_ability!(Filter::This, Modifier::PreventDamageRemovingCounters { \
             kind: CounterKind::P1P1, then: Some(DamageRider::new(&[Effect::Rad { \
             who: PlayerRel::EachPlayer, amount: Amount::ThisWay(ThisWay::DamagePrevented) }])) })"
        ]
    );
}

#[test]
fn wave27e_per_recipient_damage_riders_and_refusals() {
    // 5. Crumbling Sanctuary (per-recipient ExileTop rider with PlayerRel::DamagedPlayer).
    let sanctuary = read(
        "Name:SanctuaryProbe\nManaCost:5\nTypes:Artifact\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Player | \
         ReplaceWith$ ExileTop | ExecuteMode$ PerTarget\n\
         SVar:ExileTop:DB$ Dig | Defined$ ReplacedTarget | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        sanctuary.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen { \
             sources: &Filter::Any, recipients: DamageRecipients::Players(PlayerRel::EachPlayer), \
             combat: None, is_replacement: true, then: DamageRider::new(&[Effect::ExileTop { \
             who: PlayerRel::DamagedPlayer, count: Amount::ThisWay(ThisWay::DamagePrevented) }]) })"
        ]
    );

    // 6. Undead Alchemist (per-recipient combat Mill rider with PlayerRel::DamagedPlayer).
    let alchemist = read(
        "Name:AlchemistProbe\nManaCost:3 U\nTypes:Creature Goblin\nPT:4/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Creature.Goblin+YouCtrl | \
         ValidTarget$ Player | ReplaceWith$ Mill | IsCombat$ True | ExecuteMode$ PerTarget\n\
         SVar:Mill:DB$ Mill | Defined$ ReplacedTarget | NumCards$ X\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert!(
        alchemist.abilities[0].contains("combat: Some(true), is_replacement: true")
            && alchemist.abilities[0].contains(
                "Effect::Mill { amount: Amount::ThisWay(ThisWay::DamagePrevented), target: PlayerRel::DamagedPlayer }"
            ),
        "{:?}",
        alchemist.abilities
    );

    // 7. Angel of Suffering (prevent damage to you -> mill twice that many cards).
    let suffering = read(
        "Name:SufferingProbe\nManaCost:3 B B\nTypes:Creature\nPT:5/3\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | \
         ReplaceWith$ DoubleMill | AlwaysReplace$ True | PreventionEffect$ True\n\
         SVar:DoubleMill:DB$ Mill | Defined$ ReplacedTarget | NumCards$ X\n\
         SVar:X:ReplaceCount$DamageAmount/Twice",
    );
    assert_eq!(
        suffering.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen { \
             sources: &Filter::Any, recipients: DamageRecipients::Players(PlayerRel::You), \
             combat: None, is_replacement: false, then: DamageRider::new(&[Effect::Mill { \
             amount: Amount::Times { base: &Amount::ThisWay(ThisWay::DamagePrevented), factor: 2 }, \
             target: PlayerRel::You }]) })"
        ]
    );

    // 8. Per-recipient Effect::Rad rider on ReplaceOrPreventDamageWithThen.
    let rad_sanctuary = read(
        "Name:RadSanctuaryProbe\nManaCost:4\nTypes:Artifact\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Player | \
         ReplaceWith$ DBRad | ExecuteMode$ PerTarget\n\
         SVar:DBRad:DB$ Radiation | Defined$ ReplacedTarget | Num$ X\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        rad_sanctuary.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen { \
             sources: &Filter::Any, recipients: DamageRecipients::Players(PlayerRel::EachPlayer), \
             combat: None, is_replacement: true, then: DamageRider::new(&[Effect::Rad { \
             who: PlayerRel::DamagedPlayer, amount: Amount::ThisWay(ThisWay::DamagePrevented) }]) })"
        ]
    );

    // 9. PreventDamageRemovingCounters with GainLife rider (without Cleanup SVar).
    let life_hydra = read(
        "Name:LifeHydraProbe\nManaCost:2 G W\nTypes:Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_P1P1 | \
         ReplaceWith$ Counters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Counters:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | \
         CounterNum$ Y | RememberAmount$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ Z\n\
         SVar:Y:ReplaceCount$DamageAmount\n\
         SVar:Z:Count$RememberedNumber",
    );
    assert_eq!(
        life_hydra.abilities,
        [
            "static_ability!(Filter::This, Modifier::PreventDamageRemovingCounters { \
             kind: CounterKind::P1P1, then: Some(DamageRider::new(&[Effect::GainLife { \
             amount: Amount::ThisWay(ThisWay::DamagePrevented) }])) })"
        ]
    );
}

#[test]
fn wave27e_damage_and_rider_refusals() {
    // Honest refusals:
    // A. Subtract(0) or variable subtraction.
    assert!(refused(
        "Name:SubZero\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Permanent,Player | ReplaceWith$ DmgMinus0\n\
         SVar:DmgMinus0:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Minus.0"
    ));
    // B. Sekki shape: RemoveCounter + Token using DamageAmount & unguarded Card.Self -> ReplaceOrPreventDamageScopedWithThen.
    let sekki = transcode(
        &parse(
            "Name:SekkiProbe\nManaCost:5 G G G\nTypes:Creature\nPT:0/0\n\
             R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | \
             ReplaceWith$ SekkiCounters | PreventionEffect$ True | AlwaysReplace$ True\n\
             SVar:SekkiCounters:DB$ RemoveCounter | CounterType$ P1P1 | CounterNum$ SekkiX | SubAbility$ DBTok\n\
             SVar:DBTok:DB$ Token | TokenAmount$ SekkiX | TokenScript$ r_1_1_goblin\n\
             SVar:SekkiX:ReplaceCount$DamageAmount",
        ),
        &cats(),
        Some(&tokens()),
    )
    .expect("SekkiProbe reads via ReplaceOrPreventDamageScopedWithThen");
    assert!(
        sekki.abilities[0].contains("Modifier::ReplaceOrPreventDamageScopedWithThen")
            && sekki.abilities[0].contains("Effect::RemoveCounterSelf")
            && sekki.abilities[0].contains("Effect::CreateTokenN"),
        "{:?}",
        sekki.abilities
    );
    // C. Magma Pummeler shape: RemoveCounter + ImmediateTrigger (not a DamageRider).
    assert!(refused(
        "Name:PummelerProbe\nManaCost:X R R\nTypes:Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self+counters_GE1_P1P1 | \
         ReplaceWith$ Counters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Counters:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | \
         CounterNum$ Y | RememberAmount$ True | SubAbility$ DBTrig\n\
         SVar:DBTrig:DB$ ImmediateTrigger | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1\n\
         SVar:Y:ReplaceCount$DamageAmount"
    ));
    // D. Vigor / Soul-Scar Mage: ExecuteMode$ PerTarget on permanent recipients with Defined$ ReplacedTarget.
    let vigor = read(
        "Name:VigorProbe\nManaCost:3 G G G\nTypes:Creature\nPT:6/6\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Creature.YouCtrl+Other | \
         ReplaceWith$ Counters | PreventionEffect$ True | ExecuteMode$ PerTarget\n\
         SVar:Counters:DB$ PutCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert!(
        vigor.abilities[0].contains("Modifier::ReplaceOrPreventDamageScopedWithThen")
            && vigor.abilities[0].contains("scope: DamageRiderScope::PerTarget")
            && vigor.abilities[0].contains("Effect::AddCounterDamagedObject { kind: CounterKind::P1P1, amount: Amount::ThisWay(ThisWay::DamagePrevented) }"),
        "{:?}",
        vigor.abilities
    );
    // E. Unsupported ActiveZones$ on PreventDamageRemovingCounters.
    assert!(refused(
        "Name:CommandBloatfly\nManaCost:3 B\nTypes:Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Command | ValidTarget$ Card.Self+counters_GE1_P1P1 | \
         ReplaceWith$ Counters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Counters:DB$ RemoveCounter | Defined$ ReplacedTarget | CounterType$ P1P1 | \
         CounterNum$ Y | RememberAmount$ True | SubAbility$ DBRad\n\
         SVar:DBRad:DB$ Radiation | Defined$ Player | Num$ Z\n\
         SVar:Y:ReplaceCount$DamageAmount\n\
         SVar:Z:Count$RememberedNumber"
    ));
    // F. Non-paper Alchemy / Un-set lines stay refused.
    assert!(refused(
        "Name:AlchemyProbe\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ GainDouble\n\
         SVar:GainDouble:DB$ ReplaceEffect | VarName$ LifeGained | VarValue$ ReplaceCount$LifeGained/Twice\n\
         A:AB$ Seek | Cost$ 1 | Type$ Creature"
    ));
}
