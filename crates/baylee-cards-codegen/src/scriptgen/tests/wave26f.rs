use super::*;

#[test]
fn wave26f_prevention_half_damage_replacements() {
    let ghosts = read(
        "Name:Ghosts of the Innocent\nManaCost:5 W W\nTypes:Creature Spirit\nPT:4/5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Player,Permanent | \
         ReplaceWith$ DmgHalfDown\n\
         SVar:DmgHalfDown:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/HalfDown",
    );
    assert_eq!(
        ghosts.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceDamage { sources: &Filter::Any, \
             recipients: DamageRecipients::Any, combat: None, \
             replacement: DamageAmountReplacement::HalfDown })"
        ]
    );

    let half_up = read(
        "Name:HalfUpGuard\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | IsCombat$ True | \
         ReplaceWith$ DmgHalfUp\n\
         SVar:DmgHalfUp:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/HalfUp",
    );
    assert_eq!(
        half_up.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceDamage { sources: &Filter::Any, \
             recipients: DamageRecipients::Players(PlayerRel::You), combat: Some(true), \
             replacement: DamageAmountReplacement::HalfUp })"
        ]
    );

    // Prevention-marked HalfUp replacement (*Gisela* second clause) must stay refused.
    assert!(refused(
        "Name:GiselaHalf\nManaCost:4 R W W\nTypes:Creature Angel\nPT:5/5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | \
         PreventionEffect$ True | ReplaceWith$ Half\n\
         SVar:Half:DB$ ReplaceDamage | Amount$ X\n\
         SVar:X:ReplaceCount$DamageAmount/HalfUp"
    ));
}

#[test]
fn wave26f_prevention_replace_or_prevent_with_then() {
    let phytohydra = read(
        "Name:Phytohydra\nManaCost:2 G W W\nTypes:Creature Plant Hydra\nPT:1/1\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | \
         ReplaceWith$ Counters\n\
         SVar:Counters:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        phytohydra.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen { \
             sources: &Filter::Any, recipients: DamageRecipients::Permanents(&Filter::This), \
             combat: None, is_replacement: true, \
             then: &[Effect::AddCounterSource { kind: CounterKind::P1P1, \
             amount: Amount::ThisWay(ThisWay::DamagePrevented) }] })"
        ]
    );

    let gloom = read(
        "Name:Gloom Surgeon\nManaCost:1 B\nTypes:Creature Spirit\nPT:2/1\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | \
         IsCombat$ True | ReplaceWith$ Exile | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Exile:DB$ Dig | DigNum$ X | ChangeNum$ All | ChangeValid$ Card | \
         DestinationZone$ Exile\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        gloom.abilities,
        [
            "static_ability!(Filter::Any, Modifier::ReplaceOrPreventDamageWithThen { \
             sources: &Filter::Any, recipients: DamageRecipients::Permanents(&Filter::This), \
             combat: Some(true), is_replacement: false, \
             then: &[Effect::ExileTop { who: PlayerRel::You, \
             count: Amount::ThisWay(ThisWay::DamagePrevented) }] })"
        ]
    );

    let mindskinner = read(
        "Name:The Mindskinner\nManaCost:U U U\nTypes:Legendary Enchantment Creature Nightmare\n\
         PT:10/1\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | \
         ValidSource$ Card.YouCtrl,Emblem.YouCtrl | ValidTarget$ Opponent | \
         ReplaceWith$ Mill | AlwaysReplace$ True | PreventionEffect$ True | \
         ExecuteMode$ PerSource\n\
         SVar:Mill:DB$ Mill | Defined$ Opponent | NumCards$ X\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert!(
        mindskinner.abilities.iter().any(|a| a
            .contains("Modifier::ReplaceOrPreventDamageWithThen")
            && a.contains("is_replacement: false")
            && a.contains(
                "Effect::Mill { amount: Amount::ThisWay(ThisWay::DamagePrevented), \
                     target: PlayerRel::EachOpponent }"
            )),
        "abilities: {:?}",
        mindskinner.abilities
    );

    let hostility = read_with_tokens(
        "Name:Hostility\nManaCost:3 R R R\nTypes:Creature Elemental\nPT:6/6\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Spell.YouCtrl | \
         ValidTarget$ Opponent | PreventionEffect$ True | ReplaceWith$ MakeGobs\n\
         SVar:MakeGobs:DB$ Token | TokenAmount$ X | TokenScript$ r_1_1_goblin | TokenOwner$ You\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert!(
        hostility
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ReplaceOrPreventDamageWithThen")
                && a.contains("Effect::CreateTokenN { token: &generated_tokens::")
                && a.contains("amount: Amount::ThisWay(ThisWay::DamagePrevented)")),
        "abilities: {:?}",
        hostility.abilities
    );
}

#[test]
fn wave26f_prevention_effects_with_then_and_temp_uncounterable() {
    let awe_strike = read(
        "Name:Awe Strike\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | Name$ Awe Struck | \
         ReplacementEffects$ StrikeWithAwe | ExileOnMoved$ Battlefield | \
         RememberObjects$ Targeted\n\
         SVar:StrikeWithAwe:Event$ DamageDone | ValidSource$ Card.IsRemembered | \
         ReplaceWith$ GainLifeInstead | PreventionEffect$ True\n\
         SVar:GainLifeInstead:DB$ GainLife | Defined$ You | LifeAmount$ X | \
         SubAbility$ ExileEffect\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert_eq!(
        awe_strike.abilities,
        ["spell!(&[Effect::PreventNextDamageWithThen { \
             sources: TargetSpec::Object(&Filter::CREATURE), \
             recipients: TargetSpec::AnyTarget, combat_only: false, \
             then: &[Effect::GainLife { \
             amount: Amount::ThisWay(ThisWay::DamagePrevented) }] }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"]
    );

    let inkshield = read_with_tokens(
        "Name:Inkshield\nManaCost:3 W B\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | ValidTarget$ You | PreventionEffect$ True | \
         ReplaceWith$ MakeTok | IsCombat$ True | ActiveZones$ Command\n\
         SVar:MakeTok:DB$ Token | TokenAmount$ X | TokenScript$ u_1_1_wizard_flying\n\
         SVar:X:ReplaceCount$DamageAmount",
    );
    assert!(
        inkshield
            .abilities
            .iter()
            .any(|a| a.contains("Effect::AddGlobalTempModifier")
                && a.contains("Modifier::ReplaceOrPreventDamageWithThen")
                && a.contains("combat: Some(true)")
                && a.contains("Effect::CreateTokenN { token: &generated_tokens::")
                && a.contains("duration: Duration::UntilEndOfTurn")),
        "abilities: {:?}",
        inkshield.abilities
    );

    let determined = read(
        "Name:Determined\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ CantCounter\n\
         SVar:CantCounter:Event$ Counter | ValidSA$ Spell.YouCtrl | Layer$ CantHappen | \
         ActiveZones$ Command",
    );
    assert_eq!(
        determined.abilities,
        [
            "spell!(&[Effect::AddGlobalTempModifier { filter: &Filter::Any, \
             modifier: Modifier::SpellsCantBeCountered { who: PlayerRel::You, \
             filter: &Filter::Any }, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn wave26f_static_spells_cant_be_countered_and_refusals() {
    let serpopard = read(
        "Name:Prowling Serpopard\nManaCost:1 G G\nTypes:Creature Cat Snake\nPT:4/3\n\
         R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen\n\
         R:Event$ Counter | ValidSA$ Spell.Creature+YouCtrl | Layer$ CantHappen | \
         ActiveZones$ Battlefield",
    );
    assert!(
        serpopard
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::UNCOUNTERABLE")
    );
    assert_eq!(
        serpopard.abilities,
        [
            "static_ability!(Filter::Any, Modifier::SpellsCantBeCountered { \
             who: PlayerRel::You, filter: &Filter::CREATURE })"
        ]
    );

    let destiny_spinner = read(
        "Name:Destiny Spinner\nManaCost:1 G\nTypes:Enchantment Creature Human\nPT:2/3\n\
         R:Event$ Counter | ValidSA$ Spell.Creature+YouCtrl,Spell.Enchantment+YouCtrl | \
         Layer$ CantHappen | ActiveZones$ Battlefield",
    );
    assert!(
        destiny_spinner
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SpellsCantBeCountered { who: PlayerRel::You")),
        "abilities: {:?}",
        destiny_spinner.abilities
    );
    assert!(
        destiny_spinner
            .statics
            .contains("Filter::Or(&[Filter::CREATURE, Filter::ENCHANTMENT])"),
        "statics: {}",
        destiny_spinner.statics
    );

    let lier = read(
        "Name:Lier Probe\nManaCost:3 U U\nTypes:Creature Human Wizard\nPT:3/4\n\
         R:Event$ Counter | ValidSA$ Spell | Layer$ CantHappen | ActiveZones$ Battlefield",
    );
    assert_eq!(
        lier.abilities,
        [
            "static_ability!(Filter::Any, Modifier::SpellsCantBeCountered { \
             who: PlayerRel::EachPlayer, filter: &Filter::Any })"
        ]
    );

    let taigam = read(
        "Name:Taigam Probe\nManaCost:2 W U\nTypes:Creature Human Monk\nPT:3/4\n\
         R:Event$ Counter | ValidCard$ Instant.YouCtrl,Sorcery.YouCtrl | ValidSA$ Spell | \
         Layer$ CantHappen | ActiveZones$ Battlefield",
    );
    assert!(
        taigam
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SpellsCantBeCountered { who: PlayerRel::You")),
        "abilities: {:?}",
        taigam.abilities
    );

    // Spider-Punk (abilities too), cast-by-you scoping, and Guile (ReplaceWith) stay refused.
    assert!(refused(
        "Name:SpiderPunk\nManaCost:1 R\nTypes:Creature Human\nPT:2/1\n\
         R:Event$ Counter | ValidSA$ Spell,Activated,Triggered | Layer$ CantHappen | \
         ActiveZones$ Battlefield"
    ));
    assert!(refused(
        "Name:ThryxGuard\nManaCost:3 U U\nTypes:Creature Elemental\nPT:4/5\n\
         R:Event$ Counter | ValidCard$ Card.cmcGE5+wasCastByYou | ValidSA$ Spell | \
         Layer$ CantHappen | ActiveZones$ Battlefield"
    ));
    assert!(refused(
        "Name:GuileGuard\nManaCost:3 U U U\nTypes:Creature Elemental\nPT:6/6\n\
         R:Event$ Counter | ActiveZones$ Battlefield | ValidSA$ Spell | \
         ValidCause$ SpellAbility.YouCtrl | ReplaceWith$ DBRemove\n\
         SVar:DBRemove:DB$ ChangeZone | Origin$ Stack | Destination$ Exile"
    ));
}

#[test]
fn wave26f_phase_check_svar_wave14b_metrics() {
    let devotion = read(
        "Name:DevotionUpkeep\nManaCost:2 W U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GE7 | Execute$ TrigDraw\n\
         SVar:X:Count$DevotionDual.White.Blue\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        devotion.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::Devotion { who: PlayerRel::You, \
             colors: ColorSet::from_slice(&[Color::White, Color::Blue]) }, \
             op: AmountCmp::AtLeast, right: &Amount::Fixed(7) }"
        ),
        "ability: {}",
        devotion.abilities[0]
    );

    let total_power = read(
        "Name:TotalPowerPhase\nManaCost:3 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GE10 | Execute$ TrigDraw\n\
         SVar:X:Count$Valid Goblin.YouCtrl$CardPower\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        total_power.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::TotalPowerAmong(&COUNT1), \
             op: AmountCmp::AtLeast, right: &Amount::Fixed(10) }"
        ),
        "ability: {}",
        total_power.abilities[0]
    );

    let greatest_power = read(
        "Name:GreatestPowerPhase\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GE6 | Execute$ TrigDraw\n\
         SVar:X:Count$ValidExile Card$GreatestCardManaCost\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        greatest_power.abilities[0].contains(
            "Condition::AmountCompare { \
             left: &Amount::GreatestManaValueAmong(&Filter::InZone(ZoneRef::Exile)), \
             op: AmountCmp::AtLeast, right: &Amount::Fixed(6) }"
        ),
        "ability: {}",
        greatest_power.abilities[0]
    );

    let party_and_hand = read(
        "Name:PartyPhase\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ P | SVarCompare$ GE4 | Execute$ TrigGain\n\
         SVar:P:Count$Party\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        party_and_hand.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::PartySize(PlayerRel::You), 4)"),
        "ability: {}",
        party_and_hand.abilities[0]
    );
}

#[test]
fn wave26f_phase_check_svar_turn_and_graveyard() {
    let impatience = read(
        "Name:Impatience\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ Player | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ EQ0 | Execute$ TrigDmg\n\
         SVar:X:Count$ThisTurnCast_Card.ControlledBy TriggeredPlayer\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 2",
    );
    assert!(
        impatience.abilities[0].contains(
            "Condition::AmountCompare { \
             left: &Amount::SpellsCastThisTurn(PlayerRel::ActivePlayer), \
             op: AmountCmp::Equal, right: &Amount::Fixed(0) }"
        ),
        "ability: {}",
        impatience.abilities[0]
    );

    let descend = read(
        "Name:DescendPhase\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ EQ4 | Execute$ TrigDraw\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        descend.abilities[0].contains(
            "Amount::CountOf { filter: &Filter::CREATURE, \
             zone: ZoneSel::GraveyardOf(PlayerRel::You) }"
        ),
        "ability: {}",
        descend.abilities[0]
    );

    let cross_life = read(
        "Name:CrossLifePhase\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Opponent | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GTY | Execute$ TrigGain\n\
         SVar:X:TriggeredPlayer$LifeTotal\n\
         SVar:Y:Count$YourLifeTotal/Plus.5\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        cross_life.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::Difference { \
             left: &Amount::LifeTotal(PlayerRel::ActivePlayer), \
             right: &Amount::Plus { base: &Amount::LifeTotal(PlayerRel::You), offset: 5 } }, 1)"
        ),
        "ability: {}",
        cross_life.abilities[0]
    );

    // Mixed graveyard owners must stay refused.
    assert!(refused(
        "Name:BadMixedGy\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         CheckSVar$ X | SVarCompare$ GE4 | Execute$ TrigDraw\n\
         SVar:X:Count$ValidGraveyard Permanent.YouOwn,Card.OppOwn\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave26f_etb_counter_unlocks() {
    let zegana = read(
        "Name:Zegana Probe\nManaCost:2 G G U U\nTypes:Creature Merfolk Wizard\nPT:1/1\n\
         K:etbCounter:P1P1:X:no Condition\n\
         SVar:X:Count$Valid Creature.Other+YouCtrl$GreatestCardPower",
    );
    assert_eq!(
        zegana.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::GreatestPowerAmong(&Filter::ANOTHER_CREATURE_YOU_CONTROL) }"]
    );

    let ulamog = read(
        "Name:Ulamog Probe\nManaCost:10\nTypes:Creature Eldrazi\nPT:7/7\n\
         K:etbCounter:P1P1:Y:no Condition\n\
         SVar:Y:Count$ValidExile Card$GreatestCardManaCost",
    );
    assert_eq!(
        ulamog.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::GreatestManaValueAmong(&Filter::InZone(ZoneRef::Exile)) }"]
    );

    let mindsinger = read(
        "Name:Vodalian Mindsinger Probe\nManaCost:1 U U\nTypes:Creature Merfolk Wizard\nPT:2/2\n\
         K:Multikicker:1 R\n\
         K:etbCounter:P1P1:X:CheckSVar$ X\n\
         SVar:X:Count$TimesKicked/Twice",
    );
    assert_eq!(
        mindsinger.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 2 } }"]
    );

    let effortless = read(
        "Name:Effortless Master\nManaCost:2 U R\nTypes:Creature Orc Monk\nPT:4/3\n\
         K:etbCounter:P1P1:2:CheckSVar$ X\n\
         SVar:X:Count$Compare Y GE2.1.0\n\
         SVar:Y:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert_eq!(
        effortless.enter_modifiers,
        ["EnterModifier::WithCountersIf { kind: CounterKind::P1P1, \
             count: Amount::Fixed(2), \
             condition: Condition::AmountCompare { \
             left: &Amount::SpellsCastThisTurn(PlayerRel::You), op: AmountCmp::AtLeast, \
             right: &Amount::Fixed(2) } }"]
    );

    let nontoken_devotion = read(
        "Name:DevotionEtb\nManaCost:2 G\nTypes:Creature Beast\nPT:0/0\n\
         K:etbCounter:P1P1:X:ValidCard$ Card.Self+!token\n\
         SVar:X:Count$Devotion.Green",
    );
    assert_eq!(
        nontoken_devotion.enter_modifiers,
        ["EnterModifier::WithCountersIf { kind: CounterKind::P1P1, \
             count: Amount::Devotion { who: PlayerRel::You, \
             colors: ColorSet::from_slice(&[Color::Green]) }, \
             condition: Condition::SourceMatches(&Filter::Not(&Filter::IsToken)) }"]
    );

    let drew_etb = read(
        "Name:DrewEtb\nManaCost:2 U\nTypes:Creature Wizard\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition\n\
         SVar:X:Count$YouDrewThisTurn",
    );
    assert_eq!(
        drew_etb.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::CardsDrawnThisTurn(PlayerRel::You) }"]
    );
}

#[test]
fn wave26f_moved_replacement_unlocks() {
    let minion = read(
        "Name:Minion Probe\nManaCost:3 B B B\nTypes:Creature Minion\nPT:1/1\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplaceWith$ PayLife | ReplacementResult$ Updated\n\
         SVar:PayLife:AB$ StoreSVar | Cost$ Mandatory PayLife<X> | SVar$ LifePaidOnETB | \
         Type$ Calculate | Expression$ X\n\
         SVar:X:Count$xPaid\n\
         SVar:LifePaidOnETB:Number$0",
    );
    assert_eq!(
        minion.enter_modifiers,
        ["EnterModifier::PayLifeUpTo(Amount::LifeTotal(PlayerRel::You))"]
    );

    let bounded_pay = read(
        "Name:BoundedPayLife\nManaCost:3 B\nTypes:Creature\nPT:1/1\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplaceWith$ PayLife | ReplacementResult$ Updated\n\
         SVar:PayLife:AB$ StoreSVar | Cost$ Mandatory PayLife<X> | XMax$ Cap | \
         SVar$ LifePaidOnETB | Type$ Calculate | Expression$ X\n\
         SVar:X:Count$xPaid\n\
         SVar:Cap:Count$Valid Creature.OppCtrl\n\
         SVar:LifePaidOnETB:Number$0",
    );
    assert!(
        bounded_pay.enter_modifiers[0].starts_with("EnterModifier::PayLifeUpTo("),
        "mods: {:?}",
        bounded_pay.enter_modifiers
    );

    let suspected = read(
        "Name:SuspectedEntrant\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/1\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ EnterSus\n\
         SVar:EnterSus:DB$ AlterAttribute | Defined$ Self | Attributes$ Suspected | \
         Activate$ True | ETB$ True",
    );
    assert_eq!(suspected.enter_modifiers, ["EnterModifier::Suspected"]);

    let chained_ctr_tap = read(
        "Name:ChainedCounterTap\nManaCost:3\nTypes:Artifact Creature\nPT:0/0\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ AddCtr\n\
         SVar:AddCtr:DB$ PutCounter | Defined$ Self | ETB$ True | CounterType$ P1P1 | \
         CounterNum$ 2 | SubAbility$ DoTap\n\
         SVar:DoTap:DB$ Tap | Defined$ Self | ETB$ True",
    );
    assert!(
        chained_ctr_tap
            .enter_modifiers
            .contains(&"EnterModifier::Tapped".to_string())
            && chained_ctr_tap.enter_modifiers.contains(
                &"EnterModifier::WithCounters { kind: CounterKind::P1P1, \
                  amount: Amount::Fixed(2) }"
                    .to_string()
            ),
        "mods: {:?}",
        chained_ctr_tap.enter_modifiers
    );

    let metalcraft_tap = read(
        "Name:MetalcraftTapLand\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ ETBTap\n\
         SVar:ETBTap:DB$ Tap | Defined$ Self | ETB$ True | Condition$ Metalcraft",
    );
    assert_eq!(
        metalcraft_tap.enter_modifiers,
        ["EnterModifier::TappedUnlessCondition(Condition::Not(&CHECK1))"]
    );

    let other_tapped = read(
        "Name:OtherOppTapped\nManaCost:2 W\nTypes:Creature Knight\nPT:2/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Creature.Other+OppCtrl | \
         Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBTapped\n\
         SVar:ETBTapped:DB$ Tap | ETB$ True | Defined$ ReplacedCard",
    );
    assert_eq!(
        other_tapped.abilities,
        ["AbilityDef::Replacement(ReplacementRule::EnterTapped(&ENTER1))"]
    );
    assert!(other_tapped.statics.contains("Filter::Another"));

    let other_untapped = read(
        "Name:OtherYourUntapped\nManaCost:2 G\nTypes:Creature Elf\nPT:2/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Land.Other+YouCtrl | \
         Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBUntapped\n\
         SVar:ETBUntapped:DB$ Untap | ETB$ True | Defined$ ReplacedCard",
    );
    assert_eq!(
        other_untapped.abilities,
        ["AbilityDef::Replacement(ReplacementRule::EnterUntapped(&ENTER1))"]
    );

    // Opponent-scoped external EnterUntapped must stay refused.
    assert!(refused(
        "Name:BadOtherOppUntapped\nManaCost:2 G\nTypes:Creature Elf\nPT:2/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Land.Other+OppCtrl | \
         Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBUntapped\n\
         SVar:ETBUntapped:DB$ Untap | ETB$ True | Defined$ ReplacedCard"
    ));
}

#[test]
fn wave26f_token_found_goad_and_peer_refusals() {
    let harness = read_with_tokens(
        "Name:Fractal Harness Probe\nManaCost:2 G\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | \
         Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | \
         SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Remembered | CounterType$ P1P1 | \
         CounterNum$ 2 | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        harness.abilities[0].contains("Effect::RepeatEachObject")
            && harness.abilities[0].contains("only_found: true")
            && harness.abilities[0].contains("Effect::AddCounterFilter")
            && harness.abilities[0]
                .contains("Effect::AttachSelf { target: TargetSpec::EventObject }"),
        "ability: {}",
        harness.abilities[0]
    );

    let goad_token = read_with_tokens(
        "Name:GoadTokenTrigger\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ Opponent | \
         RememberTokens$ True | SubAbility$ DBGoad\n\
         SVar:DBGoad:DB$ Goad | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        goad_token.abilities[0].contains("only_found: true")
            && goad_token.abilities[0].contains("Effect::Goad { target: TargetSpec::EventObject }"),
        "ability: {}",
        goad_token.abilities[0]
    );

    let untap_token = read_with_tokens(
        "Name:UntapTokenTrigger\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | TokenTapped$ True | \
         RememberTokens$ True | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        untap_token.abilities[0].contains("Effect::UntapFound"),
        "ability: {}",
        untap_token.abilities[0]
    );

    let goad_attacker = read(
        "Name:GoadAttackerTrigger\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | ValidAttackers$ Creature.OppCtrl | \
         AttackedTarget$ You | TriggerZones$ Battlefield | Execute$ TrigGoad\n\
         SVar:TrigGoad:DB$ Goad | Defined$ TriggeredAttackers",
    );
    assert!(
        goad_attacker.abilities[0].contains("Effect::Goad { target: TargetSpec::EventObject }"),
        "ability: {}",
        goad_attacker.abilities[0]
    );

    // Soundness refusals: permanent goad, missing Cleanup, ChoiceRestriction (w26c handoff),
    // Alchemy, and Un-Set/Planechase must stay refused.
    assert!(refused_with_tokens(
        "Name:PermanentGoadToken\nManaCost:3 R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | \
         SubAbility$ DBGoad\n\
         SVar:DBGoad:DB$ Goad | Defined$ Remembered | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True"
    ));
    assert!(refused_with_tokens(
        "Name:UnclosedRememberTokens\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | \
         SubAbility$ DBGoad\n\
         SVar:DBGoad:DB$ Goad | Defined$ Remembered"
    ));
    assert!(refused(
        "Name:RestrictedModalTrigger\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | \
         Execute$ TrigCharm\n\
         SVar:TrigCharm:DB$ Charm | ChoiceRestriction$ OncePerGame | Choices$ ModeA,ModeB\n\
         SVar:ModeA:DB$ Draw | NumCards$ 1\n\
         SVar:ModeB:DB$ GainLife | LifeAmount$ 2"
    ));
    assert!(refused(
        "Name:AlchemySeekPhase\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigSeek\n\
         SVar:TrigSeek:DB$ Seek | Type$ Instant"
    ));
    assert!(refused(
        "Name:PlanechaseTrigger\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ PlaneswalkedTo | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
}
