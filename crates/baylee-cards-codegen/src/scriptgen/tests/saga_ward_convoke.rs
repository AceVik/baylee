//! Synthetic unit tests for `saga_ward_convoke.rs` keyword, entry-counter,
//! static, and replacement lowering rules and their refusal boundaries.

use super::*;

#[test]
fn suspend_lowers_across_card_types_and_refuses_invalid_counts_or_costs() {
    let sorcery = read(
        "Name:SuspendedDraw\nManaCost:no cost\nTypes:Sorcery\n\
         K:Suspend:4:1 U\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert!(
        sorcery
            .abilities
            .iter()
            .any(|a| a == r#"AbilityDef::Suspend { counters: 4, cost: mana!("{1}{U}") }"#),
        "abilities: {:?}",
        sorcery.abilities
    );
    assert!(sorcery.notes.iter().any(|n| n == "suspend"));

    let zero_cost = read(
        "Name:SuspendedRelic\nManaCost:no cost\nTypes:Artifact\n\
         K:Suspend:3:0",
    );
    assert_eq!(
        zero_cost.abilities,
        [r#"AbilityDef::Suspend { counters: 3, cost: mana!("{0}") }"#]
    );

    let hybrid = read(
        "Name:SuspendedCharm\nManaCost:2 W\nTypes:Enchantment\n\
         K:Suspend:2:WU",
    );
    assert_eq!(
        hybrid.abilities,
        [r#"AbilityDef::Suspend { counters: 2, cost: mana!("{W/U}") }"#]
    );

    // Creatures, instants, and flash permanents all lower cleanly (CR 702.62a).
    let beast = read("Name:SuspendedBeast\nManaCost:4 G\nTypes:Creature\nPT:4/4\nK:Suspend:3:1 G");
    assert_eq!(
        beast.abilities,
        [r#"AbilityDef::Suspend { counters: 3, cost: mana!("{1}{G}") }"#]
    );
    let instant = read(
        "Name:SuspendedInstant\nManaCost:1 U\nTypes:Instant\n\
         K:Suspend:2:U\nA:SP$ Draw | NumCards$ 1",
    );
    assert!(
        instant
            .abilities
            .iter()
            .any(|a| a == r#"AbilityDef::Suspend { counters: 2, cost: mana!("{U}") }"#)
    );
    let flash_relic =
        read("Name:SuspendedFlashRelic\nManaCost:2\nTypes:Artifact\nK:Flash\nK:Suspend:2:1");
    assert_eq!(
        flash_relic.abilities,
        [r#"AbilityDef::Suspend { counters: 2, cost: mana!("{1}") }"#]
    );
    // Variable X, zero counters, non-mana cost, or missing type line stay refused.
    assert!(refused(
        "Name:SuspendedX\nManaCost:no cost\nTypes:Sorcery\nK:Suspend:X:1 U\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:SuspendedZero\nManaCost:1 U\nTypes:Sorcery\nK:Suspend:0:U\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:SuspendedXCost\nManaCost:1 U\nTypes:Sorcery\nK:Suspend:2:X U\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:SuspendedLife\nManaCost:1 B\nTypes:Sorcery\nK:Suspend:2:PayLife<2>\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused("Name:NoTypes\nK:Suspend:2:1 U"));
}

#[test]
fn ward_non_numeric_forms_lower_and_refuse_unsupported_costs() {
    // Plain integers keep routing to `AbilityDef::Ward { mana }`.
    let plain = read("Name:WardPlain\nTypes:Creature\nPT:2/2\nK:Ward:3");
    assert_eq!(plain.abilities, ["AbilityDef::Ward { mana: 3 }"]);
    assert!(refused("Name:WardZero\nTypes:Creature\nPT:2/2\nK:Ward:0"));
    assert!(refused(
        "Name:WardEleven\nTypes:Creature\nPT:2/2\nK:Ward:11"
    ));

    let life = read("Name:WardLife\nTypes:Creature\nPT:2/2\nK:Ward:PayLife<3>");
    assert_eq!(
        life.abilities,
        ["triggered!(Trigger::Ward, &[Effect::PlayerMayPayLifeOr { \
             player: PlayerRel::ControllerOfTarget, life: Amount::Fixed(3), \
             effect: &Effect::CounterTargetSpellOrAbility }])"]
    );

    let power_life = read(
        "Name:WardPower\nTypes:Creature\nPT:5/4\n\
         K:Ward:PayLife<X/life equal to this creature's power>\n\
         SVar:X:Count$CardPower",
    );
    assert_eq!(
        power_life.abilities,
        ["triggered!(Trigger::Ward, &[Effect::PlayerMayPayLifeOr { \
             player: PlayerRel::ControllerOfTarget, life: Amount::SourcePower, \
             effect: &Effect::CounterTargetSpellOrAbility }])"]
    );

    let poison = read(
        "Name:WardPoison\nTypes:Creature\nPT:2/2\n\
         K:Ward:AddCounterYou<2/POISON/get two poison counters>",
    );
    assert_eq!(
        poison.abilities,
        [
            "triggered!(Trigger::Ward, &[Effect::PlayerMayGetPoisonOr { \
             player: PlayerRel::ControllerOfTarget, poison: Amount::Fixed(2), \
             effect: &Effect::CounterTargetSpellOrAbility }])"
        ]
    );

    let discard = read("Name:WardDiscard\nTypes:Creature\nPT:2/2\nK:Ward:Discard<1/Card>");
    assert!(discard.statics.is_empty(), "statics: {}", discard.statics);
    assert_eq!(
        discard.abilities,
        ["triggered!(Trigger::Ward, &[Effect::PlayerMayPayCostOr { \
             player: PlayerRel::ControllerOfTarget, cost: &CostPart::Discard(&Filter::Any), \
             effect: &Effect::CounterTargetSpellOrAbility }])"]
    );

    let discard_or =
        read("Name:WardDiscardOr\nTypes:Creature\nPT:2/2\nK:Ward:Discard<1/Artifact;Creature>");
    assert!(discard_or.statics.is_empty());
    assert_eq!(
        discard_or.abilities,
        ["triggered!(Trigger::Ward, &[Effect::PlayerMayPayCostOr { \
             player: PlayerRel::ControllerOfTarget, \
             cost: &CostPart::Discard(&Filter::ARTIFACT_OR_CREATURE), \
             effect: &Effect::CounterTargetSpellOrAbility }])"]
    );

    let sac_one = read("Name:WardSacOne\nTypes:Creature\nPT:3/3\nK:Ward:Sac<1/Creature>");
    assert!(sac_one.statics.is_empty(), "statics: {}", sac_one.statics);
    assert_eq!(
        sac_one.abilities,
        ["triggered!(Trigger::Ward, &[Effect::PlayerMayPayCostOr { \
             player: PlayerRel::ControllerOfTarget, cost: &CostPart::Sacrifice(&Filter::CREATURE), \
             effect: &Effect::CounterTargetSpellOrAbility }])"]
    );

    let sac_two =
        read("Name:WardSacTwo\nTypes:Creature\nPT:4/4\nK:Ward:Sac<2/Artifact;Enchantment>");
    assert!(sac_two.statics.is_empty());
    assert_eq!(
        sac_two.abilities,
        [
            "triggered!(Trigger::Ward, &[Effect::PlayerMaySacrificeNOr { \
             player: PlayerRel::ControllerOfTarget, filter: &Filter::ARTIFACT_OR_ENCHANTMENT, \
             count: 2, effect: &Effect::CounterTargetSpellOrAbility }])"
        ]
    );

    let colored = read("Name:WardMana\nTypes:Creature\nPT:2/2\nK:Ward:1 B");
    assert_eq!(
        colored.abilities,
        [
            r#"triggered!(Trigger::Ward, &[Effect::PlayerMayPayManaOr { player: PlayerRel::ControllerOfTarget, cost: mana!("{1}{B}"), effect: &Effect::CounterTargetSpellOrAbility }])"#
        ]
    );

    let hybrid = read("Name:WardHybrid\nTypes:Creature\nPT:2/2\nK:Ward:WU");
    assert_eq!(
        hybrid.abilities,
        [
            r#"triggered!(Trigger::Ward, &[Effect::PlayerMayPayManaOr { player: PlayerRel::ControllerOfTarget, cost: mana!("{W/U}"), effect: &Effect::CounterTargetSpellOrAbility }])"#
        ]
    );
}

#[test]
fn ward_refuses_unsupported_and_composite_costs() {
    for bad in [
        "K:Ward:PayLife<0>",
        "K:Ward:PayLife<X>\nSVar:X:Count$CardToughness",
        "K:Ward:AddCounterYou<1/ENERGY>",
        "K:Ward:AddCounterYou<0/POISON>",
        "K:Ward:Discard<2/Card>",
        "K:Ward:Discard<1/Random>",
        "K:Ward:Discard<1/Card.YouCtrl>",
        "K:Ward:Sac<0/Creature>",
        "K:Ward:Sac<1/CARDNAME>",
        "K:Ward:Sac<1/Creature.YouCtrl>",
        "K:Ward:2 PayLife<2>",
        "K:Ward:Discard<1/Card>:2",
        "K:Ward:Waterbend<3>",
    ] {
        assert!(
            refused(&format!("Name:BadWard\nTypes:Creature\nPT:2/2\n{bad}")),
            "expected refusal for `{bad}`"
        );
    }
}

#[test]
fn partner_with_and_commander_deckbuilding_keywords() {
    let pw = read(
        "Name:Alpha Partner\nTypes:Legendary Creature\nPT:2/2\n\
         K:Partner with:Beta Partner",
    );
    assert!(!pw.has_partner);
    assert!(
        pw.statics
            .contains(r#"static PARTNER1: Filter = Filter::Named("Beta Partner");"#),
        "statics: {}",
        pw.statics
    );
    assert_eq!(
        pw.abilities,
        ["triggered!(Trigger::ETB, &[Effect::SearchLibraryOf { \
             library: PlayerRel::Chosen, owner_searches: true, filter: &PARTNER1, \
             mana_value: None, finds: &[Find::HAND], optional: true }], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"]
    );

    let pw_short = read(
        "Name:Alpha Partner\nTypes:Legendary Creature\nPT:2/2\n\
         K:Partner with:Beta Partner:Beta",
    );
    assert_eq!(pw_short.abilities, pw.abilities);
    assert_eq!(pw_short.statics, pw.statics);

    assert!(refused(
        "Name:SpellPartner\nTypes:Sorcery\nK:Partner with:Beta\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:EmptyPartner\nTypes:Legendary Creature\nPT:2/2\nK:Partner with:"
    ));
    assert!(refused(
        "Name:SplitPartner\nTypes:Legendary Creature\nPT:2/2\nK:Partner with:Left // Right"
    ));

    for (kw, note) in [
        ("K:Choose a Background", "choose a background"),
        ("K:Doctor's companion", "doctor's companion"),
        ("K:Partner:Friends forever", "friends forever"),
        ("K:CARDNAME can be your commander.", "can be your commander"),
    ] {
        let body = read(&format!(
            "Name:Cmdr\nTypes:Legendary Creature\nPT:2/2\nK:Flying\n{kw}"
        ));
        assert!(!body.has_partner, "`{kw}` must not set bare `has_partner`");
        assert!(
            body.notes.iter().any(|n| n == note),
            "missing note `{note}`"
        );
    }

    assert!(refused(
        "Name:CharSel\nTypes:Legendary Creature\nPT:2/2\nK:Partner:Character select"
    ));
    assert!(refused(
        "Name:Surv\nTypes:Legendary Creature\nPT:2/2\nK:Partner:Survivors"
    ));
}

#[test]
fn storied_ascend_flanking_and_combat_keyword_sentences() {
    let storied = read("Name:StoriedHero\nTypes:Creature\nPT:2/2\nK:Storied");
    assert_eq!(storied.keywords, ["KeywordSet::STORIED"]);
    assert!(refused(
        "Name:StoriedSpell\nTypes:Instant\nK:Storied\nA:SP$ Draw | NumCards$ 1"
    ));

    let ascend = read("Name:AscendPerm\nTypes:Creature\nPT:2/2\nK:Ascend");
    assert_eq!(ascend.keywords, ["KeywordSet::ASCEND"]);
    assert!(refused(
        "Name:AscendInstant\nTypes:Instant\nK:Ascend\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:AscendSorcery\nTypes:Sorcery\nK:Ascend\nA:SP$ Draw | NumCards$ 1"
    ));

    let double_flank = read("Name:DoubleFlank\nTypes:Creature\nPT:2/2\nK:Flanking\nK:Flanking");
    assert_eq!(double_flank.keywords, ["KeywordSet::FLANKING"]);
    assert_eq!(
        double_flank.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::FLANKING))"]
    );

    let immobile = read("Name:Statue\nTypes:Creature\nPT:0/5\nK:CARDNAME can't attack or block.");
    assert_eq!(
        immobile.keywords,
        ["KeywordSet::CANT_ATTACK", "KeywordSet::CANT_BLOCK"]
    );

    let lone = read("Name:Loner\nTypes:Creature\nPT:4/4\nK:CARDNAME can only attack alone.");
    assert_eq!(
        lone.abilities,
        ["static_ability!(Filter::This, Modifier::AttacksAloneOnly)"]
    );
}

#[test]
fn etb_counter_extended_amounts_and_refusals() {
    let mana_spent = read(
        "Name:ManaEntry\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:Count$CastTotalManaSpent",
    );
    assert_eq!(
        mana_spent.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::ManaSpentToCast }"
        ]
    );

    let life_total = read(
        "Name:LifeEntry\nManaCost:4 W\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$YourLifeTotal",
    );
    assert_eq!(
        life_total.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::LifeTotal(PlayerRel::You) }"]
    );

    let hand = read(
        "Name:HandEntry\nManaCost:3 U\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidHand Card.YouOwn",
    );
    assert_eq!(
        hand.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::HandSize(PlayerRel::You) }"]
    );

    let double_x = read(
        "Name:DoubleXEntry\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:Y\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:SVar$X/Twice",
    );
    assert_eq!(
        double_x.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::DoubleX }"]
    );
    assert!(refused(
        "Name:NoXDouble\nManaCost:2 G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$xPaid/Twice"
    ));

    let valid_plus = read(
        "Name:CountPlus\nManaCost:3\nTypes:Artifact Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:Count$Valid Artifact.YouCtrl/Plus.2",
    );
    assert!(valid_plus.statics.is_empty());
    assert_eq!(
        valid_plus.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Plus { base: &Amount::CountOf { filter: &Filter::YOUR_ARTIFACT, \
             zone: ZoneSel::Battlefield }, offset: 2 } }"]
    );

    let last_times = read(
        "Name:LastTimes\nManaCost:3 G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$LastStateBattlefield Land.YouCtrl/Times.2",
    );
    assert_eq!(
        last_times.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Times { base: &Amount::CountOf { filter: &Filter::YOUR_LAND, \
             zone: ZoneSel::Battlefield }, factor: 2 } }"]
    );
}

#[test]
fn etb_counter_graveyard_each_and_refusals() {
    let gy_all = read(
        "Name:GyAll\nManaCost:3 U\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidGraveyard Instant,Sorcery",
    );
    assert!(gy_all.statics.is_empty());
    assert_eq!(
        gy_all.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::CountOf { filter: &Filter::INSTANT_OR_SORCERY, zone: ZoneSel::GraveyardAll } }"]
    );

    let gy_you = read(
        "Name:GyYou\nManaCost:3 B\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    assert_eq!(
        gy_you.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::CountOf { filter: &Filter::CREATURE, zone: ZoneSel::GraveyardYou } }"]
    );

    let gy_opp = read(
        "Name:GyOpp\nManaCost:3 B\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidGraveyard Card.OppOwn",
    );
    assert_eq!(
        gy_opp.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOpponents } }"]
    );

    let each = read(
        "Name:EachEntry\nManaCost:3 W\nTypes:Creature\nPT:2/2\n\
         K:etbCounter:EACH P1P1,Flying:1:no Condition:desc",
    );
    assert_eq!(
        each.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }",
            "EnterModifier::WithCounters { kind: CounterKind::Flying, amount: Amount::Fixed(1) }",
        ]
    );

    // Refusals: conditions, property queries, mixed graveyard owners, unknown EACH.
    assert!(refused(
        "Name:CondEtb\nTypes:Creature\nPT:1/1\n\
         K:etbCounter:P1P1:X:CheckSVar$ Y:desc\n\
         SVar:X:Count$YourLifeTotal\nSVar:Y:Count$Valid Creature.YouCtrl"
    ));
    assert!(refused(
        "Name:PropEtb\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardToughness"
    ));
    assert!(refused(
        "Name:MixedGy\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn,Creature.OppOwn"
    ));
    assert!(refused(
        "Name:ZeroEach\nTypes:Creature\nPT:2/2\nK:etbCounter:EACH P1P1,Flying:0"
    ));
    assert!(refused(
        "Name:UnknownEach\nTypes:Creature\nPT:2/2\nK:etbCounter:EACH P1P1,BOGUS:1"
    ));
}

#[test]
fn extra_statics_cant_attack_block_flash_panharmonicon_and_suppress() {
    let immobile = read(
        "Name:ImmobileStatic\nTypes:Creature\nPT:4/4\n\
         S:Mode$ CantAttack,CantBlock | ValidCard$ Card.Self | IsPresent$ Artifact.YouCtrl | PresentCompare$ EQ0",
    );
    assert_eq!(
        immobile.abilities,
        ["static_ability!(Filter::This, \
             Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)), \
             condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_ARTIFACT, 0)))"]
    );

    let flash_all = read(
        "Name:FlashAll\nTypes:Enchantment\n\
         S:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell | Caster$ You",
    );
    assert_eq!(
        flash_all.abilities,
        ["static_ability!(Filter::Any, Modifier::SpellsHaveFlash(None))"]
    );

    let flash_sorcery = read(
        "Name:FlashSorcery\nTypes:Creature\nPT:4/4\n\
         S:Mode$ CastWithFlash | ValidCard$ Sorcery | ValidSA$ Spell | Caster$ You",
    );
    assert_eq!(
        flash_sorcery.abilities,
        ["static_ability!(Filter::Any, Modifier::SorceriesHaveFlash)"]
    );

    let flash_types = read(
        "Name:FlashTypes\nTypes:Artifact\n\
         S:Mode$ CastWithFlash | ValidCard$ Artifact,Creature | ValidSA$ Spell | Caster$ You",
    );
    assert_eq!(
        flash_types.abilities,
        ["static_ability!(Filter::Any, \
             Modifier::SpellsHaveFlash(Some(TypeSet::ARTIFACT.union(TypeSet::CREATURE))))"]
    );

    for bad in [
        "S:Mode$ CastWithFlash | ValidSA$ Activated.Equip | Caster$ You",
        "S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You",
    ] {
        assert!(
            refused(&format!("Name:BadFlash\nTypes:Enchantment\n{bad}")),
            "expected refusal for `{bad}`"
        );
    }
}

#[test]
fn extra_statics_panharmonicon_and_disable_triggers() {
    let mult_any = read(
        "Name:MultAny\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Panharmonicon | ValidCard$ Creature.YouCtrl+Legendary",
    );
    assert!(
        mult_any.statics.contains(
            "static TRIGGER_MULT1: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::HasSupertype(SupertypeSet::LEGENDARY)]);"
        ),
        "statics: {}",
        mult_any.statics
    );
    assert_eq!(
        mult_any.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::TriggerMultiplier { \
             source_filter: &TRIGGER_MULT1, event: TriggerEventKind::Any })"
        ]
    );

    let mult_etb = read(
        "Name:MultEtb\nTypes:Creature\nPT:3/5\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | \
         ValidCard$ Permanent.YouCtrl | ValidCause$ Permanent | Destination$ Battlefield",
    );
    assert_eq!(
        mult_etb.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::TriggerMultiplier { \
             source_filter: &Filter::ControlledByYou, event: TriggerEventKind::EntersBattlefield })"
        ]
    );

    assert!(refused(
        "Name:NarrowCause\nTypes:Artifact\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | \
         ValidCard$ Permanent.YouCtrl | ValidCause$ Artifact,Creature | Destination$ Battlefield"
    ));
    assert!(refused(
        "Name:DrawMult\nTypes:Artifact\n\
         S:Mode$ Panharmonicon | ValidMode$ Drawn | ValidCard$ Permanent.YouCtrl"
    ));

    let suppress = read(
        "Name:SuppressEtb\nTypes:Creature\nPT:4/7\n\
         S:Mode$ DisableTriggers | ValidCause$ Permanent | \
         ValidMode$ ChangesZone,ChangesZoneAll | Destination$ Battlefield | \
         ValidCard$ Permanent.OppCtrl+inZoneBattlefield",
    );
    assert!(suppress.statics.is_empty());
    assert_eq!(
        suppress.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::TriggerSuppress { \
             source_filter: &Filter::ControlledByOpponent, event: TriggerEventKind::EntersBattlefield })"
        ]
    );
    assert!(refused(
        "Name:NarrowSuppress\nTypes:Artifact\n\
         S:Mode$ DisableTriggers | ValidCause$ Creature | \
         ValidMode$ ChangesZone,ChangesZoneAll | Destination$ Battlefield"
    ));
}

#[test]
fn extra_replacements_create_token_and_add_counter() {
    let double_tokens = read(
        "Name:DoubleTokens\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Card.YouCtrl | \
         EffectOnly$ True | ReplaceWith$ DoubleTok\n\
         SVar:DoubleTok:DB$ ReplaceToken | Type$ Amount",
    );
    assert_eq!(
        double_tokens.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation { \
             controller_filter: &Filter::ControlledByYou })"
        ]
    );

    let triple_creatures = read(
        "Name:TripleCreatures\nTypes:Creature\nPT:6/6\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Creature.YouCtrl | \
         ReplaceWith$ TripleTok\n\
         SVar:TripleTok:DB$ ReplaceToken | Type$ Amount | Amount$ Thrice",
    );
    assert_eq!(
        triple_creatures.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::MultiplyTypedTokens { \
             controller_filter: &Filter::ControlledByYou, types: TypeSet::CREATURE, factor: 3 })"
        ]
    );

    assert!(refused(
        "Name:TripleAll\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Card.YouCtrl | \
         ReplaceWith$ TripleTok\n\
         SVar:TripleTok:DB$ ReplaceToken | Type$ Amount | Amount$ Thrice"
    ));
    assert!(refused(
        "Name:CommandTok\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Command | ValidToken$ Card.YouCtrl | \
         ReplaceWith$ DoubleTok\n\
         SVar:DoubleTok:DB$ ReplaceToken | Type$ Amount"
    ));
}

#[test]
fn extra_replacements_add_counter() {
    let double_by_you = read(
        "Name:DoubleByYou\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidSource$ You | \
         ValidObject$ Permanent.inZoneBattlefield,Player | ReplaceWith$ DoubleC\n\
         SVar:DoubleC:DB$ ReplaceCounter | ChooseCounter$ True | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Twice",
    );
    assert_eq!(
        double_by_you.abilities,
        ["static_ability!(Filter::Any, Modifier::DoubleCountersPlacedByYou)"]
    );

    let double_effect_only = read(
        "Name:DoubleEffectOnly\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Permanent.YouCtrl+inZoneBattlefield | EffectOnly$ True | ReplaceWith$ DoubleC\n\
         SVar:DoubleC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Twice",
    );
    assert!(double_effect_only.statics.is_empty());
    assert_eq!(
        double_effect_only.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement { \
             object_filter: &Filter::ControlledByYou })"
        ]
    );

    let plus_one = read(
        "Name:PlusOneCounter\nTypes:Creature\nPT:2/3\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Artifact.YouCtrl+inZoneBattlefield,Creature.YouCtrl+inZoneBattlefield | \
         ReplaceWith$ PlusC\n\
         SVar:PlusC:DB$ ReplaceCounter | ChooseCounter$ True | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Plus.1",
    );
    assert_eq!(
        plus_one.abilities,
        [
            "static_ability!(Filter::Or(&[Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou]), \
             Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])]), \
             Modifier::AdditionalCounterPlacement { n: 1 })"
        ]
    );

    for (script, expected) in [
        (
            "Name:TypedCounter\nTypes:Enchantment\n\
             R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCounterType$ P1P1 | \
             ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ PlusC\n\
             SVar:PlusC:DB$ ReplaceCounter | Amount$ X\n\
             SVar:X:ReplaceCount$CounterNum/Plus.1",
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::YOUR_CREATURE, kind: Some(CounterKind::P1P1), placer: None, \
             effect_only: false, op: CounterReplaceOp::Add(1) })",
        ),
        (
            "Name:UncondDoubler\nTypes:Enchantment\n\
             R:Event$ AddCounter | ActiveZones$ Battlefield | \
             ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ DoubleC\n\
             SVar:DoubleC:DB$ ReplaceCounter | Amount$ X\n\
             SVar:X:ReplaceCount$CounterNum/Twice",
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::YOUR_CREATURE, kind: None, placer: None, \
             effect_only: false, op: CounterReplaceOp::Multiply(2) })",
        ),
        (
            "Name:EffectOnlyPlus\nTypes:Enchantment\n\
             R:Event$ AddCounter | ActiveZones$ Battlefield | EffectOnly$ True | \
             ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ PlusC\n\
             SVar:PlusC:DB$ ReplaceCounter | Amount$ X\n\
             SVar:X:ReplaceCount$CounterNum/Plus.1",
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::YOUR_CREATURE, kind: None, placer: None, \
             effect_only: true, op: CounterReplaceOp::Add(1) })",
        ),
    ] {
        assert_eq!(read(script).abilities, [expected]);
    }

    // Refusals: missing battlefield zone, ValidCause$, and literal constant Amount$ 1.
    for bad in [
        "Name:MissingZone\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Creature.YouCtrl | EffectOnly$ True | ReplaceWith$ DoubleC\n\
         SVar:DoubleC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Twice",
        "Name:CauseScoped\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCause$ Spell | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ PlusC\n\
         SVar:PlusC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Plus.1",
        "Name:LiteralConst\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ ConstC\n\
         SVar:ConstC:DB$ ReplaceCounter | Amount$ 1",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn provoke_single_and_duplicate_instances() {
    let single = read("Name:SingleProvoke\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Provoke");
    assert_eq!(single.keywords, ["KeywordSet::PROVOKE"]);
    assert!(single.abilities.is_empty());

    let double = read(
        "Name:DoubleProvoke\nManaCost:3 R\nTypes:Creature\nPT:3/2\n\
         K:Provoke\nK:Provoke",
    );
    assert_eq!(double.keywords, ["KeywordSet::PROVOKE"]);
    assert_eq!(
        double.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::PROVOKE))"]
    );
}

#[test]
fn etb_counter_dynamic_suffixes_and_dual_kicker() {
    let multikick = read(
        "Name:KickedCounter\nManaCost:2 G\nTypes:Creature\nPT:0/0\n\
         K:Multikicker:1 G\n\
         K:etbCounter:P1P1:X:no Condition:desc\n\
         SVar:X:Count$TimesKicked/Times.2",
    );
    assert_eq!(
        multikick.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 2 } }"]
    );

    let died = read(
        "Name:MorbidEntry\nManaCost:3 B\nTypes:Creature\nPT:1/1\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature/Plus.1",
    );
    assert_eq!(
        died.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Plus { base: &Amount::CreaturesDiedThisTurn, offset: 1 } }"]
    );

    let x_half = read(
        "Name:HalfXEntry\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$xPaid/HalfDown",
    );
    assert_eq!(
        x_half.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Half { base: &Amount::X, round_up: false } }"]
    );

    let x_sub = read(
        "Name:SubXEntry\nManaCost:X U\nTypes:Artifact\n\
         K:etbCounter:CHARGE:Y\n\
         SVar:X:Count$xPaid\nSVar:Y:SVar$X/Minus.1",
    );
    assert_eq!(
        x_sub.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::Charge, \
             amount: Amount::SaturatingSub { base: &Amount::X, subtract: 1 } }"]
    );

    let dual = read(
        "Name:DualKickEntry\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         K:Kicker:1 B:1 R\n\
         K:etbCounter:P1P1:2:CheckSVar$ K1:desc\n\
         K:etbCounter:P1P1:1:CheckSVar$ K2:desc\n\
         SVar:K1:Count$Kicked1.1.0\n\
         SVar:K2:Count$Kicked2.1.0\n\
         Oracle:Kicker {1}{R} and/or {1}{B}",
    );
    assert!(
        dual.enter_modifiers.contains(
            &"EnterModifier::IfKicked { index: Some(1), counters: 2, modifiers: &[] }".to_string()
        ),
        "mods: {:?}",
        dual.enter_modifiers
    );
    assert!(
        dual.enter_modifiers.contains(
            &"EnterModifier::IfKicked { index: Some(0), counters: 1, modifiers: &[] }".to_string()
        ),
        "mods: {:?}",
        dual.enter_modifiers
    );
}

#[test]
fn game_loss_win_players_zero_life_and_conditions() {
    let everyone = read(
        "Name:NoOneLoses\nTypes:Artifact\n\
         R:Event$ GameLoss | ActiveZones$ Battlefield | ValidPlayer$ Player | Layer$ CantHappen\n\
         R:Event$ GameWin | ActiveZones$ Battlefield | ValidPlayer$ Player | Layer$ CantHappen",
    );
    assert_eq!(
        everyone.abilities,
        [
            "static_ability!(Filter::Any, Modifier::PlayersCantLose)",
            "static_ability!(Filter::Any, Modifier::PlayersCantWin(PlayerRel::EachPlayer))",
        ]
    );

    let zero_life = read(
        "Name:ZeroLifeGuard\nTypes:Enchantment\n\
         R:Event$ GameLoss | ActiveZones$ Battlefield | ValidPlayer$ You | \
         ValidLoseReason$ LifeReachedZero | Layer$ CantHappen | IsPresent$ Artifact.YouCtrl",
    );
    assert_eq!(
        zero_life.abilities,
        [
            "static_ability!(Filter::Any, Modifier::NoLossForZeroLife { who: PlayerRel::You }, \
             condition = Some(Condition::ControlCount(&Filter::YOUR_ARTIFACT, 1)))"
        ]
    );

    assert!(refused(
        "Name:NoMillLoss\nTypes:Enchantment\n\
         R:Event$ GameLoss | ActiveZones$ Battlefield | ValidPlayer$ You | \
         ValidLoseReason$ Milled | Layer$ CantHappen"
    ));
}

#[test]
fn moved_self_etb_tapped_chains_reveal_and_life_loss() {
    let chained = read(
        "Name:TappedChargeLand\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ ETBTapped\n\
         SVar:ETBTapped:DB$ Tap | Defined$ Self | ETB$ True | SubAbility$ DBCharge\n\
         SVar:DBCharge:DB$ PutCounter | Defined$ Self | ETB$ True | \
         CounterType$ CHARGE | CounterNum$ 2",
    );
    assert_eq!(
        chained.enter_modifiers,
        [
            "EnterModifier::Tapped",
            "EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(2) }",
        ]
    );

    let reveal = read(
        "Name:RevealLand\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ ETBTapped\n\
         SVar:ETBTapped:DB$ Tap | Defined$ Self | ETB$ True | \
         UnlessCost$ Reveal<1/Forest;Island/a Forest or Island> | UnlessPayer$ You",
    );
    assert_eq!(
        reveal.enter_modifiers,
        ["EnterModifier::TappedUnlessReveal(&REVEAL1)"]
    );
    assert!(
        reveal.statics.contains("subtypes::land::FOREST")
            && reveal.statics.contains("subtypes::land::ISLAND"),
        "statics: {}",
        reveal.statics
    );

    let reveal_or_ctrl = read(
        "Name:RevealOrControlLand\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ DBReveal\n\
         SVar:DBReveal:DB$ Reveal | Optional$ True | RevealValid$ Goblin | \
         RememberRevealed$ True | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Self | ETB$ True | \
         ConditionCheckSVar$ Total | ConditionSVarCompare$ EQ0 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Total:Count$RememberedSize/Plus.Ctrl\n\
         SVar:Ctrl:Count$Valid Goblin.inZoneBattlefield+YouCtrl",
    );
    assert_eq!(
        reveal_or_ctrl.enter_modifiers,
        ["EnterModifier::TappedUnlessRevealOrControl(&REVEAL1)"]
    );
    assert!(
        reveal_or_ctrl
            .statics
            .contains("subtypes::creature::GOBLIN")
    );

    let life_loss = read(
        "Name:HalfLifeArtifact\nTypes:Artifact\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | \
         ReplacementResult$ Updated | ReplaceWith$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$YourLifeTotal",
    );
    assert_eq!(
        life_loss.enter_modifiers,
        ["EnterModifier::LoseLifeEqualToLife"]
    );
}

#[test]
fn moved_external_enter_cant_enter_and_exile_replacements() {
    let tap_opp = read(
        "Name:OppCreaturesTapped\nTypes:Enchantment\n\
         R:Event$ Moved | ValidCard$ Creature.OppCtrl | Destination$ Battlefield | \
         ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBTapped\n\
         SVar:ETBTapped:DB$ Tap | Defined$ ReplacedCard | ETB$ True",
    );
    assert_eq!(
        tap_opp.abilities,
        ["AbilityDef::Replacement(ReplacementRule::EnterTapped(&Filter::OPPONENT_CREATURE))"]
    );

    let untap_yours = read(
        "Name:YourLandsUntapped\nTypes:Enchantment\n\
         R:Event$ Moved | ValidCard$ Land.YouCtrl | Destination$ Battlefield | \
         ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBUntapped\n\
         SVar:ETBUntapped:DB$ Untap | Defined$ ReplacedCard | ETB$ True",
    );
    assert_eq!(
        untap_yours.abilities,
        ["AbilityDef::Replacement(ReplacementRule::EnterUntapped(&Filter::YOUR_LAND))"]
    );

    let cant_enter = read(
        "Name:NoLandEntry\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Battlefield | \
         ValidCard$ Land | Prevent$ True | Layer$ CantHappen",
    );
    assert_eq!(
        cant_enter.abilities,
        ["static_ability!(Filter::This, Modifier::CantEnterBattlefield(&Filter::LAND))"]
    );

    let exile_leave = read(
        "Name:ExileOnLeave\nTypes:Creature\nPT:2/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.Self | \
         Origin$ Battlefield | ExcludeDestination$ Exile | ReplaceWith$ ExileSelf\n\
         SVar:ExileSelf:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile",
    );
    assert_eq!(
        exile_leave.abilities,
        ["static_ability!(Filter::This, Modifier::ExileInsteadOfLeavingBattlefield)"]
    );

    let exile_opp_gy = read(
        "Name:VoidExiler\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.!token+OppOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileOpp\n\
         SVar:ExileOpp:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | \
         Destination$ Exile | WithCountersType$ VOID",
    );
    assert_eq!(
        exile_opp_gy.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileOpponentsGraveyard { \
             counter: Some(counters::VOID) })"
        ]
    );

    let exile_your_gy = read(
        "Name:SelfExiler\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.!token+YouOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileYou\n\
         SVar:ExileYou:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | Destination$ Exile",
    );
    assert_eq!(
        exile_your_gy.abilities,
        ["static_ability!(Filter::Any, Modifier::ExileInsteadOfYourGraveyard)"]
    );

    let token_gy_exile = read(
        "Name:TokenGyExile\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.YouOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileYou\n\
         SVar:ExileYou:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | Destination$ Exile",
    );
    assert_eq!(
        token_gy_exile.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::Any, origin: None, owner: Some(PlayerRel::You), \
             controller_lki: None, damaged_by: DamagedByScope::Any, \
             link_to_source: false, counter: None })"
        ]
    );

    // Refusals: Origin-scoped CantEnterBattlefield, opp-scoped EnterUntapped,
    // and unsupported GY-exile origin/active zone (`Origin$ Stack`, `ActiveZones$ Command`).
    for bad in [
        "Name:CageOnly\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Graveyard | \
         Destination$ Battlefield | ValidCard$ Creature | Prevent$ True | Layer$ CantHappen",
        "Name:OppUntap\nTypes:Enchantment\n\
         R:Event$ Moved | ValidCard$ Creature.OppCtrl | Destination$ Battlefield | \
         ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBUntapped\n\
         SVar:ETBUntapped:DB$ Untap | Defined$ ReplacedCard | ETB$ True",
        "Name:StackOriginGyExile\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Stack | ValidCard$ Card.YouOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileYou\n\
         SVar:ExileYou:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Stack | Destination$ Exile",
        "Name:CommandZoneGyExile\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Command | ValidCard$ Card.YouOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileYou\n\
         SVar:ExileYou:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | Destination$ Exile",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn extra_statics_life_lock_search_lock_and_life_gain_draw() {
    let body = read(
        "Name:LockAura\nTypes:Enchantment\n\
         S:Mode$ CantPreventDamage | IsCombat$ True\n\
         S:Mode$ CantLoseLife | ValidPlayer$ You\n\
         S:Mode$ CantChangeLife | ValidPlayer$ Player\n\
         S:Mode$ CantSearchLibrary | ValidPlayer$ Opponent\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ DrawInstead\n\
         SVar:DrawInstead:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:ReplaceCount$LifeGained",
    );
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::Any, Modifier::CombatDamageCantBePrevented)",
            "static_ability!(Filter::Any, Modifier::CantLoseLife { who: PlayerRel::You })",
            "static_ability!(Filter::Any, Modifier::LifeTotalCantChange { who: PlayerRel::EachPlayer })",
            "static_ability!(Filter::Any, Modifier::OpponentsCantSearch)",
            "static_ability!(Filter::Any, Modifier::LifeGainDrawsInstead { who: PlayerRel::You })",
        ]
    );

    assert!(refused(
        "Name:AllDmgNoPrevent\nTypes:Enchantment\nS:Mode$ CantPreventDamage"
    ));
    assert!(refused(
        "Name:AllCantSearch\nTypes:Enchantment\nS:Mode$ CantSearchLibrary | ValidPlayer$ Player"
    ));
}

#[test]
fn regeneration_turn_effects_defender_flash_life_damage_combat_and_counters() {
    let defender_self = read(
        "Name:WallAttacker\nTypes:Creature\nPT:0/5\n\
         A:AB$ Effect | Cost$ 1 W | StaticAbilities$ CanAtk\n\
         SVar:CanAtk:Mode$ CanAttackDefender | ValidCard$ Card.EffectSource",
    );
    assert_eq!(
        defender_self.abilities,
        [
            r#"activated!(cost!("{1}{W}"), &[Effect::ModifySource { modifier: Modifier::AttacksDespiteDefender, duration: Duration::UntilEndOfTurn }])"#
        ]
    );

    let defender_tgt = read(
        "Name:GrantWallAttack\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ CanAtk\n\
         SVar:CanAtk:Mode$ CanAttackDefender | ValidCard$ Card.IsRemembered",
    );
    assert_eq!(
        defender_tgt.abilities,
        [
            "spell!(&[Effect::ModifyTargets { modifier: Modifier::AttacksDespiteDefender, \
             duration: Duration::UntilEndOfTurn }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let flash_turn = read(
        "Name:TurnFlash\nManaCost:U\nTypes:Instant\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ FlashAll\n\
         SVar:FlashAll:Mode$ CastWithFlash | ValidCard$ Sorcery | ValidSA$ Spell | Caster$ You",
    );
    assert_eq!(
        flash_turn.abilities,
        [
            "spell!(&[Effect::continuous(&Filter::Any, Modifier::SorceriesHaveFlash, \
             Duration::UntilYourNextTurn)])"
        ]
    );

    let no_loss_no_prev = read(
        "Name:TurnLocks\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ NoLoss | SubAbility$ DBNoPrev\n\
         SVar:NoLoss:Mode$ CantLoseLife | ValidPlayer$ You\n\
         SVar:DBNoPrev:DB$ Effect | StaticAbilities$ NoPrev\n\
         SVar:NoPrev:Mode$ CantPreventDamage | IsCombat$ True",
    );
    assert_eq!(
        no_loss_no_prev.abilities,
        [
            "spell!(&[Effect::continuous(&Filter::Any, Modifier::CantLoseLife { who: PlayerRel::You }, Duration::UntilEndOfTurn), \
             Effect::continuous(&Filter::Any, Modifier::CombatDamageCantBePrevented, Duration::UntilEndOfTurn)])"
        ]
    );

    let cant_block_all = read(
        "Name:FalterSpell\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ NoBlk\n\
         SVar:NoBlk:Mode$ CantBlock | ValidCard$ Creature.withoutFlying",
    );
    assert!(cant_block_all.abilities[0].contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"));

    let plus_counter = read(
        "Name:CounterBoost\nManaCost:G\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ RPlus\n\
         SVar:RPlus:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ DBPlus\n\
         SVar:DBPlus:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Plus.1",
    );
    assert!(
        plus_counter.abilities[0].contains("Modifier::AdditionalCounterPlacement { n: 1 }"),
        "abilities: {:?}",
        plus_counter.abilities
    );

    // Refusal: ForgetOnCast$ one-shot flash effect stays refused.
    assert!(refused(
        "Name:NextSpellFlash\nManaCost:U\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ FlashOne | ForgetOnCast$ True\n\
         SVar:FlashOne:Mode$ CastWithFlash | ValidCard$ Sorcery | ValidSA$ Spell | Caster$ You"
    ));
}

#[test]
fn station_keyword_and_threshold_statics_abilities_and_triggers() {
    let craft = read(
        "Name:OrbitalCarrier\nManaCost:4 U\nTypes:Artifact\nPT:5/5\n\
         K:Station\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.counters_GE8_CHARGE | \
         AddType$ Creature | AddKeyword$ Flying & Trample | AddStaticAbility$ Buff | \
         AddAbility$ TapMana | AddTrigger$ AtkDraw | AddSVar$ DrawOne\n\
         SVar:Buff:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1\n\
         SVar:TapMana:AB$ Mana | Cost$ T | Produced$ U\n\
         SVar:AtkDraw:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ DrawOne\n\
         SVar:DrawOne:DB$ Draw | NumCards$ 1",
    );
    assert_eq!(
        craft.abilities[0],
        "activated!(cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)), \
         &[Effect::AddCounter { kind: CounterKind::Charge, amount: Amount::TappedPower }], \
         timing = ActivationTiming::SorcerySpeed)"
    );
    assert!(craft.abilities.iter().any(|a| a
        == "static_ability!(Filter::This, Modifier::AddTypeIfCountersAtLeast { \
            kind: CounterKind::Charge, at_least: 8, types: TypeSet::CREATURE })"));
    assert!(craft.abilities.iter().any(|a| a
        == "static_ability!(Filter::This, Modifier::AddKeywordIfCountersAtLeast { \
            kind: CounterKind::Charge, at_least: 8, \
            keywords: KeywordSet::FLYING.union(KeywordSet::TRAMPLE) })"));
    assert!(
        craft.abilities.iter().any(
            |a| a.contains("Modifier::ModifyPT(1, 1), condition = Some(Condition::Station(8))")
        )
    );
    assert!(craft.abilities.iter().any(|a| a
        == "mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Blue, 1)], \
            condition = Some(Condition::Station(8)))"));
    assert!(craft.abilities.iter().any(|a| a.starts_with("triggered!(")
        && a.ends_with(", condition = Some(Condition::Station(8)))")));

    assert!(refused(
        "Name:StationSpell\nManaCost:1 U\nTypes:Sorcery\nK:Station\nA:SP$ Draw | NumCards$ 1"
    ));
}

#[test]
fn level_up_keyword_and_level_bands_including_protection_and_granted_statics() {
    let leveler = read(
        "Name:BandAdept\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         K:Level up:2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | SetPower$ 3 | SetToughness$ 3 | \
         AddKeyword$ Flying & Protection from red | \
         IsPresent$ Card.Self+counters_GE1_LEVEL+counters_LE3_LEVEL | PresentCompare$ EQ1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | SetPower$ 5 | SetToughness$ 5 | \
         AddStaticAbility$ Anthem | IsPresent$ Card.Self+counters_GE4_LEVEL | PresentCompare$ EQ1\n\
         SVar:Anthem:Mode$ Continuous | Affected$ Creature.YouCtrl+Other | \
         AddPower$ 1 | AddToughness$ 1 | IsPresent$ Card.Self+counters_GE4_LEVEL | PresentCompare$ EQ1",
    );
    assert_eq!(
        leveler.abilities[0],
        r#"activated!(cost!("{2}"), &[Effect::AddCounter { kind: CounterKind::Level, amount: Amount::Fixed(1) }], timing = ActivationTiming::SorcerySpeed)"#
    );
    assert!(leveler.abilities.iter().any(|a| a
        == "static_ability!(Filter::This, Modifier::SetPT(3, 3), \
            condition = Some(Condition::CountersOnSelfBetween(CounterKind::Level, 1, 3)))"));
    assert!(
        leveler
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ProtectionFrom(")
                && a.contains("Condition::CountersOnSelfBetween(CounterKind::Level, 1, 3)"))
    );
    assert!(leveler.abilities.iter().any(|a| a
        == "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::FLYING), \
            condition = Some(Condition::CountersOnSelfBetween(CounterKind::Level, 1, 3)))"));
    // Duplicate inner condition on Anthem is not double-wrapped into Condition::All.
    assert!(
        leveler
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ModifyPT(1, 1)")
                && a.ends_with(
                    ", condition = Some(Condition::CountersOnSelf(CounterKind::Level, 4)))"
                )
                && !a.contains("Condition::All"))
    );

    let lt_band = read(
        "Name:OpenBand\nManaCost:U\nTypes:Creature\nPT:1/2\n\
         K:Level up:1 U\n\
         S:Mode$ Continuous | AffectedDefined$ Self | SetPower$ 2 | SetToughness$ 4 | \
         IsPresent$ Card.Self+counters_GE2_LEVEL+counters_LT5_LEVEL",
    );
    assert!(lt_band.abilities.iter().any(|a| a
        == "static_ability!(Filter::This, Modifier::SetPT(2, 4), \
            condition = Some(Condition::CountersOnSelfBetween(CounterKind::Level, 2, 4)))"));
}

#[test]
fn cant_attach_and_tap_power_value_statics() {
    let no_aura = read(
        "Name:UnenchantableConstruct\nTypes:Artifact Creature\nPT:2/2\n\
         S:Mode$ CantAttach | ValidCard$ Card.Aura | Target$ Card.Self",
    );
    assert_eq!(
        no_aura.abilities,
        ["static_ability!(Filter::This, Modifier::CantBeEnchanted)"]
    );

    let solo_aura = read(
        "Name:ExclusiveWard\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CantAttach | ValidCard$ Aura.Other | Target$ Creature.EnchantedBy",
    );
    assert!(
        solo_aura
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::CantBeEnchantedExceptSource"))
    );

    let no_new_aura = read(
        "Name:RelicGuard\nTypes:Creature\nPT:2/4\n\
         S:Mode$ CantAttach | ValidCardToTarget$ Aura.!Attached | \
         Target$ Artifact.nonCreature+YouCtrl | IsPresent$ Card.Self+untapped",
    );
    assert!(
        no_new_aura
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::CantBecomeEnchanted") && a.contains("condition = Some("))
    );

    let pilot = read(
        "Name:AcePilot\nTypes:Creature\nPT:1/1\n\
         S:Mode$ TapPowerValue | ValidSA$ Activated.Saddle+Mount,Activated.Crew+Vehicle | \
         ValidCard$ Card.Self | Value$ 2",
    );
    assert_eq!(
        pilot.abilities,
        ["static_ability!(Filter::This, Modifier::CrewSaddlePowerBonus(2))"]
    );

    // Crew-only and toughness-based TapPowerValue stay refused.
    assert!(refused(
        "Name:CrewOnly\nTypes:Creature\nPT:1/1\n\
         S:Mode$ TapPowerValue | ValidSA$ Activated.Crew+Vehicle | ValidCard$ Card.Self | Value$ 2"
    ));
    assert!(refused(
        "Name:ToughCrew\nTypes:Creature\nPT:0/6\n\
         S:Mode$ TapPowerValue | ValidSA$ Activated.Crew+Vehicle,Activated.Saddle+Mount | \
         ValidCard$ Card.Self | Value$ Toughness"
    ));
}

#[test]
fn continuous_extra_check_svar_conditions_and_cast_with_flash_svar() {
    let corrupted = read(
        "Name:CorruptedKnight\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Double Strike | \
         CheckSVar$ X | SVarCompare$ GE3\n\
         SVar:X:PlayerCountOpponents$HighestCounters.Poison",
    );
    assert_eq!(
        corrupted.abilities,
        [
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::DOUBLE_STRIKE), \
             condition = Some(Condition::OpponentPoisonAtLeast(3)))"
        ]
    );

    let empty_hands = read(
        "Name:EmptyMind\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddToughness$ 2 | \
         CheckSVar$ H | SVarCompare$ EQ0\n\
         SVar:H:Count$ValidHand Card.All",
    );
    assert_eq!(
        empty_hands.abilities,
        ["static_ability!(Filter::This, Modifier::ModifyPT(2, 2), \
             condition = Some(Condition::EveryHandEmpty))"]
    );

    let life_below = read(
        "Name:Underdog\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Lifelink | \
         CheckSVar$ L | SVarCompare$ LTS\n\
         SVar:L:Count$YourLifeTotal\nSVar:S:Count$StartingLife",
    );
    assert_eq!(
        life_below.abilities,
        [
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::LIFELINK), \
             condition = Some(Condition::LifeBelowStarting))"
        ]
    );

    let flash_cond = read(
        "Name:PoisonFlash\nTypes:Enchantment\n\
         S:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell | Caster$ You | \
         CheckSVar$ P | SVarCompare$ GE3\n\
         SVar:P:PlayerCountOpponents$HighestCounters.Poison",
    );
    assert_eq!(
        flash_cond.abilities,
        [
            "static_ability!(Filter::Any, Modifier::SpellsHaveFlash(None), \
             condition = Some(Condition::OpponentPoisonAtLeast(3)))"
        ]
    );
}

#[test]
fn annihilator_bands_with_other_and_etb_name_card() {
    let crusher = read(
        "Name:VoidColossus\nTypes:Creature\nPT:8/8\n\
         K:Annihilator:2\nK:Annihilator:1",
    );
    assert_eq!(
        crusher.abilities,
        [
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::SacrificeAmountForPlayers { \
             who: PlayerRel::DefendingPlayer, filter: &Filter::Any, amount: Amount::Fixed(2) }])",
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::SacrificeAmountForPlayers { \
             who: PlayerRel::DefendingPlayer, filter: &Filter::Any, amount: Amount::Fixed(1) }])",
        ]
    );
    assert!(refused(
        "Name:ZeroAnnih\nTypes:Creature\nPT:2/2\nK:Annihilator:0"
    ));
    assert!(refused("Name:SpellAnnih\nTypes:Sorcery\nK:Annihilator:1"));

    let bands_kw = read(
        "Name:GoblinCaptain\nTypes:Creature Goblin\nPT:2/2\n\
         K:Bands with other:Creature.Goblin:Goblins",
    );
    assert_eq!(
        bands_kw.abilities,
        ["static_ability!(Filter::This, Modifier::BandsWithOther(&BANDS1))"]
    );
    assert!(bands_kw.statics.contains("subtypes::creature::GOBLIN"));

    let bands_static = read(
        "Name:GoblinKeep\nTypes:Land\n\
         S:Mode$ Continuous | Affected$ Creature.Goblin+YouCtrl | \
         AddKeyword$ Bands with other:Creature.Goblin:Goblins",
    );
    assert!(
        bands_static
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::BandsWithOther(&BANDS1)"))
    );

    let name_any = read(
        "Name:NeedleRelic\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChooseName\n\
         SVar:ChooseName:DB$ NameCard | Defined$ You",
    );
    assert_eq!(name_any.enter_modifiers, ["EnterModifier::ChooseCardName"]);

    let name_nonland = read(
        "Name:MageSeer\nTypes:Creature\nPT:2/2\n\
         K:ETBReplacement:Other:ChooseName\n\
         SVar:ChooseName:DB$ NameCard | Defined$ You | ValidCards$ Card.nonLand",
    );
    assert_eq!(
        name_nonland.enter_modifiers,
        ["EnterModifier::ChooseCardNameExcept(TypeSet::LAND)"]
    );

    assert!(refused(
        "Name:RandomNamer\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChooseName\n\
         SVar:ChooseName:DB$ NameCard | Defined$ You | AtRandom$ True"
    ));
    assert!(refused(
        "Name:LandOnlyNamer\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChooseName\n\
         SVar:ChooseName:DB$ NameCard | Defined$ You | ValidCards$ Land"
    ));
}

#[test]
fn library_graveyard_and_hand_zone_continuous_statics() {
    let lib = read(
        "Name:OracleCitadel\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Library | Affected$ Card.TopLibrary+YouCtrl | MayLookAt$ You\n\
         S:Mode$ Continuous | AffectedZone$ Library | Affected$ Card.TopLibrary+YouOwn | MayLookAt$ Player\n\
         S:Mode$ Continuous | AffectedZone$ Library | Affected$ Card.TopLibrary | MayLookAt$ Player\n\
         S:Mode$ Continuous | AffectedZone$ Library | Affected$ Land.TopLibrary+YouCtrl | MayPlay$ True\n\
         S:Mode$ Continuous | AffectedZone$ Library | Affected$ Card.TopLibrary+YouCtrl | \
         MayPlay$ True | MayPlayAltManaCost$ PayLife<ConvertedManaCost>",
    );
    assert_eq!(
        lib.abilities,
        [
            "static_ability!(Filter::Any, Modifier::LookAtLibraryTop)",
            "static_ability!(Filter::Any, Modifier::RevealLibraryTop)",
            "static_ability!(Filter::Any, Modifier::RevealAllLibraryTops)",
            "static_ability!(Filter::Any, Modifier::PlayLandsFromLibraryTop)",
            "static_ability!(Filter::Any, Modifier::PlayFromLibraryTopForManaValueLife)",
        ]
    );

    let gy_lands = read(
        "Name:WorldCrucible\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Land.YouOwn | MayPlay$ True",
    );
    assert_eq!(
        gy_lands.abilities,
        ["static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard)"]
    );

    let gy_all = read(
        "Name:WillRelic\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Card.YouOwn | MayPlay$ True",
    );
    assert_eq!(
        gy_all.abilities,
        [
            "static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard)",
            "static_ability!(Filter::Any, Modifier::CastSpellsFromGraveyard)",
        ]
    );

    let gy_perm = read(
        "Name:PermRecaster\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Permanent.nonLand+YouOwn | MayPlay$ True",
    );
    assert_eq!(
        gy_perm.abilities,
        ["static_ability!(Filter::Any, Modifier::CastPermanentSpellsFromGraveyard)"]
    );

    let gy_once = read(
        "Name:SpellMage\nTypes:Creature\nPT:3/4\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | \
         MayPlay$ True | MayPlayLimit$ 1 | ReplaceGraveyard$ Exile | Condition$ PlayerTurn",
    );
    assert_eq!(
        gy_once.abilities,
        ["static_ability!(Filter::Any, Modifier::GraveyardInstantSorceryOnce)"]
    );

    let reveal_hand = read(
        "Name:OpenHand\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedZone$ Hand | Affected$ Card.YouOwn | MayLookAt$ Player",
    );
    assert_eq!(
        reveal_hand.abilities,
        ["static_ability!(Filter::Any, Modifier::RevealYourHand)"]
    );

    // Subtype-filtered graveyard land permission now lowers to filtered `PlayLandsFromGraveyard`.
    let forest_crucible = read(
        "Name:ForestCrucible\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Forest.YouOwn | MayPlay$ True",
    );
    assert_eq!(
        forest_crucible.abilities,
        [
            "static_ability!(Filter::HasSubtype(subtypes::land::FOREST), Modifier::PlayLandsFromGraveyard)"
        ]
    );
}

#[test]
fn wave5_characteristic_continuous_and_lose_abilities() {
    let frogify = read(
        "Name:GoblinHex\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | SetPower$ 1 | SetToughness$ 1 | \
         RemoveAllAbilities$ True | RemoveCardTypes$ True | RemoveCreatureTypes$ True | \
         AddType$ Creature & Goblin",
    );
    assert!(frogify.abilities.iter().any(|a| a.contains(
        "Modifier::BecomeType { types: TypeSet::CREATURE, subtype: subtypes::creature::GOBLIN }"
    )));
    assert!(
        frogify
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SetPT(1, 1)"))
    );
    assert!(
        frogify
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::LoseAllAbilities"))
    );

    let replace_types = read(
        "Name:MaskConspiracy\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | RemoveCreatureTypes$ True | AddType$ Goblin\n\
         S:Mode$ Continuous | Affected$ Creature.OppCtrl | AddAllCreatureTypes$ True\n\
         S:Mode$ Continuous | Affected$ Land | AddType$ AllBasicLandType\n\
         S:Mode$ Continuous | Affected$ Permanent | SetColor$ Colorless",
    );
    assert!(
        replace_types
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ReplaceCreatureTypes(subtypes::creature::GOBLIN)"))
    );
    assert!(
        replace_types
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AllCreatureTypes"))
    );
    assert!(
        replace_types
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AllBasicLandTypes"))
    );
    assert!(
        replace_types
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SetColor(ColorSet::EMPTY)"))
    );

    // Same-line RemoveAllAbilities + AddKeyword stays refused (timestamp wipe guard).
    assert!(refused(
        "Name:WipeAndGrant\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature | RemoveAllAbilities$ True | AddKeyword$ Flying"
    ));
    // Mixed card types with BecomeType stay refused (would clear artifact subtypes).
    assert!(refused(
        "Name:MixedBecome\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | RemoveCardTypes$ True | \
         RemoveCreatureTypes$ True | AddType$ Artifact & Creature & Goblin"
    ));
}

#[test]
fn wave5_combat_draw_hexproof_ward_legend_and_mana_convert_statics() {
    let statics = read(
        "Name:RuleEngine\nTypes:Enchantment\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Creature.YouCtrl | Min$ 2\n\
         S:Mode$ IgnoreLegendRule | ValidCard$ Permanent.YouCtrl\n\
         S:Mode$ AttackRestrict | MaxAttackers$ 1\n\
         S:Mode$ BlockRestrict | MaxBlockers$ 1\n\
         S:Mode$ CantDraw | ValidPlayer$ Opponent | DrawLimit$ 1\n\
         S:Mode$ CantDraw | ValidPlayer$ You\n\
         S:Mode$ IgnoreHexproof | ValidEntity$ Creature.OppCtrl\n\
         S:Mode$ DisableTriggers | ValidTrigger$ Triggered.Ward\n\
         S:Mode$ ManaConvert | ManaConversion$ AnyType->AnyColor\n\
         S:Mode$ ManaConvert | ValidPlayer$ You | ManaConversion$ Red->Green\n\
         S:Mode$ ManaConvert | ValidPlayer$ You | ValidCard$ Creature.YouCtrl+inZoneBattlefield | \
         ValidSA$ Activated | ManaConversion$ AnyType->AnyColor",
    );
    assert_eq!(
        statics.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::MinimumBlockers(2))",
            "static_ability!(Filter::Any, Modifier::LegendRuleOff)",
            "static_ability!(Filter::Any, Modifier::MaxAttackersPerCombat(1))",
            "static_ability!(Filter::Any, Modifier::MaxBlockersPerCombat(1))",
            "static_ability!(Filter::Any, Modifier::DrawLimitPerTurn { who: PlayerRel::EachOpponent, limit: 1 })",
            "static_ability!(Filter::Any, Modifier::DrawLimitPerTurn { who: PlayerRel::You, limit: 0 })",
            "static_ability!(Filter::OPPONENT_CREATURE, Modifier::CanBeTargetedAsThoughNoHexproof)",
            "static_ability!(Filter::Any, Modifier::WardDoesntTrigger)",
            "static_ability!(Filter::Any, Modifier::ManaIsAnyColor)",
            "static_ability!(Filter::Any, Modifier::SpendManaAs { from: ManaColor::Red, to: ManaColor::Green })",
            "static_ability!(Filter::Any, Modifier::CreatureAbilityManaAnyColor)",
        ]
    );

    // Per-attacker Max$ lowers to Modifier::MaxBlockers; zero stays refused.
    let solo = read(
        "Name:SoloBlocker\nTypes:Creature\nPT:3/3\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Card.Self | Max$ 1",
    );
    assert_eq!(
        solo.abilities,
        ["static_ability!(Filter::This, Modifier::MaxBlockers(1))"]
    );
    assert!(refused(
        "Name:ZeroBlocker\nTypes:Creature\nPT:3/3\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Card.Self | Max$ 0"
    ));
    assert!(refused(
        "Name:GlobalMirror\nTypes:Artifact\n\
         S:Mode$ IgnoreLegendRule | ValidCard$ Permanent"
    ));
    assert!(refused(
        "Name:PersonalCrawlspace\nTypes:Artifact\n\
         S:Mode$ AttackRestrict | MaxAttackers$ 2 | ValidDefender$ You"
    ));
}

#[test]
fn wave5_cant_be_activated_cant_be_cast_and_self_cast_conditions() {
    let locks = read(
        "Name:GrandLock\nTypes:Artifact\n\
         S:Mode$ CantBeActivated | ValidCard$ Card.NamedCard | ValidSA$ Activated.!ManaAbility\n\
         S:Mode$ CantBeActivated | AffectedZone$ Battlefield | ValidCard$ Artifact.OppCtrl | ValidSA$ Activated\n\
         S:Mode$ CantBeActivated | AffectedZone$ Battlefield | ValidCard$ Artifact,Creature | ValidSA$ Activated.!ManaAbility\n\
         S:Mode$ CantBeActivated | AffectedZone$ Battlefield | Activator$ Player | ValidCard$ Creature | ValidSA$ Activated\n\
         S:Mode$ CantBeCast | ValidCard$ Card | Caster$ Opponent | OnlySorcerySpeed$ True\n\
         S:Mode$ CantBeCast | ValidCard$ Card.NamedCard | Caster$ Player\n\
         S:Mode$ CantBeCast | ValidCard$ Card.NamedCard+nonLand | Caster$ Opponent\n\
         S:Mode$ CantBeCast | Origin$ Graveyard,Library\n\
         S:Mode$ CantBeCast | ValidCard$ Instant | Phases$ BeginCombat->EndCombat\n\
         S:Mode$ CantBeCast | ValidCard$ Creature | Caster$ Player.EnchantedController",
    );
    assert!(
        locks.abilities.contains(
            &"static_ability!(Filter::Any, Modifier::ChosenNameCantActivate)".to_string()
        )
    );
    assert!(
        locks
            .abilities
            .contains(&"static_ability!(Filter::Any, Modifier::CantActivateArtifacts)".to_string())
    );
    assert!(
        locks
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::CantActivateNonMana"))
    );
    assert!(
        locks
            .abilities
            .contains(&"static_ability!(Filter::CREATURE, Modifier::CantActivate)".to_string())
    );
    assert!(
        locks.abilities.contains(
            &"static_ability!(Filter::Any, Modifier::OpponentsCastAsSorcery)".to_string()
        )
    );
    assert!(locks.abilities.contains(
        &"static_ability!(Filter::Any, Modifier::PlayersCantCast { who: PlayerRel::EachPlayer, filter: &Filter::NamedBySource })".to_string()
    ));
    assert!(
        locks.abilities.contains(
            &"static_ability!(Filter::Any, Modifier::OpponentsCantCast(&Filter::NamedBySource))"
                .to_string()
        )
    );
    assert!(locks.abilities.iter().any(|a| {
        a.contains("Modifier::PlayersCantCast { who: PlayerRel::EachPlayer")
            && a.contains("CANT_CAST")
    }));
    assert!(locks.abilities.iter().any(|a| {
        a.contains("Modifier::PlayersCantCast")
            && a.contains("condition = Some(Condition::DuringCombat)")
    }));
    assert!(
        locks
            .abilities
            .iter()
            .any(|a| { a.contains("who: PlayerRel::ControllerOfAttached") })
    );
    assert!(locks.statics.contains("Filter::InZone(ZoneRef::Graveyard)"));
    assert!(locks.statics.contains("Filter::InZone(ZoneRef::Library)"));
}

#[test]
fn wave5_self_cast_conditions_and_activation_lock_refusals() {
    let avenger = read(
        "Name:PatientAngel\nTypes:Creature\nPT:3/3\n\
         S:Mode$ CantBeCast | ValidCard$ Card.Self | Caster$ Player.Active | \
         EffectZone$ All | CheckSVar$ Count$YourTurns | SVarCompare$ LE3",
    );
    assert_eq!(
        avenger.abilities,
        ["spell!(&[], condition = Some(Condition::Not(&Condition::DuringYourFirstTurns(3))))"]
    );

    let morbid_cast = read(
        "Name:GraveWalker\nTypes:Creature\nPT:5/3\n\
         S:Mode$ CantBeCast | ValidCard$ Card.Self | EffectZone$ All | \
         CheckSVar$ Died | SVarCompare$ EQ0\n\
         SVar:Died:Count$Morbid.1.0",
    );
    assert_eq!(
        morbid_cast.abilities,
        ["spell!(&[], condition = Some(Condition::CreaturesDiedThisTurnAtLeast(1)))"]
    );

    // Refusals: Activator$ Opponent on Artifact, AffectedZone$ on NamedCard,
    // all-zone Card without AffectedZone$ Battlefield, self-cast on Instant.
    assert!(refused(
        "Name:WrongKarn\nTypes:Artifact\n\
         S:Mode$ CantBeActivated | ValidCard$ Artifact | Activator$ Opponent"
    ));
    assert!(refused(
        "Name:BfOnlyNeedle\nTypes:Artifact\n\
         S:Mode$ CantBeActivated | AffectedZone$ Battlefield | ValidCard$ Card.NamedCard | \
         ValidSA$ Activated.!ManaAbility"
    ));
    assert!(refused(
        "Name:AllZoneLock\nTypes:Enchantment\n\
         S:Mode$ CantBeActivated | ValidCard$ Card | Phases$ BeginCombat->EndCombat | \
         ValidSA$ Activated.!ManaAbility"
    ));
    assert!(refused(
        "Name:InstantFirstTurns\nTypes:Instant\n\
         S:Mode$ CantBeCast | ValidCard$ Card.Self | Caster$ Player.Active | \
         EffectZone$ All | CheckSVar$ Count$YourTurns | SVarCompare$ LE3\n\
         A:SP$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave5_combat_unless_conditional_attack_and_compound_svar_conditions() {
    let taxes = read(
        "Name:FortressWall\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You,Planeswalker.YouCtrl | Cost$ 2\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You,Planeswalker.YouCtrl | Cost$ PayLife<2>\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Cost$ 3\n\
         S:Mode$ CantBlockUnless | ValidCard$ Creature | Cost$ 2\n\
         S:Mode$ CantBlockUnless | ValidCard$ Creature | Attacker$ Creature.YouCtrl | Cost$ 1",
    );
    assert_eq!(
        taxes.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::AttackTaxMana { mana: 2 })",
            "static_ability!(Filter::CREATURE, Modifier::AttackTaxLife { life: 2 })",
            "static_ability!(Filter::CREATURE, Modifier::CantAttackUnlessPayMana(3))",
            "static_ability!(Filter::CREATURE, Modifier::CantBlockUnlessPayMana(2))",
            "static_ability!(Filter::CREATURE, Modifier::CantBlockMatchingUnlessPayMana { attacker: &Filter::YOUR_CREATURE, mana: 1 })",
        ]
    );

    // Refusal: Target$ You alone does not cover planeswalkers you control.
    assert!(refused(
        "Name:PlayerOnlyPrison\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You | Cost$ 2"
    ));

    let sea_beast = read(
        "Name:DeepLeviathan\nTypes:Creature\nPT:10/10\n\
         S:Mode$ CantAttackUnless | ValidCard$ Card.Self | Cost$ Sac<2/Island/Islands>",
    );
    assert_eq!(
        sea_beast.abilities,
        [
            "static_ability!(Filter::This, Modifier::CantAttackUnlessYouSacrifice { count: 2, filter: &ATTACK_SAC1 })"
        ]
    );
    assert!(sea_beast.statics.contains("subtypes::land::ISLAND"));

    let cond_self = read(
        "Name:SleepingBrute\nTypes:Creature\nPT:5/5\n\
         S:Mode$ CantAttack | ValidCard$ Card.Self | CheckSVar$ X | SVarCompare$ EQ0\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert_eq!(
        cond_self.abilities,
        [
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_ATTACK), \
             condition = Some(Condition::Not(&Condition::CastInstantOrSorceryThisTurn)))"
        ]
    );

    let compound = read(
        "Name:DualCheck\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | \
         CheckSVar$ A | SVarCompare$ GE4 | CheckSecondSVar$ B | SecondSVarCompare$ LE20\n\
         SVar:A:Count$TotalPower Creature.YouCtrl\n\
         SVar:B:PlayerCount$LowestCardsInLibrary",
    );
    assert_eq!(
        compound.abilities,
        [
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::FLYING), \
             condition = Some(Condition::All(&[Condition::ControlledCreaturePowerAtLeast(4), Condition::AnyLibraryAtMost(20)])))"
        ]
    );

    let lock_three = read(
        "Name:ArrestAura\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CantAttack,CantBlock,CantBeActivated | ValidCard$ Creature.EnchantedBy | \
         ValidSA$ Activated.!ManaAbility",
    );
    assert!(
        lock_three
            .abilities
            .iter()
            .any(|a| { a.contains("KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)") })
    );
    assert!(
        lock_three
            .abilities
            .iter()
            .any(|a| { a.contains("Modifier::CantActivateNonMana") })
    );

    let block_svar = read(
        "Name:ShadowStalker\nTypes:Creature\nPT:2/2\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Card.Self | ValidBlocker$ Creature | \
         CheckSVar$ H | SVarCompare$ LE1\n\
         SVar:H:Count$CardsInYourHand",
    );
    assert!(
        block_svar
            .abilities
            .iter()
            .any(|a| { a.contains("Condition::HandSizeAtMost(1)") })
    );
}

#[test]
fn wave5_draw_turn_life_and_discard_replacements() {
    let skip_draw = read(
        "Name:DrawSkipper\nTypes:Enchantment\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Draw | ValidPlayer$ You | Skip$ True\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ Player | Prevent$ True",
    );
    assert_eq!(
        skip_draw.abilities,
        [
            "static_ability!(Filter::Any, Modifier::SkipDrawStep { who: PlayerRel::You })",
            "static_ability!(Filter::Any, Modifier::LifeGainNoLifeInstead { who: PlayerRel::EachPlayer })",
        ]
    );

    let fasting = read(
        "Name:AsceticFast\nTypes:Enchantment\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Draw | ValidPlayer$ You | \
         Optional$ True | ReplaceWith$ GainTwo\n\
         SVar:GainTwo:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    );
    assert_eq!(
        fasting.abilities,
        ["AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepGainLife(2))"]
    );

    let vault = read(
        "Name:ClockVault\nTypes:Artifact\n\
         R:Event$ BeginTurn | ActiveZones$ Battlefield | ValidPlayer$ You | \
         IsPresent$ Card.Self+tapped | Optional$ True | ReplaceWith$ UntapSelf\n\
         SVar:UntapSelf:DB$ Untap | Defined$ Self",
    );
    assert_eq!(
        vault.abilities,
        ["AbilityDef::Replacement(ReplacementRule::SkipTurnToUntapSelf)"]
    );

    let chains = read(
        "Name:MephChains\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Player | \
         NotFirstCardInDrawStep$ True | ReplaceWith$ DiscardFirst\n\
         SVar:DiscardFirst:DB$ Discard | Defined$ ReplacedPlayer | Mandatory$ True | \
         NumCards$ 1 | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DrawIf\n\
         SVar:DrawIf:DB$ Draw | Defined$ ReplacedPlayer | ConditionDefined$ Remembered | \
         ConditionPresent$ Card | ConditionCompare$ GE1 | SubAbility$ MillElse\n\
         SVar:MillElse:DB$ Mill | Defined$ ReplacedPlayer | NumCards$ 1 | \
         ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | \
         SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        chains.abilities,
        ["AbilityDef::Replacement(ReplacementRule::DiscardInsteadOfLaterDraws)"]
    );

    let sanctuary = read(
        "Name:QuietIsle\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ActivePhases$ Draw | \
         PlayerTurn$ True | Optional$ True | ReplaceWith$ Shield\n\
         SVar:Shield:DB$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ NoAtk\n\
         SVar:NoAtk:Mode$ CantAttack | ValidCard$ Creature.withoutFlying+!hasKeywordLandwalk:Island | Target$ You",
    );
    assert_eq!(
        sanctuary.abilities,
        ["AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw)"]
    );

    let renewal = read(
        "Name:CycleRenewal\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ Peek\n\
         SVar:Peek:DB$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | SubAbility$ Bin\n\
         SVar:Bin:DB$ ChangeZone | Defined$ Remembered | ConditionDefined$ Remembered | \
         ConditionPresent$ Creature | Origin$ Library | Destination$ Graveyard | SubAbility$ DrawRest\n\
         SVar:DrawRest:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Creature | \
         ConditionCompare$ EQ0 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        renewal.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::RevealDrawPutMatchingInGraveyard(&Filter::CREATURE))"
        ]
    );

    let weirding = read(
        "Name:MindWeirding\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Player | ReplaceWith$ Peek\n\
         SVar:Peek:DB$ PeekAndReveal | Defined$ ReplacedPlayer | NoPeek$ True | SubAbility$ PayBin\n\
         SVar:PayBin:DB$ Dig | Defined$ ReplacedPlayer | DestinationZone$ Graveyard | \
         DigNum$ 1 | ChangeNum$ All | UnlessCost$ PayLife<2> | UnlessPayer$ NonReplacedPlayer | \
         UnlessSwitched$ True | UnlessResolveSubs$ WhenNotPaid | SubAbility$ DrawOne\n\
         SVar:DrawOne:DB$ Draw | Defined$ ReplacedPlayer | NumCards$ 1",
    );
    assert_eq!(
        weirding.abilities,
        ["AbilityDef::Replacement(ReplacementRule::RevealDrawOthersMayPayLife(2))"]
    );
}

#[test]
fn wave5_discard_and_phoenix_moved_replacements() {
    let leng = read(
        "Name:LibraryRelic\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.YouCtrl | \
         Discard$ True | EffectOnly$ True | Optional$ True | ReplaceWith$ ToTop\n\
         SVar:ToTop:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | \
         Destination$ Library | NoShuffle$ True",
    );
    assert_eq!(
        leng.abilities,
        ["AbilityDef::Replacement(ReplacementRule::MayDiscardToLibraryTop)"]
    );

    let phoenix = read(
        "Name:AshBird\nTypes:Creature\nPT:3/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Card.Self | ReplaceWith$ ToHand\n\
         SVar:ToHand:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | \
         Defined$ ReplacedCard | SubAbility$ LockEff\n\
         SVar:LockEff:DB$ Effect | Triggers$ CleanupTrig | RememberObjects$ ReplacedCard | \
         Duration$ UntilYourNextTurn | StaticAbilities$ NoCast,ShowIt\n\
         SVar:NoCast:Mode$ CantBeCast | ValidCard$ Card.IsRemembered | Caster$ You\n\
         SVar:ShowIt:Mode$ Continuous | AffectedZone$ Hand | Affected$ Card.IsRemembered | MayLookAt$ Player\n\
         SVar:CleanupTrig:Mode$ ChangesZone | Origin$ Hand | Destination$ Any | \
         ValidCard$ Card.IsRemembered | Execute$ ExileEff | TriggerZones$ Command | \
         TriggerController$ TriggeredCardController | Static$ True\n\
         SVar:ExileEff:DB$ ChangeZone | Origin$ Command | Destination$ Exile | Defined$ Self",
    );
    assert_eq!(
        phoenix.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ReturnSelfToHandRevealedCantPlayUntilOwnersNextTurn)"
        ]
    );

    // Refusal: mismatched cleanup trigger on Phoenix-shaped Moved replacement.
    assert!(refused(
        "Name:BadPhoenix\nTypes:Creature\nPT:3/2\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Card.Self | ReplaceWith$ ToHand\n\
         SVar:ToHand:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | \
         Defined$ ReplacedCard | SubAbility$ LockEff\n\
         SVar:LockEff:DB$ Effect | Triggers$ OtherTrig | RememberObjects$ ReplacedCard | \
         Duration$ UntilYourNextTurn | StaticAbilities$ NoCast,ShowIt\n\
         SVar:NoCast:Mode$ CantBeCast | ValidCard$ Card.IsRemembered | Caster$ You\n\
         SVar:ShowIt:Mode$ Continuous | AffectedZone$ Hand | Affected$ Card.IsRemembered | MayLookAt$ Player\n\
         SVar:OtherTrig:Mode$ Phase | Phase$ Upkeep | Execute$ DrawOne\n\
         SVar:DrawOne:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave6_filter_atoms_and_mana_restrictions() {
    let filter_card = read(
        "Name:AtomProbe\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.!IsCommander+wasDealtDamageThisTurn+faceUp+ThisTurnEntered",
    );
    assert!(filter_card.statics.contains("Filter::IsCommander"));
    assert!(filter_card.statics.contains("Filter::DealtDamageThisTurn"));
    assert!(filter_card.statics.contains("Filter::FaceDown"));
    assert!(filter_card.statics.contains("Filter::EnteredThisTurn"));

    let attached_and_color = read(
        "Name:AuraSweep\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Enchantment.AttachedTo Creature.YouCtrl,Creature.hasManaAbility+counters_GE1_ALL+SharesColorWith Valid Creature.YouCtrl",
    );
    let sweep = attached_and_color.abilities.join("\n");
    assert!(
        sweep.contains("Filter::AttachedTo(&Filter::YOUR_CREATURE)"),
        "abilities: {sweep}"
    );
    assert!(
        sweep.contains("Filter::SharesAColorWith(&Filter::YOUR_CREATURE)"),
        "abilities: {sweep}"
    );
    assert!(sweep.contains("Filter::HasManaAbility"));
    assert!(sweep.contains("Filter::HasAnyCounter"));
    assert!(refused(
        "Name:AmbiguousAttached\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Enchantment.AttachedTo Creature.YouCtrl+tapped"
    ));

    let battle_and_hand = read(
        "Name:CastCheck\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Battle,Card.wasCastFromHand+nonPermanent",
    );
    assert!(battle_and_hand.statics.contains("TypeSet::BATTLE"));
    assert!(battle_and_hand.statics.contains("Filter::CastFromHand"));

    let powerstone = read(
        "Name:StoneMana\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ CantCastNonArtifactSpells",
    );
    assert!(
        powerstone
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::NonSpellPaymentsOrMatchingSpells"))
    );

    let paired = read(
        "Name:AbilityRock\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ Activated\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ CumulativeUpkeep\n\
         A:AB$ Mana | Cost$ T | Produced$ G | RestrictValid$ Spell.Creature,Activated.Creature+inZoneBattlefield",
    );
    assert!(
        paired
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::Abilities"))
    );
    assert!(
        paired
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::CumulativeUpkeep"))
    );
    assert!(
        paired
            .abilities
            .iter()
            .any(|a| a.contains("ManaUse::SpellsOrAbilities"))
    );
    let split_pair = read(
        "Name:SplitPair\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ G | RestrictValid$ Spell.Creature,Activated.Artifact",
    );
    assert!(
        split_pair.abilities[0].contains("ManaUse::SpellsOrAbilities")
            && split_pair.abilities[0].contains(".for_abilities_matching(&Filter::ARTIFACT)")
    );
    assert!(refused(
        "Name:BadPair\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ G | RestrictValid$ Spell.Creature,Activated.Equip"
    ));
}

#[test]
fn wave6_counts_costs_and_peer_hooks() {
    let counts = read(
        "Name:CountSpell\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ Y\n\
         SVar:Y:Count$xPaid/Twice",
    );
    assert!(
        counts
            .abilities
            .iter()
            .any(|a| a.contains("Amount::DoubleX"))
    );

    let more_counts = read(
        "Name:KickedBurn\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ D\n\
         SVar:D:SVar$K/Div.2\n\
         SVar:K:Count$TimesKicked",
    );
    assert!(
        more_counts
            .abilities
            .iter()
            .any(|a| a.contains("Amount::divided(&Amount::TimesKicked, 2, false)"))
    );

    let library_and_zones = read(
        "Name:MillReader\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ A\n\
         SVar:A:Count$ValidGraveyard Card.OppOwn/Plus.B\n\
         SVar:B:Count$ValidExile Card.ExiledWithSource",
    );
    assert!(
        library_and_zones
            .abilities
            .iter()
            .any(|a| a.contains("ZoneSel::GraveyardOpponents"))
    );
    assert!(
        library_and_zones
            .abilities
            .iter()
            .any(|a| a.contains("ZoneSel::LinkedExile"))
    );

    let peer_counts = read(
        "Name:PeerTrigger\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Creature.Other | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ T\n\
         SVar:T:TriggeredCard$Toughness",
    );
    assert!(
        peer_counts
            .abilities
            .iter()
            .any(|a| a.contains("Amount::EventLastToughness"))
    );

    let costs = read(
        "Name:CostEngine\nTypes:Creature\nPT:3/3\n\
         A:AB$ Draw | Cost$ Exert<1/CARDNAME> Discard<0/Hand> | NumCards$ 1\n\
         A:AB$ Mana | Cost$ SubCounter<X/CHARGE> | Produced$ C | Amount$ X\n\
         A:AB$ Draw | Cost$ Sac<All/Artifact> tapXType<2/Creature> | NumCards$ 1\n\
         SVar:X:Count$xPaid",
    );
    assert!(costs.abilities.iter().any(|a| a.contains("ExertSelf")));
    assert!(costs.abilities.iter().any(|a| a.contains("DiscardHand")));
    assert!(
        costs
            .abilities
            .iter()
            .any(|a| a.contains("RemoveCounterSelfX { kind: CounterKind::Charge }"))
    );
    assert!(costs.abilities.iter().any(|a| a.contains("SacrificeAll(&")));
    assert!(
        costs
            .abilities
            .iter()
            .any(|a| a.contains("TapN { filter: &"))
    );

    // Refusals: Exert on a spell and variable-X loyalty cost.
    assert!(refused(
        "Name:SpellExert\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ R Exert<1/CARDNAME> | NumCards$ 1"
    ));
    assert!(refused(
        "Name:LoyaltyX\nTypes:Planeswalker\n\
         A:AB$ Draw | Cost$ SubCounter<X/LOYALTY> | NumCards$ X\n\
         SVar:X:Count$xPaid"
    ));
    // Non-mana stack-using `RemoveCounterSelfX` stays refused until backed by a
    // dedicated end-to-end engine test (Wave 7b / Hourglass of the Lost).
    assert!(refused(
        "Name:NonManaCounterX\nTypes:Artifact\n\
         A:AB$ GainLife | Cost$ SubCounter<X/CHARGE> | LifeAmount$ X\n\
         SVar:X:Count$xPaid"
    ));
}

#[test]
fn wave6_player_statics_and_conditions() {
    let player_rules = read(
        "Name:SanctumRelic\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ You | SetMaxHandSize$ Unlimited\n\
         S:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 2\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Hexproof",
    );
    assert_eq!(
        player_rules.abilities,
        [
            "static_ability!(Filter::Any, Modifier::NoMaxHandSize)",
            "static_ability!(Filter::Any, Modifier::ExtraLandDrops(2))",
            "static_ability!(Filter::Any, Modifier::PlayerHexproof { who: PlayerRel::You })",
        ]
    );

    let chosen_rack = read(
        "Name:ViseRack\nTypes:Artifact\n\
         K:ETBReplacement:Other:PickOpp\n\
         SVar:PickOpp:DB$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent\n\
         S:Mode$ Continuous | Affected$ Player.Chosen | SetMaxHandSize$ 4",
    );
    assert_eq!(
        chosen_rack.enter_modifiers,
        ["EnterModifier::ChooseOpponent"]
    );
    assert_eq!(
        chosen_rack.abilities,
        ["static_ability!(Filter::Any, Modifier::ChosenPlayerMaxHandSize(4))"]
    );

    // Refusals: global NoMaxHandSize/ExtraLandDrops, player Shroud, unchosen rack.
    assert!(refused(
        "Name:GlobalHand\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Player | SetMaxHandSize$ Unlimited"
    ));
    assert!(refused(
        "Name:GlobalLands\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Player | AdjustLandPlays$ 1"
    ));
    assert!(refused(
        "Name:PlayerShroud\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Shroud"
    ));
    assert!(refused(
        "Name:UnchosenRack\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Player.Chosen | SetMaxHandSize$ 4"
    ));

    let conds = read(
        "Name:CondTest\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 2 | CheckSVar$ E | SVarCompare$ EQ0\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddToughness$ 2 | CheckSVar$ D | SVarCompare$ GE7\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ Flying | CheckSVar$ G | SVarCompare$ GE4\n\
         SVar:E:Count$YourLifeTotal/Mod.2\n\
         SVar:D:Count$Valid Land.YouCtrl$DifferentCardNames\n\
         SVar:G:Count$ValidGraveyard Card.YouOwn$CardTypes",
    );
    assert!(
        conds
            .abilities
            .iter()
            .any(|a| a.contains("Condition::AmountIsEven(Amount::LifeTotal(PlayerRel::You))"))
    );
    assert!(
        conds
            .abilities
            .iter()
            .any(|a| a.contains("Condition::ControlDistinctNames(&Filter::YOUR_LAND, 7)"))
    );
    assert!(
        conds
            .abilities
            .iter()
            .any(|a| a.contains("Condition::GraveyardTypesAtLeast(4)"))
    );
}

#[test]
fn wave6_damage_preventions_life_floor_and_replacements() {
    let aura_redirect = read(
        "Name:ShieldAura\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | \
         DamageTarget$ Enchanted | ReplaceWith$ Redirect\n\
         SVar:Redirect:DB$ ReplaceEffect | VarName$ Affected | VarValue$ Enchanted | VarType$ Card",
    );
    assert!(aura_redirect.abilities.iter().any(|a| {
        a == "static_ability!(Filter::AttachedToBySource, Modifier::RedirectDamageToYou(&Filter::Any))"
    }));

    let cap_and_floor = read(
        "Name:DamageBounds\nTypes:Creature\nPT:4/4\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Instant,Sorcery | \
         ValidTarget$ You | DamageAmount$ GE3 | ReplaceWith$ CapTwo\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.RedSource+YouCtrl | \
         IsCombat$ False | ValidTarget$ Opponent | DamageAmount$ LTX | ReplaceWith$ FloorPow\n\
         SVar:CapTwo:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ 2\n\
         SVar:FloorPow:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:Count$CardPower",
    );
    assert!(
        cap_and_floor
            .abilities
            .iter()
            .any(|a| a.contains("DamageAmountReplacement::Cap(2)"))
    );
    assert!(
        cap_and_floor
            .abilities
            .iter()
            .any(|a| a.contains("DamageAmountReplacement::FloorAtEffectSourcePower"))
    );

    let life_floor = read(
        "Name:FaithFloor\nTypes:Enchantment\n\
         R:Event$ LifeReduced | ActiveZones$ Battlefield | ValidPlayer$ You.lifeGE1 | \
         Result$ LT1 | IsDamage$ True | IsPresent$ Creature.YouCtrl | ReplaceWith$ Floor\n\
         SVar:Floor:DB$ ReplaceEffect | VarName$ Amount | VarValue$ X\n\
         SVar:X:ReplaceCount$Amount/LimitMax.Diff\n\
         SVar:Diff:Count$YourLifeTotal/Minus.1",
    );
    assert_eq!(
        life_floor.abilities,
        [
            "static_ability!(Filter::This, Modifier::DamageLifeLossFloor { players: PlayerRel::You, \
             minimum: 1 }, condition = Some(Condition::All(&[Condition::AmountAtLeast(\
             Amount::LifeTotal(PlayerRel::You), 1), Condition::ControlCount(&Filter::YOUR_CREATURE, 1)])))"
        ]
    );

    let mut tok_cats = cats();
    tok_cats.artifact.push("Treasure".to_string());
    tok_cats.normalize();
    let tokens = crate::tokengen::TokenLookup::held(&[(
        "t_treasure",
        "Name:Treasure Token\nTypes:Artifact Treasure\n\
         A:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any",
    )]);
    let hull_script = parse(
        "Name:DrawThief\nTypes:Creature\nPT:3/2\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Opponent | \
         NotFirstCardInDrawStep$ True | ReplaceWith$ Mk\n\
         SVar:Mk:DB$ Token | TokenScript$ t_treasure",
    );
    let hull = transcode(&hull_script, &tok_cats, Some(&tokens)).expect("draw-to-token");
    assert_eq!(
        hull.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::OpponentsExtraDrawsCreateToken(&generated_tokens::TREASURE))"
        ]
    );

    let disturb_back = read(
        "Name:GhostBack\nTypes:Creature\nPT:2/2\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Graveyard | ReplaceWith$ ExileSelf\n\
         SVar:ExileSelf:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert_eq!(
        disturb_back.abilities,
        ["AbilityDef::Replacement(ReplacementRule::ExileSelfInsteadOfGraveyard)"]
    );

    // Refusal: non-combat permanent source outside Self/Attached, and mismatched LifeReduced threshold.
    assert!(refused(
        "Name:BadPrevent\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Creature | Prevent$ True"
    ));
    assert!(refused(
        "Name:BadFloor\nTypes:Enchantment\n\
         R:Event$ LifeReduced | ActiveZones$ Battlefield | ValidPlayer$ You.lifeGE7 | \
         Result$ LT1 | IsDamage$ True | ReplaceWith$ Floor\n\
         SVar:Floor:DB$ ReplaceEffect | VarName$ Amount | VarValue$ X\n\
         SVar:X:ReplaceCount$Amount/LimitMax.Diff\n\
         SVar:Diff:Count$YourLifeTotal/Minus.7"
    ));
}

#[test]
fn wave7_support_hooks_mana_counts_and_filter_atoms() {
    for api in ["Amass", "AlterAttribute", "BecomeMonarch", "GainControl"] {
        assert!(crate::scriptgen::is_supported_api(api), "{api}");
    }
    let prep = parse(
        "Name:Front\nTypes:Creature\nPT:2/2\nAlternateMode:Prepare\nALTERNATE\nName:Back\nTypes:Sorcery",
    );
    let why = refusal_reason(&prep, &cats(), None);
    assert!(
        why.as_deref() != Some("unsupported alternate mode `Prepare`"),
        "{why:?}"
    );

    let grant_sac = read(
        "Name:TorchGear\nTypes:Artifact Equipment\nK:Equip:1\n\
         S:Mode$ Continuous | Affected$ Creature.EquippedBy | AddAbility$ Throw\n\
         SVar:Throw:AB$ DealDamage | Cost$ 1 T Sac<1/OriginalHost/this equipment> | ValidTgts$ Any | NumDmg$ 2",
    );
    assert!(grant_sac.statics.contains("SacrificeGrantor"));
    assert!(
        grant_sac
            .abilities
            .iter()
            .any(|a| a.contains("GrantActivatedDefinition"))
    );

    let etb_x = read(
        "Name:FlareCharm\nManaCost:X R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Zap\n\
         SVar:Zap:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(etb_x.abilities[0].contains("amount: Amount::X"));
    assert!(refused(
        "Name:NoXCost\nManaCost:1 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Zap\n\
         SVar:Zap:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$xPaid"
    ));

    let counters_and_atoms = read(
        "Name:PulseCheck\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ E | SubAbility$ Zap\n\
         SVar:Zap:DB$ Destroy | ValidTgts$ Creature.HasCounters\n\
         SVar:E:Count$YourCountersEnergy",
    );
    assert!(counters_and_atoms.abilities[0].contains("Amount::CountersOnYou(CounterKind::Energy)"));
    assert!(counters_and_atoms.statics.contains("Filter::HasAnyCounter"));

    let band_and_share = read(
        "Name:BannerUnit\nTypes:Artifact Equipment\nK:Equip:1\n\
         S:Mode$ Continuous | Affected$ Creature.sharesCreatureTypeWith Equipped | AddPower$ 1 | AddToughness$ 1\n\
         A:AB$ PumpAll | Cost$ 2 | ValidCards$ Creature.attackersBandedWith | KW$ Trample",
    );
    assert!(
        band_and_share
            .abilities
            .iter()
            .any(|a| a.contains("SharesCreatureTypeWith(&Filter::AttachedToBySource)"))
    );
    assert!(
        band_and_share.statics.contains("Filter::BandedWithSource")
            || band_and_share
                .abilities
                .iter()
                .any(|a| a.contains("Filter::BandedWithSource"))
    );

    // Argon PR 394 regression guard (#215): single-target graveyard Oracle must refuse OppOwn.
    assert!(refused(
        "Name:SingleYardCount\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ GainLife | ValidTgts$ Opponent | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Creature.OppOwn\n\
         Oracle:Gain 1 life for each creature card in target opponent's graveyard."
    ));
}

#[test]
fn wave7_continuous_conditions_svar_pairs_and_graveyard_cda() {
    let svar_pairs = read(
        "Name:LifeScale\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 2 | AddToughness$ 2 | \
         CheckSVar$ Life | SVarCompare$ GEGoal\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Indestructible | \
         CheckSVar$ Life | SVarCompare$ LEHalf\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | AddToughness$ 1 | \
         Condition$ PlayerTurn | CheckSVar$ Atk | SVarCompare$ GE1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Trample | \
         CheckSVar$ Pow | SVarCompare$ GE8\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Flying | \
         CheckSVar$ Cnt | SVarCompare$ GE1\n\
         SVar:Life:Count$YourLifeTotal\n\
         SVar:Goal:Count$StartingLife/Plus.7\n\
         SVar:Half:Count$YourStartingLife/HalfDown\n\
         SVar:Atk:Count$AttackersDeclared\n\
         SVar:Pow:Count$Valid Creature.YouCtrl$CardPower\n\
         SVar:Cnt:Count$CardCounters.ALL",
    );
    assert!(svar_pairs.abilities[0].contains("Amount::Difference"));
    assert!(svar_pairs.abilities[0].contains("Amount::StartingLifeTotal(PlayerRel::You)"));
    assert!(svar_pairs.abilities[1].contains("Amount::Half"));
    assert!(
        svar_pairs.abilities[2]
            .contains("Condition::All(&[Condition::YourTurn, Condition::AttackedWithAtLeast(1)])")
    );
    assert!(svar_pairs.abilities[3].contains("Condition::ControlledCreaturePowerAtLeast(8)"));
    assert!(svar_pairs.abilities[4].contains("Condition::SourceMatches(&Filter::HasAnyCounter)"));

    let cda_you = read(
        "Name:YardWurm\nTypes:Creature\nPT:*/*\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    assert!(cda_you.abilities[0].contains("Modifier::CharacteristicPT"));
    assert!(cda_you.abilities[0].contains("PtCount::InAllGraveyards"));
    assert!(cda_you.statics.contains("Filter::OwnedByYou"));

    let cda_half = read(
        "Name:SpellDrake\nTypes:Creature\nPT:*/4\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X\n\
         SVar:X:Count$ValidGraveyard Instant.YouOwn,Sorcery.YouOwn",
    );
    assert!(cda_half.abilities[0].contains("Modifier::CharacteristicPTSeparate"));
    assert!(cda_half.abilities[0].contains("PtCount::Fixed(4)"));
}

#[test]
fn wave7_player_protection_cant_target_dice_pay_etb_and_attack_sac() {
    let mixed = read(
        "Name:WardAngel\nTypes:Creature\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ You,Creature.YouCtrl+Other | AddKeyword$ Hexproof",
    );
    assert!(
        mixed
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::PlayerHexproof { who: PlayerRel::You }"))
    );
    assert!(
        mixed
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AddKeyword(KeywordSet::HEXPROOF)"))
    );

    let halo = read(
        "Name:NameWard\nTypes:Enchantment\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ NameCard | Defined$ You\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Protection:ChosenName",
    );
    assert!(halo.abilities[0].contains(
        "Modifier::PlayerProtectionFrom { who: PlayerRel::You, from: &Filter::NamedBySource }"
    ));
    assert!(refused(
        "Name:BadHalo\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Protection:ChosenName"
    ));

    let cant_tgt = read(
        "Name:VeilCharm\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CantTarget | ValidTarget$ Creature.EnchantedBy | ValidSA$ Spell | Secondary$ True\n\
         S:Mode$ CantTarget | ValidTarget$ Card.EnchantedBy | ValidSource$ Artifact\n\
         Oracle:Enchanted creature can't be the target of abilities from artifact sources.",
    );
    assert!(cant_tgt.statics.contains("Filter::InZone(ZoneRef::Stack)"));
    assert!(
        cant_tgt
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::CantBeTargetedBy(&SOURCE"))
    );
    assert!(
        cant_tgt
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::CantBeTargetedByAbilitiesFrom(&Filter::ARTIFACT)"))
    );

    let dice = read(
        "Name:LuckyGuide\nTypes:Creature\nPT:1/3\n\
         R:Event$ RollDice | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ PlusRoll\n\
         SVar:PlusRoll:DB$ ReplaceEffect | VarName$ Number | VarValue$ ReplaceCount$Number/Plus.1 | SubAbility$ Ign\n\
         SVar:Ign:DB$ ReplaceEffect | VarName$ Ignore | VarValue$ ReplaceCount$Ignore/Plus.1",
    );
    assert!(dice.abilities[0].contains("Modifier::RollAdditionalDieIgnoreLowest"));

    let atk_sac = read(
        "Name:MarshTax\nTypes:Enchantment\n\
         S:Mode$ CantAttackUnless | ValidCard$ Creature.Green | Cost$ Sac<1/Land>",
    );
    assert!(atk_sac.abilities[0].contains("Modifier::CantAttackUnlessYouSacrifice"));

    let pay_sac = read(
        "Name:ValeLand\nTypes:Land\n\
         R:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self | ReplaceWith$ Pay\n\
         SVar:Pay:DB$ Sacrifice | SacValid$ Land.untapped | Defined$ You | RememberSacrificed$ True | Amount$ 2 | StrictAmount$ True | SubAbility$ Gy\n\
         SVar:Gy:DB$ ChangeZone | Origin$ All | Destination$ Graveyard | Defined$ ReplacedCard | ConditionCheckSVar$ X | ConditionSVarCompare$ LT2 | SubAbility$ Bf\n\
         SVar:Bf:DB$ ChangeZone | Origin$ All | Destination$ Battlefield | Defined$ ReplacedCard | ConditionCheckSVar$ X | ConditionSVarCompare$ GE2 | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(pay_sac.enter_modifiers[0].contains("EnterModifier::SacrificeOrGraveyard"));
    assert!(pay_sac.enter_modifiers[0].contains("count: 2"));

    let pay_disc = read(
        "Name:GemMox\nTypes:Artifact\n\
         R:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self | ReplaceWith$ Pay\n\
         SVar:Pay:DB$ Discard | DiscardValid$ Land | Mode$ TgtChoose | RememberDiscarded$ True | Optional$ True | SubAbility$ Gy\n\
         SVar:Gy:DB$ ChangeZone | Origin$ All | Destination$ Graveyard | Defined$ ReplacedCard | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ Bf\n\
         SVar:Bf:DB$ ChangeZone | Origin$ All | Destination$ Battlefield | Defined$ ReplacedCard | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ1 | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert_eq!(
        pay_disc.enter_modifiers,
        ["EnterModifier::DiscardOrGraveyard(&Filter::LAND)"]
    );
}

#[test]
fn wave8_support_hooks_filter_atoms_and_mana_counts() {
    for api in [
        "ChooseColor",
        "ProtectionAll",
        "RepeatEach",
        "ChooseCard",
        "PeekAndReveal",
        "Reveal",
        "Goad",
    ] {
        assert!(crate::scriptgen::is_supported_api(api), "{api}");
    }

    let filters = read(
        "Name:CrestTool\nTypes:Artifact Equipment\nK:Equip:1\n\
         S:Mode$ Continuous | Affected$ Equipped | AddPower$ 1\n\
         S:Mode$ Continuous | Affected$ Creature.sharesCreatureTypeWith Commander | AddToughness$ 1\n\
         A:AB$ Destroy | Cost$ 2 | ValidTgts$ Creature.!ChosenColor+withRampage",
    );
    assert!(
        filters
            .abilities
            .iter()
            .any(|a| a.contains("Filter::AttachedToBySource"))
    );
    assert!(
        filters
            .abilities
            .iter()
            .any(|a| a.contains("Filter::SharesSubtypeWithCommander"))
    );
    assert!(
        filters
            .statics
            .contains("Filter::Not(&Filter::OfSourceChosenColor)")
    );
    assert!(filters.statics.contains("Filter::HasRampage"));

    let dmg_counts = read(
        "Name:ReaperEdge\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | TriggerZones$ Battlefield | Execute$ Cut\n\
         T:Mode$ DamageDoneOnce | CombatDamage$ True | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | TriggerZones$ Battlefield | Execute$ Poke\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ Drain\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Buff\n\
         SVar:Cut:DB$ LoseLife | Defined$ TriggeredTarget | LifeAmount$ Half\n\
         SVar:Half:TriggeredTarget$LifeTotal/HalfUp\n\
         SVar:Poke:DB$ Destroy | ValidTgts$ Creature.ControlledBy TriggeredTarget\n\
         SVar:Drain:DB$ LoseLife | Defined$ You | LifeAmount$ Mv\n\
         SVar:Mv:TriggeredCard$CardManaCost\n\
         SVar:Buff:DB$ GainLife | Defined$ You | LifeAmount$ Tough\n\
         SVar:Tough:TriggeredAttacker$CardToughness",
    );
    assert!(dmg_counts.abilities[0].contains(
        "Amount::Half { base: &Amount::LifeTotal(PlayerRel::DamagedPlayer), round_up: true }"
    ));
    assert!(dmg_counts.abilities[1].contains("TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE)"));
    assert!(dmg_counts.abilities[2].contains("Amount::EventManaValue"));
    assert!(dmg_counts.abilities[3].contains("Amount::EventLastToughness"));

    let yards_and_cmd = read(
        "Name:RelicGauge\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ Gy | SubAbility$ Art\n\
         SVar:Art:DB$ GainLife | Defined$ You | LifeAmount$ ArtGy | SubAbility$ Melee\n\
         SVar:Melee:DB$ GainLife | Defined$ You | LifeAmount$ Atk | SubAbility$ Cmd\n\
         SVar:Cmd:DB$ GainLife | Defined$ You | LifeAmount$ CmdMv\n\
         SVar:Gy:TargetedPlayer$CardsInGraveyard\n\
         SVar:ArtGy:Count$ValidGraveyard Artifact.OwnedBy TargetedPlayer\n\
         SVar:Atk:PlayerCountPropertyYou$OpponentsAttackedThisCombat\n\
         SVar:CmdMv:Count$ValidAll Card.IsCommander+YouOwn$GreatestCardManaCost",
    );
    assert!(yards_and_cmd.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::Chosen)"));
    assert!(yards_and_cmd.abilities[0].contains("Amount::OpponentsYouAttackedThisCombat"));
    assert!(yards_and_cmd.abilities[0].contains("Amount::GreatestCommanderManaValue"));
}

#[test]
fn attacking_count_preserves_printed_tense() {
    for (oracle, amount) in [
        (
            "This spell costs {1} less for each opponent you're attacking.",
            "Amount::OpponentsYouAreAttacking",
        ),
        (
            "This spell costs {1} less for each opponent you attacked this combat.",
            "Amount::OpponentsYouAttackedThisCombat",
        ),
    ] {
        let body = read(&format!(
            "Name:Combat Toll\nManaCost:4 W\nTypes:Instant\n\
             S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ Stack | Amount$ Seats\n\
             SVar:Seats:PlayerCountPropertyYou$OpponentsAttackedThisCombat\n\
             A:SP$ Draw | NumCards$ 1\nOracle:{oracle}"
        ));
        assert!(
            body.abilities[0].contains(&format!("SpellsCostLessByAmount({amount})")),
            "{body:?}"
        );
    }

    let body = read(
        "Name:Battle Feast\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ You | Execute$ Feast\n\
         SVar:Feast:DB$ GainLife | Defined$ You | LifeAmount$ Seats\n\
         SVar:Seats:PlayerCountPropertyYou$OpponentsAttackedThisCombat\n\
         Oracle:Whenever you attack, gain 1 life for each player being attacked.",
    );
    assert!(body.abilities[0].contains("Amount::OpponentsYouAreAttacking"));
}

#[test]
fn permanent_and_controller_damage_follows_the_printed_event_boundary() {
    let spec = "Name:Forked Flames\nManaCost:2 R\nTypes:Instant\n\
                A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ Scorch\n\
                SVar:Scorch:DB$ DealDamage | Defined$ TargetedController | NumDmg$ 1";
    let joined = read(&format!(
        "{spec}\nOracle:Forked Flames deals 3 damage to target creature and 1 damage to its controller."
    ));
    assert!(
        joined.abilities[0].contains(
            "Effect::DamagePermanentAndController { target: TargetSpec::Object(&Filter::CREATURE), permanent_amount: Amount::Fixed(3), controller_amount: Amount::Fixed(1) }"
        )
    );

    for oracle in [
        "Forked Flames deals 3 damage to target creature. Then it deals 1 damage to that creature's controller.",
        "Forked Flames deals 3 damage to target creature and then 1 damage to its controller.",
    ] {
        let separate = read(&format!("{spec}\nOracle:{oracle}"));
        assert!(
            separate.abilities[0].contains("TargetSpec::Player(PlayerRel::ControllerOfTarget)")
        );
        assert!(!separate.abilities[0].contains("DamagePermanentAndController"));
    }

    let unsupported = format!(
        "{spec} | DamageSource$ TriggeredCard\nOracle:Forked Flames deals 3 damage to target creature and 1 damage to its controller."
    );
    assert!(refused(&unsupported));

    let multiple = format!(
        "{spec}\nOracle:Forked Flames deals 3 damage to target creature and 1 damage to its controller. Then it deals 2 damage to that creature's controller."
    );
    assert!(refused(&multiple));
}

#[test]
fn threshold_chooses_the_damage_event_before_applying_either_amount() {
    let body = read(
        "Name:Cinder Sledge\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | DamageMap$ True | SubAbility$ Singe\n\
         SVar:Singe:DB$ DealDamage | Defined$ TargetedController | Condition$ Threshold | NumDmg$ 3 | SubAbility$ Finish\n\
         SVar:Finish:DB$ DamageResolve\n\
         Oracle:Cinder Sledge deals 2 damage to target creature. Threshold — If there are seven or more cards in your graveyard, instead Cinder Sledge deals 2 damage to that creature and 3 damage to that creature's controller.",
    );
    assert!(
        body.abilities[0].contains(
            "Effect::IfCondition { condition: Condition::GraveyardCountAtLeast(7), then: &[Effect::DamagePermanentAndController { target: TargetSpec::Object(&Filter::CREATURE), permanent_amount: Amount::Fixed(2), controller_amount: Amount::Fixed(3) }], otherwise: &[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::Object(&Filter::CREATURE) }] }"
        )
    );
}

#[test]
fn paired_damage_preserves_the_permanent_filter_amounts_and_later_effects() {
    let mut catalog = cats();
    catalog.artifact.push("Equipment".into());
    catalog.normalize();
    let script = parse(
        "Name:Ashfall\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ 3 | ReplaceDyingDefined$ Targeted | SubAbility$ Burst\n\
         SVar:Burst:DB$ DealDamage | Defined$ TargetedController | NumDmg$ Wounds | SubAbility$ Finish\n\
         SVar:Finish:DB$ DamageResolve | SubAbility$ Cantrip\n\
         SVar:Cantrip:DB$ Draw | NumCards$ 1\n\
         SVar:Wounds:Count$Valid Equipment.YouCtrl\n\
         Oracle:Ashfall deals 3 damage to target creature or planeswalker and X damage to that permanent's controller, where X is the number of Equipment you control. If that permanent would die this turn, exile it instead. Draw a card.",
    );
    let body = transcode(&script, &catalog, None).expect("the damage pair reads in full");
    let spell = &body.abilities[0];
    let damage = spell.find("Effect::DamagePermanentAndController").unwrap();
    let exile = spell.find("Effect::ExileIfDiesThisTurn").unwrap();
    let draw = spell.find("Effect::draw(1)").unwrap();
    assert!(damage < exile && exile < draw);
    assert!(spell.contains("TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER)"));
    assert!(spell.contains("controller_amount: Amount::CountOf"));
    assert!(body.statics.contains("artifact::EQUIPMENT"));
    assert!(body.statics.contains("Filter::ControlledByYou"));
}

#[test]
fn paired_damage_uses_the_creature_at_either_side_of_a_block() {
    let body = read(
        "Name:Cinder Brute\nTypes:Creature\nPT:4/3\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature | ValidBlocker$ Card.Self | Execute$ ScorchAttacker\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Execute$ ScorchBlocker | Secondary$ True\n\
         SVar:ScorchAttacker:DB$ DealDamage | Defined$ TriggeredAttacker | NumDmg$ 1 | SubAbility$ AttackController\n\
         SVar:ScorchBlocker:DB$ DealDamage | Defined$ TriggeredBlocker | NumDmg$ 1 | SubAbility$ BlockController\n\
         SVar:AttackController:DB$ DealDamage | Defined$ TriggeredAttackerController | NumDmg$ 2\n\
         SVar:BlockController:DB$ DealDamage | Defined$ TriggeredBlockerController | NumDmg$ 2\n\
         Oracle:Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature and 2 damage to its controller.",
    );
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].contains("Trigger::BlocksOrBecomesBlockedBy("));
    assert!(
        body.abilities[0].contains(
            "Effect::DamagePermanentAndController { target: TargetSpec::EventObject, permanent_amount: Amount::Fixed(1), controller_amount: Amount::Fixed(2) }"
        )
    );
}

#[test]
fn wave8_svar_conditions_opponent_counts_and_commander() {
    let conds = read(
        "Name:TaxScale\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | CheckSVar$ Start | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 1 | CheckSVar$ Lib | SVarCompare$ LE20\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Flying | CheckSVar$ OppL | SVarCompare$ GE4\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Vigilance | CheckSVar$ OppL | SVarCompare$ GTYours\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Lifelink | CheckSVar$ OppL | SVarCompare$ GEPlus2\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Menace | CheckSVar$ LowC | SVarCompare$ EQ0\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Haste | CheckSVar$ Life | SVarCompare$ LTBase\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Reach | CheckSVar$ Yours | SVarCompare$ LTOppL\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Trample | CheckSVar$ OppGy | SVarCompare$ LE6\n\
         SVar:Start:Count$StartingPlayer.0.1\n\
         SVar:Lib:PlayerCountPlayers$LowestValidLibrary Card.YouOwn\n\
         SVar:OppL:PlayerCountOpponents$HighestValid Land.YouCtrl\n\
         SVar:Yours:Count$Valid Land.YouCtrl\n\
         SVar:Plus2:Count$Valid Land.YouCtrl/Plus.2\n\
         SVar:LowC:PlayerCountOpponents$LowestValid Creature.YouCtrl\n\
         SVar:Life:Count$YourLifeTotal\n\
         SVar:Base:Count$YourStartingLife\n\
         SVar:OppGy:PlayerCountOpponents$HighestCardsInGraveyard",
    );
    assert!(conds.abilities[0].contains("Condition::NotStartingPlayer"));
    assert!(conds.abilities[1].contains("Condition::AnyLibraryAtMost(20)"));
    assert!(conds.abilities[2].contains("Condition::OpponentControlCount(&CHECK"));
    assert!(conds.abilities[2].contains(", 4)"));
    assert!(conds.statics.contains("Filter::ControlledByOpponent"));
    assert!(conds.abilities[3].contains("Condition::OpponentControlsMore(&Filter::LAND)"));
    assert!(conds.abilities[4].contains("Condition::OpponentControlsMoreBy(&Filter::LAND, 2)"));
    assert!(
        conds.abilities[5].contains("Condition::OpponentControlCountAtMost(&Filter::CREATURE, 0)")
    );
    assert!(conds.abilities[6].contains("Condition::LifeBelowStarting"));
    assert!(conds.abilities[7].contains("Condition::OpponentControlsMore(&Filter::LAND)"));
    assert!(
        conds.abilities[8].contains("Condition::Not(&Condition::OpponentGraveyardCountAtLeast(7))")
    );
}

#[test]
fn wave8_prevention_redirect_and_moved_enter_modifiers() {
    let martyr = read(
        "Name:ShieldVow\nManaCost:W W\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ Redir\n\
         SVar:Redir:Event$ DamageDone | ValidTarget$ Creature | Optional$ True | OptionalDecider$ You | DamageTarget$ You | ReplaceWith$ ToYou\n\
         SVar:ToYou:DB$ ReplaceEffect | VarName$ Affected | VarValue$ You | VarType$ Player",
    );
    assert!(martyr.abilities[0].contains("Modifier::MayRedirectDamageToYou"));
    assert!(martyr.abilities[0].contains("Duration::UntilEndOfTurn"));

    let self_shield = read(
        "Name:WardShell\nTypes:Creature\nPT:2/2\n\
         A:AB$ Effect | Cost$ 1 | ReplacementEffects$ Keep\n\
         SVar:Keep:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Artifact | ValidTarget$ Card.Self | Prevent$ True",
    );
    assert!(
        self_shield.abilities[0].contains(
            "PreventAllDamageFromMatchingThisTurn { target: Some(TargetSpec::Source), sources: &Filter::ARTIFACT, combat_only: false }"
        )
    );

    let shape = read(
        "Name:ClayForm\nManaCost:4\nTypes:Artifact Creature Shapeshifter\nPT:*/*\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | Layer$ Copy | ReplacementResult$ Updated | ReplaceWith$ Pick\n\
         SVar:Pick:DB$ GenericChoice | Defined$ You | Choices$ Flat,Wing,Wall\n\
         SVar:Flat:DB$ Clone | Defined$ Self | SetPower$ 3 | SetToughness$ 3\n\
         SVar:Wing:DB$ Clone | Defined$ Self | SetPower$ 2 | SetToughness$ 2 | AddKeywords$ Flying\n\
         SVar:Wall:DB$ Clone | Defined$ Self | SetPower$ 1 | SetToughness$ 6 | AddTypes$ Goblin | AddKeywords$ Defender",
    );
    assert!(shape.enter_modifiers[0].contains("EnterModifier::ChooseShape"));
    assert!(shape.enter_modifiers[0].contains("CopyMod::SetPT(3, 3)"));
    assert!(shape.enter_modifiers[0].contains("CopyMod::AddKeyword(KeywordSet::FLYING)"));
    assert!(shape.enter_modifiers[0].contains("CopyMod::AddSubtype(subtypes::creature::GOBLIN)"));

    let sac_others = read(
        "Name:QuietValley\nTypes:Land\n\
         R:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self | ReplaceWith$ Sac\n\
         SVar:Sac:DB$ SacrificeAll | ValidCards$ Permanent.YouCtrl+namedQuietValley | SubAbility$ Bf\n\
         SVar:Bf:DB$ ChangeZone | Origin$ All | Destination$ Battlefield | Defined$ ReplacedCard\n\
         A:AB$ Mana | Cost$ T | Produced$ C",
    );
    assert!(sac_others.enter_modifiers[0].contains("EnterModifier::SacrificeOthers(&ENTRY_SAC"));
    assert!(
        sac_others
            .statics
            .contains("Filter::Named(\"QuietValley\")")
    );

    let town = read(
        "Name:EarlyTown\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ Tap | ReplacementResult$ Updated\n\
         SVar:Tap:DB$ Tap | Defined$ Self | ETB$ True | ConditionCheckSVar$ X | ConditionSVarCompare$ GT3\n\
         SVar:X:Count$Compare Y GE1.Z.4\n\
         SVar:Y:PlayerCountPropertyYou$HasPropertyActive\n\
         SVar:Z:Count$YourTurns\n\
         A:AB$ Mana | Cost$ T | Produced$ C",
    );
    assert_eq!(
        town.enter_modifiers,
        ["EnterModifier::TappedUnlessCondition(Condition::DuringYourFirstTurns(3))"]
    );
}

#[test]
fn wave9_filter_atoms_and_graveyard_targets() {
    let filters = read(
        "Name:RelicLens\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Permanent.Historic+YouCtrl | AddKeyword$ Vigilance\n\
         S:Mode$ Continuous | Affected$ Creature.Outlaw+YouCtrl | AddPower$ 1\n\
         S:Mode$ Continuous | Affected$ Land.hasABasicLandType+YouCtrl | AddKeyword$ Indestructible\n\
         S:Mode$ Continuous | Affected$ Creature.!attacking+YouCtrl | AddToughness$ 1\n\
         S:Mode$ Continuous | Affected$ Creature.!EnchantedBy+nonColorless | AddPower$ 1\n\
         S:Mode$ Continuous | Affected$ Creature.counters_EQ0_P1P1+YouCtrl | AddToughness$ 1\n\
         S:Mode$ Continuous | Affected$ Creature.counters_EQ2_CHARGE+YouCtrl | AddPower$ 1\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Creature.DefenderCtrl+YouDontOwn\n\
         A:AB$ Counter | Cost$ 1 U | TargetType$ Spell | ValidTgts$ Card.IsSingleTarget+AdventureCard+inZoneStack\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ Snatch\n\
         SVar:Snatch:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.DefenderCtrl",
    );
    assert!(filters.abilities[0].contains("Filter::Historic"));
    assert!(
        filters.abilities[1].contains("subtypes::creature::ASSASSIN")
            && filters.abilities[1].contains("subtypes::creature::WARLOCK")
    );
    assert!(
        filters.abilities[2].contains("subtypes::land::PLAINS")
            && filters.abilities[2].contains("subtypes::land::FOREST")
    );
    assert!(filters.abilities[3].contains("Filter::Not(&Filter::Attacking)"));
    assert!(
        filters.abilities[4].contains("Filter::Not(&Filter::AttachedToBySource)")
            && filters.abilities[4].contains("Filter::Not(&Filter::IsColorless)")
    );
    assert!(
        filters.abilities[5]
            .contains("Filter::Not(&Filter::CountersAtLeast(CounterKind::P1P1, 1))")
    );
    assert!(
        filters.abilities[6].contains("Filter::CountersAtLeast(CounterKind::Charge, 2)")
            && filters.abilities[6]
                .contains("Filter::Not(&Filter::CountersAtLeast(CounterKind::Charge, 3))")
    );
    assert!(
        filters
            .statics
            .contains("Filter::ControlledByDefendingPlayer")
            && filters.statics.contains("Filter::Not(&Filter::OwnedByYou)")
    );
    assert!(
        filters.statics.contains("Filter::WithSingleTarget")
            && filters.statics.contains("Filter::HasAdventure")
            && filters.statics.contains("Filter::InZone(ZoneRef::Stack)")
    );
    assert!(
        filters.abilities[9]
            .contains("TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::DefendingPlayer)")
    );
}

#[test]
fn wave9_mana_cmc_and_event_amounts() {
    let rainbow = read(
        "Name:ChromaWell\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Special EachColorAmong_Valid Permanent.YouCtrl",
    );
    assert!(rainbow.abilities[0].contains("Effect::ManaForColorsAmong(&Filter::ControlledByYou)"));

    let converge = read(
        "Name:PrismSweep\nManaCost:X W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Permanent.nonLand+cmcLEX\n\
         SVar:X:Count$Converge",
    );
    assert!(converge.statics.contains("Filter::CmcAtMostColorsSpent"));

    let pool_and_chalice = read(
        "Name:VaultGauge\nTypes:Artifact\n\
         A:AB$ Destroy | Cost$ T | ValidTgts$ Artifact.cmcEQY\n\
         T:Mode$ SpellCast | ValidCard$ Card.cmcEQX | TriggerZones$ Battlefield | Execute$ No\n\
         SVar:No:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:X:Count$CardCounters.CHARGE\n\
         SVar:Y:Count$ManaPool:All",
    );
    assert!(
        pool_and_chalice
            .statics
            .contains("Filter::CmcEqualsYourUnspentMana")
    );
    assert!(
        pool_and_chalice
            .statics
            .contains("Filter::CmcEqualsCountersOnSource(CounterKind::Charge)")
    );

    let event_amts = read(
        "Name:EchoBell\nTypes:Enchantment\n\
         T:Mode$ Discarded | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ Burn\n\
         T:Mode$ Taps | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ Heal\n\
         A:AB$ GainLife | Cost$ T | Defined$ You | LifeAmount$ LandTypes\n\
         SVar:Burn:DB$ LoseLife | Defined$ You | LifeAmount$ Mv\n\
         SVar:Mv:TriggeredCard$CardManaCost\n\
         SVar:Heal:DB$ GainLife | Defined$ You | LifeAmount$ Pw\n\
         SVar:Pw:TriggeredCard$CardPower\n\
         SVar:LandTypes:Count$Valid Land.YouCtrl$BasicLandTypes",
    );
    assert!(event_amts.abilities[0].contains("Amount::EventManaValue"));
    assert!(event_amts.abilities[1].contains("Amount::EventPower"));
    assert!(event_amts.abilities[2].contains("Amount::BasicLandTypesAmong(&"));
}

#[test]
fn wave9_svar_flags_prevention_and_replacements() {
    let flags = read(
        "Name:OmenStone\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | CheckSVar$ B | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 1 | CheckSVar$ P | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Flying | CheckSVar$ NK | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Vigilance | CheckSVar$ FH | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Menace | CheckSVar$ Del | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Reach | CheckSVar$ Odd | SVarCompare$ EQ1\n\
         SVar:B:Count$StartingPlayer.0.1\n\
         SVar:P:Count$Metalcraft.1.0\n\
         SVar:NK:Count$Kicked.0.1\n\
         SVar:FH:Count$wasCastFromYourHandByYou.1.0\n\
         SVar:Del:Count$Delirium.1.0\n\
         SVar:Odd:Count$YourLifeTotal/Mod.2",
    );
    assert!(flags.abilities[0].contains("Condition::NotStartingPlayer"));
    assert!(flags.abilities[1].contains("Condition::ControlCount(&Filter::ARTIFACT, 3)"));
    assert!(
        flags.abilities[2]
            .contains("Condition::Not(&Condition::AmountAtLeast(Amount::TimesKicked, 1))")
    );
    assert!(flags.abilities[3].contains("Condition::SourceWasCastByYou"));
    assert!(flags.abilities[3].contains("Filter::CastFromHand"));
    assert!(flags.abilities[4].contains("Condition::GraveyardTypesAtLeast(4)"));
    assert!(
        flags.abilities[5].contains(
            "Condition::Not(&Condition::AmountIsEven(Amount::LifeTotal(PlayerRel::You)))"
        )
    );

    let prev = read(
        "Name:AegisPrism\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ 1 | ValidTgts$ Any | RememberObjects$ Targeted | ExileOnMoved$ Battlefield | ReplacementEffects$ Shield\n\
         A:AB$ Effect | Cost$ 2 | RememberObjects$ Self | ReplacementEffects$ SelfShield\n\
         A:AB$ Effect | Cost$ 3 | ValidTgts$ Land | RememberObjects$ Targeted | ForgetOnMoved$ Battlefield | ReplacementEffects$ LandSave\n\
         SVar:Shield:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Artifact | ValidTarget$ Player.IsRemembered,Card.IsRemembered | Prevent$ True\n\
         SVar:SelfShield:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Artifact | ValidTarget$ Card.IsRemembered | Prevent$ True\n\
         SVar:LandSave:Event$ Destroy | ValidCard$ Land.IsRemembered | ReplaceWith$ ClearDmg\n\
         SVar:ClearDmg:DB$ HealDamage | Defined$ ReplacedCard | SubAbility$ ExSelf\n\
         SVar:ExSelf:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile",
    );
    assert!(prev.abilities[0].contains("target: Some(TargetSpec::AnyTarget)"));
    assert!(prev.abilities[1].contains("target: Some(TargetSpec::Source)"));
    assert!(prev.abilities[2].contains("Modifier::NextDestructionClearsDamage"));

    let repl = read(
        "Name:VaultRules\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.YouCtrl | ValidCause$ SpellAbility.OppCtrl | ReplaceWith$ Rev | Optional$ True | EffectOnly$ True | Discard$ True\n\
         SVar:Rev:DB$ Reveal | Defined$ You | RevealDefined$ ReplacedCard | SubAbility$ Top\n\
         SVar:Top:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Library | NoShuffle$ True | Defined$ ReplacedCard\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Battlefield | ValidCard$ Land.OppCtrl+ControllerControlsAtLeastAsManyLand | ReplaceWith$ EqMove\n\
         SVar:EqMove:DB$ ChangeZone | Origin$ All | Destination$ Battlefield | Defined$ ReplacedCard | SubAbility$ EqSac\n\
         SVar:EqSac:DB$ Sacrifice | SacValid$ Land | Defined$ ReplacedCardController\n\
         R:Event$ Explore | ActiveZones$ Battlefield | ValidExplorer$ Creature.YouCtrl | ReplaceWith$ ScryFirst\n\
         SVar:ScryFirst:DB$ Scry | SubAbility$ ThenExp\n\
         SVar:ThenExp:DB$ Explore | Defined$ ReplacedCard",
    );
    assert_eq!(
        repl.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::MayRevealOpponentDiscardToLibraryTop)",
            "AbilityDef::Replacement(ReplacementRule::LandEquilibrium)",
            "AbilityDef::Replacement(ReplacementRule::ExploreScry1First)",
        ]
    );
}

#[test]
fn wave10c_exert_optional_attack_cost_ward_grant_and_ai_svar() {
    let bare = read(
        "Name:TirelessScout\nTypes:Creature\nPT:2/2\n\
         S:Mode$ OptionalAttackCost | ValidCard$ Card.Self | Cost$ Exert<1/CARDNAME>",
    );
    assert_eq!(
        bare.abilities,
        ["triggered!(Trigger::Attacks(&Filter::This), \
             &[Effect::MayDo { effects: &[Effect::ExertSource] }])"]
    );

    let reflexive = read(
        "Name:FlameStriker\nTypes:Creature\nPT:4/4\n\
         S:Mode$ OptionalAttackCost | ValidCard$ Card.Self | Trigger$ Burn | Cost$ Exert<1/CARDNAME>\n\
         SVar:Burn:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 3",
    );
    assert!(reflexive.abilities[0].contains("Effect::MayDo { effects: &[Effect::ExertSource] }"));
    assert!(
        reflexive.abilities[0].contains("Effect::Reflexive { when: ReflexiveEvent::ExertedThis")
    );
    assert!(reflexive.abilities[0].contains("target: Some(TargetSpec::Object(&Filter::CREATURE))"));

    assert!(refused(
        "Name:ExtraGuard\nTypes:Creature\nPT:4/1\n\
         S:Mode$ OptionalAttackCost | ValidCard$ Card.Self | IsPresent$ Creature.Self | Cost$ Exert<1/CARDNAME>"
    ));

    let ward_grant = read(
        "Name:AegisBanner\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Ward:2 & Ward:PayLife<3>",
    );
    assert!(ward_grant.abilities[0].contains("Modifier::GrantTriggered { trigger: Trigger::Ward"));
    assert!(ward_grant.abilities[0].contains("mana: Amount::Fixed(2)"));
    assert!(ward_grant.abilities[1].contains("Effect::PlayerMayPayLifeOr"));

    let ai_hint = read(
        "Name:ShadowCloak\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ 2 | AddSVar$ AE\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddSVar$ AE\n\
         SVar:AE:SVar:HasAttackEffect:TRUE",
    );
    assert_eq!(ai_hint.abilities.len(), 2);
    assert!(ai_hint.abilities[1].contains("Modifier::ModifyPT(2, 0)"));

    assert!(refused(
        "Name:BadHint\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddSVar$ RealRule\n\
         SVar:RealRule:Count$Valid Creature.YouCtrl"
    ));
}

#[test]
fn wave10c_ignore_landwalk_min_blockers_flash_and_cast_self() {
    let walks = read(
        "Name:FlatPlains\nTypes:Enchantment\n\
         S:Mode$ IgnoreLandWalk\n\
         S:Mode$ IgnoreLandWalk | ValidKeyword$ Landwalk:Island\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Creature.YouCtrl | Min$ 3\n\
         S:Mode$ CastWithFlash | Phases$ End of Turn | PlayerTurn$ Opponent | ValidCard$ Card | ValidSA$ Spell | Caster$ You",
    );
    assert_eq!(
        walks.abilities,
        [
            "static_ability!(Filter::CREATURE, Modifier::BlockableWithoutAnyLandwalk)",
            "static_ability!(Filter::CREATURE, Modifier::BlockableWithoutLandwalk(KeywordSet::ISLANDWALK))",
            "static_ability!(Filter::YOUR_CREATURE, Modifier::MinimumBlockers(3))",
            "static_ability!(Filter::Any, Modifier::SpellsHaveFlash(None), \
             condition = Some(Condition::All(&[Condition::OpponentsTurn, Condition::DuringStep(StepKind::End)])))",
        ]
    );
    assert!(refused(
        "Name:BlockerOnly\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ IgnoreLandWalk | ValidBlocker$ Creature.EnchantedBy"
    ));
    let max_one = read(
        "Name:MaxOne\nTypes:Creature\nPT:4/4\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Card.Self | Max$ 1",
    );
    assert_eq!(
        max_one.abilities,
        ["static_ability!(Filter::This, Modifier::MaxBlockers(1))"]
    );

    let cast_etb = read(
        "Name:CastTitan\nTypes:Artifact Creature\nPT:7/7\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self+wasCastByYou | Destination$ Battlefield | Execute$ Sweep\n\
         SVar:Sweep:DB$ DestroyAll | ValidCards$ Creature.nonArtifact",
    );
    assert!(cast_etb.abilities[0].starts_with("triggered!(Trigger::ETB, "));
    assert!(cast_etb.abilities[0].contains("Effect::destroy_all("));
    assert!(cast_etb.abilities[0].contains("condition = Some(Condition::SourceWasCastByYou)"));
}

#[test]
fn wave10c_spellcast_hand_atoms_svar_conditions_and_divide() {
    let paradox = read(
        "Name:RiftWatcher\nTypes:Creature\nPT:3/3\n\
         T:Mode$ SpellCast | ValidCard$ Card.!wasCastFromYourHand | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ Ping\n\
         T:Mode$ SpellCast | ValidCard$ Instant.wasCastFromYourHandByYou | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ Ping\n\
         SVar:Ping:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        paradox
            .statics
            .contains("Filter::Not(&Filter::CastFromHand)")
    );
    assert!(paradox.statics.contains("Filter::CastFromHand"));
    assert!(refused(
        "Name:BadOriginDiscount\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card.!wasCastFromYourHand | Activator$ You | Type$ Spell | Amount$ 1"
    ));

    let div_pt = read(
        "Name:GraveScale\nTypes:Creature\nPT:1/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X\n\
         SVar:X:SVar$Y/Times.2\n\
         SVar:Y:Count$ValidGraveyard Card.YouOwn/DivideEvenlyDown.7",
    );
    assert!(div_pt.abilities[0].contains("Modifier::ModifyPTByAmount"));
    assert!(div_pt.abilities[0].contains("Amount::divided("));
    assert!(div_pt.abilities[0].contains("factor: 2"));

    let svars = read(
        "Name:ConditionHub\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | CheckSVar$ Cast | SVarCompare$ LT1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 1 | CheckSVar$ Lands | SVarCompare$ EQ0\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Flying | CheckSVar$ Pois | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Vigilance | CheckSVar$ Desc | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Menace | CheckSVar$ OddC | SVarCompare$ EQ1\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ PlayerCountPropertyYou$HasPropertyActive | SVarCompare$ GE1 | Execute$ Heal\n\
         SVar:Cast:Count$ThisTurnCast_Card.YouCtrl\n\
         SVar:Lands:PlayerCountPropertyYou$LandsPlayed\n\
         SVar:Pois:Count$TotalOppPoisonCounters\n\
         SVar:Desc:Count$ThisTurnEntered_Graveyard_from_Any_Permanent.YouOwn+!token\n\
         SVar:OddC:Count$CardCounters.ALL/Mod.2\n\
         SVar:Heal:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(svars.abilities[0].contains("Condition::YouCastNoSpellThisTurn"));
    assert!(
        svars.abilities[1].contains("Condition::Not(&Condition::LandsPlayedThisTurnAtLeast(1))")
    );
    assert!(svars.abilities[2].contains("Condition::OpponentPoisonAtLeast(1)"));
    assert!(svars.abilities[3].contains("Condition::DescendedThisTurn"));
    assert!(svars.abilities[4].contains("Condition::Not(&Condition::EvenCountersOnSelf)"));
    assert!(svars.abilities[5].contains("Condition::YourTurn"));
    assert!(refused(
        "Name:BadMultiPoison\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | CheckSVar$ P | SVarCompare$ GE2\n\
         SVar:P:Count$TotalOppPoisonCounters"
    ));
}

#[test]
fn wave11c_oracle_scope_corrections_for_named_and_commander_filters() {
    let pack = read(
        "Name:PackHound\nTypes:Creature Beast\nPT:3/3\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Destination$ Battlefield | Execute$ Buff\n\
         SVar:Buff:DB$ PutCounterAll | ValidCards$ Creature.StrictlyOther+namedPackHound | CounterType$ P1P1 | CounterNum$ 2\n\
         Oracle:When PackHound enters, put two +1/+1 counters on each other creature you control named PackHound.",
    );
    assert!(pack.statics.contains("Filter::Named(\"PackHound\")"));
    assert!(pack.statics.contains("Filter::ControlledByYou"));

    let global_named = read(
        "Name:TwinHound\nTypes:Creature Beast\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.Other+namedTwinHound | AddPower$ 1\n\
         Oracle:Each other creature named TwinHound gets +1/+0.",
    );
    assert!(global_named.abilities[0].contains("Filter::Named(\"TwinHound\")"));
    assert!(!global_named.abilities[0].contains("Filter::ControlledByYou"));

    let choir = read(
        "Name:ChoirAngel\nTypes:Creature Angel\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | ValidCard$ Card.IsCommander+YouOwn | Execute$ Grow\n\
         T:Mode$ Attacks | ValidCard$ Card.IsCommander+YouOwn | TriggerZones$ Battlefield | Execute$ Grow | Secondary$ True\n\
         SVar:Grow:DB$ Proliferate\n\
         Oracle:Whenever a commander you control enters or attacks, proliferate.",
    );
    assert!(choir.statics.contains("Filter::IsCommander"));
    assert!(choir.statics.contains("Filter::ControlledByYou"));
    assert!(!choir.statics.contains("Filter::OwnedByYou"));

    let owned_cmd = read(
        "Name:CommandTome\nTypes:Artifact\n\
         T:Mode$ Attacks | ValidCard$ Card.IsCommander+YouOwn | TriggerZones$ Battlefield | Execute$ Grow\n\
         SVar:Grow:DB$ Proliferate\n\
         Oracle:Whenever your commander attacks, proliferate.",
    );
    assert!(owned_cmd.statics.contains("Filter::OwnedByYou"));
    assert!(!owned_cmd.statics.contains("Filter::ControlledByYou"));
}

#[test]
fn wave11c_enchanted_player_filters_relations_statics_and_amounts() {
    let curse = read(
        "Name:HexOfSilence\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedPlayerCtrl | AddPower$ -1 | AddToughness$ -1\n\
         S:Mode$ MustAttack | ValidCreature$ Creature.EnchantedPlayerCtrl\n\
         S:Mode$ CantBeCast | ValidCard$ Card | Caster$ Player.EnchantedBy | NumLimitEachTurn$ 1\n\
         S:Mode$ CantBeCast | ValidCard$ Instant | Caster$ Player.EnchantedBy\n\
         S:Mode$ RaiseCost | EffectZone$ Battlefield | ValidCard$ Card.NamedCard | Type$ Spell | Activator$ Player.EnchantedBy | Amount$ 2\n\
         S:Mode$ ReduceCost | ValidCard$ Creature | Type$ Spell | Activator$ Player.EnchantedBy | Amount$ 1\n\
         S:Mode$ CantDraw | ValidPlayer$ Player.EnchantedBy | DrawLimit$ 1\n\
         S:Mode$ CantLoseLife | ValidPlayer$ Player.EnchantedBy\n\
         S:Mode$ Continuous | Affected$ Player.EnchantedBy | AddKeyword$ UntapAdjust:Creature:1\n\
         S:Mode$ Continuous | Affected$ Player.EnchantedBy | AddKeyword$ Hexproof\n\
         R:Event$ Moved | ValidCard$ Creature.EnchantedPlayerCtrl | Destination$ Battlefield | ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ TapIn\n\
         R:Event$ GainLife | ActiveZones$ Battlefield | ValidPlayer$ Player.EnchantedBy | Prevent$ True\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Draw | ValidPlayer$ Player.EnchantedBy | Skip$ True\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Player.EnchantedBy | DamageAmount$ GE3 | ReplaceWith$ Cap2\n\
         A:AB$ LoseLife | Cost$ T | Defined$ EnchantedPlayer | LifeAmount$ X\n\
         SVar:TapIn:DB$ Tap | Defined$ ReplacedCard | ETB$ True\n\
         SVar:Cap2:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ 2\n\
         SVar:X:EnchantedPlayer$CardsInHand",
    );
    let rendered = format!("{} {}", curse.statics, curse.abilities.join("\n"));
    assert!(rendered.contains("Filter::ControlledByEnchantedPlayer"));
    assert!(rendered.contains("Modifier::AttacksEachCombat"));
    assert!(
        rendered.contains(
            "Modifier::SpellCastLimitPerTurn { who: PlayerRel::EnchantedPlayer, limit: 1 }"
        )
    );
    assert!(rendered.contains("Modifier::PlayersCantCast { who: PlayerRel::EnchantedPlayer,"));
    assert!(rendered.contains("Modifier::SpellsCostMore(2)"));
    assert!(rendered.contains("Modifier::SpellsCostLess(1)"));
    assert!(
        rendered
            .contains("Modifier::DrawLimitPerTurn { who: PlayerRel::EnchantedPlayer, limit: 1 }")
    );
    assert!(rendered.contains("Modifier::CantLoseLife { who: PlayerRel::EnchantedPlayer }"));
    assert!(rendered.contains("Modifier::UntapAtMost { who: PlayerRel::EnchantedPlayer,"));
    assert!(rendered.contains("Modifier::PlayerHexproof { who: PlayerRel::EnchantedPlayer }"));
    assert!(rendered.contains("ReplacementRule::EnterTapped("));
    assert!(
        rendered.contains("Modifier::LifeGainNoLifeInstead { who: PlayerRel::EnchantedPlayer }")
    );
    assert!(rendered.contains("Modifier::SkipDrawStep { who: PlayerRel::EnchantedPlayer }"));
    assert!(rendered.contains("recipients: PlayerRel::EnchantedPlayer"));
    assert!(rendered.contains("Amount::HandSize(PlayerRel::EnchantedPlayer)"));
}

#[test]
fn wave11c_counter_thresholds_spell_limits_and_static_extras() {
    let limits = read(
        "Name:LawCensor\nTypes:Enchantment\n\
         S:Mode$ CantBeCast | ValidCard$ Card.nonCreature | Caster$ Player | NumLimitEachTurn$ 1\n\
         S:Mode$ CantBeCast | ValidCard$ Card.nonArtifact | Caster$ Player | NumLimitEachTurn$ 1\n\
         S:Mode$ CantBeCast | ValidCard$ Card | Caster$ You | NumLimitEachTurn$ 1\n\
         S:Mode$ CantBeCast | NumLimitEachTurn$ 2 | Caster$ You | Secondary$ True\n\
         S:Mode$ CantBeCast | EffectZone$ Command | ValidCard$ Card | Caster$ Opponent | NumLimitEachTurn$ 1\n\
         S:Mode$ CantBeCast | Caster$ You.!Active\n\
         S:Mode$ CantBeCast | ValidCard$ Card | Condition$ PlayerTurn | Caster$ Opponent | IsPresent$ Card.Self+AttachedTo Creature\n\
         S:Mode$ RaiseCost | ValidCard$ Enchantment.White | Type$ Ability | Amount$ 3 | AffectedZone$ Battlefield",
    );
    assert!(limits.abilities[0].contains(
        "Modifier::MatchingSpellCastLimitPerTurn { who: PlayerRel::EachPlayer, filter: &Filter::NONCREATURE, limit: 1 }"
    ));
    assert!(
        limits.abilities[1]
            .contains("Modifier::MatchingSpellCastLimitPerTurn { who: PlayerRel::EachPlayer,")
    );
    assert!(
        limits.abilities[2]
            .contains("Modifier::SpellCastLimitPerTurn { who: PlayerRel::You, limit: 1 }")
    );
    assert!(
        limits.abilities[3]
            .contains("Modifier::SpellCastLimitPerTurn { who: PlayerRel::You, limit: 2 }")
    );
    assert!(limits.abilities[4].contains(
        "Modifier::SpellCastLimitPerTurn { who: PlayerRel::Opponent, limit: 1 }, zone = ZoneRef::Command"
    ));
    assert!(limits.abilities[5].contains(
        "Modifier::PlayersCantCast { who: PlayerRel::You, filter: &Filter::Any }, condition = Some(Condition::Not(&Condition::YourTurn))"
    ));
    assert!(limits.abilities[6].contains("Modifier::OpponentsCantCast(&Filter::Any)"));
    assert!(limits.abilities[6].contains("Condition::YourTurn"));
    assert!(limits.abilities[6].contains("Condition::SourceMatches(&"));
    assert!(
        limits
            .statics
            .contains("Filter::AttachedTo(&Filter::CREATURE)")
    );
    assert!(limits.abilities[7].contains("Modifier::AbilitiesCostMore(3)"));

    let counters = read(
        "Name:CounterVault\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.counters_GE3_ALL+YouCtrl | AddPower$ 2\n\
         S:Mode$ Continuous | Affected$ Creature.counters_LE2+YouCtrl | AddToughness$ 1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | CheckSVar$ Total | SVarCompare$ GT4 | AddKeyword$ Flying\n\
         S:Mode$ Continuous | AffectedDefined$ Self | CheckSVar$ HandN | SVarCompare$ NE3 | AddKeyword$ Vigilance\n\
         S:Mode$ CantBeCast | ValidCard$ Card.Self | EffectZone$ All | Caster$ Player.Active | CheckSVar$ Turns | SVarCompare$ LE3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | IsPresent$ Card.StrictlySelf | Execute$ Ping\n\
         SVar:Total:Count$CardCounters.ALL\n\
         SVar:HandN:Count$ValidHand Card.YouOwn\n\
         SVar:Turns:Count$YourTurns\n\
         SVar:Ping:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(counters.abilities[0].contains("Filter::AnyCountersAtLeast(3)"));
    assert!(counters.abilities[1].contains("Filter::Not(&Filter::AnyCountersAtLeast(3))"));
    assert!(
        counters.abilities[2].contains("Condition::SourceMatches(&Filter::AnyCountersAtLeast(5))")
    );
    assert!(counters.abilities[3].contains("Condition::AmountCompare { left: &Amount::HandSize(PlayerRel::You), op: AmountCmp::NotEqual, right: &Amount::Fixed(3) }"));
    assert!(counters.abilities[4].contains("Condition::Not(&Condition::DuringYourFirstTurns(3))"));
    assert!(counters.abilities[5].contains("Condition::BattlefieldCount(&Filter::This, 1)"));

    // Negative refusal boundaries remain intact.
    assert!(refused(
        "Name:BadLimitCond\nTypes:Enchantment\n\
         S:Mode$ CantBeCast | ValidCard$ Card.nonCreature | Caster$ Player | NumLimitEachTurn$ 1 | Condition$ Unsupported"
    ));
    assert!(refused(
        "Name:BadZeroLimit\nTypes:Enchantment\n\
         S:Mode$ CantBeCast | ValidCard$ Card.nonCreature | Caster$ Player | NumLimitEachTurn$ 0"
    ));
    assert!(refused(
        "Name:BadAbilityTaxZone\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidCard$ Enchantment.White | Type$ Ability | Amount$ 3"
    ));
}

#[test]
fn wave12b_mana_emblem_and_graveyard_counts() {
    let mana = read(
        "Name:PersistentConduit\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ R G | PersistentMana$ True\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ Spell | PersistentMana$ True\n\
         A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ nonSpell\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 2 | RestrictValid$ Spell.Creature+Colorless,Activated.Creature+Colorless\n\
         A:AB$ Mana | Cost$ T | Produced$ U | RestrictValid$ Spell.Artifact,Activated\n\
         A:AB$ Draw | Cost$ T | NumCards$ X\n\
         A:AB$ Draw | Cost$ T | NumCards$ Y\n\
         SVar:X:Count$ValidBattlefield,Graveyard Creature.YouCtrl\n\
         SVar:Y:Count$ValidBattlefield,Graveyard Goblin",
    );
    assert!(
        mana.abilities[0].contains("Effect::mana(ManaColor::Red, 1).mana_until_end_of_turn()")
            && mana.abilities[0]
                .contains("Effect::mana(ManaColor::Green, 1).mana_until_end_of_turn()")
    );
    assert!(
        mana.abilities[1]
            .contains(".restricted(&Filter::Any, SpendRider::None).mana_until_end_of_turn()")
    );
    assert!(
        mana.abilities[2].contains("ManaUse::NonSpellPaymentsOrMatchingSpells")
            && mana.statics.contains("Filter::Not(&Filter::Any)")
    );
    assert!(mana.abilities[3].contains("ManaUse::SpellsOrAbilities"));
    assert!(
        mana.abilities[4].contains("ManaUse::SpellsOrAbilities")
            && mana.abilities[4].contains(".for_abilities_matching(&Filter::Any)")
    );
    assert!(
        mana.abilities[5].contains("Amount::Sum")
            && mana.abilities[5].contains("ZoneSel::Battlefield")
            && mana.abilities[5].contains("ZoneSel::GraveyardOf(PlayerRel::You)")
    );
    assert!(
        mana.abilities[6].contains("Amount::Sum")
            && mana.abilities[6].contains("ZoneSel::GraveyardAll")
    );

    let aura_gy = read(
        "Name:AuraMillCount\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ Drain | TriggerZones$ Battlefield\n\
         SVar:Drain:DB$ LoseLife | Defined$ EnchantedController | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Card.EnchantedControllerOwn",
    );
    assert!(
        aura_gy
            .abilities
            .iter()
            .any(|a| a.contains("ZoneSel::GraveyardOf(PlayerRel::ControllerOfAttached)"))
    );

    let emblem_pair = read(
        "Name:EmblemShield\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.OppCtrl,Emblem.OppCtrl | ValidTarget$ You | Prevent$ True\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.nonCreature,Emblem | ValidTarget$ You | Prevent$ True",
    );
    let emblem_text = format!(
        "{}\n{}",
        emblem_pair.statics,
        emblem_pair.abilities.join("\n")
    );
    assert!(emblem_text.contains("Filter::ControlledByOpponent"));
    assert!(emblem_text.contains("Filter::NONCREATURE"));

    let tokens = read(
        "Name:TokenMirror\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ReplaceWith$ Dbl\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Creature | ReplaceWith$ Tri\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Card.OppCtrl | ReplaceWith$ Dbl\n\
         SVar:Dbl:DB$ ReplaceToken | Type$ Amount | Amount$ Twice\n\
         SVar:Tri:DB$ ReplaceToken | Type$ Amount | Amount$ 3",
    );
    assert!(
        tokens.abilities[0].contains("DoubleTokenCreation { controller_filter: &Filter::Any }")
    );
    assert!(tokens.abilities[1].contains(
        "MultiplyTypedTokens { controller_filter: &Filter::Any, types: TypeSet::CREATURE, factor: 3 }"
    ));
    assert!(
        tokens.abilities[2]
            .contains("DoubleTokenCreation { controller_filter: &Filter::ControlledByOpponent }")
    );
}

#[test]
fn wave12b_cost_targets_grants_and_graveyard() {
    let tax_and_grant = read(
        "Name:SkyWarden\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ RaiseCost | ValidTarget$ Card.Self | Activator$ Opponent | Type$ Spell | Amount$ 2\n\
         S:Mode$ RaiseCost | ValidTarget$ Permanent.YouCtrl+inZoneBattlefield | Activator$ Player.Opponent | Type$ Spell | Amount$ 1\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | Activator$ Player.Opponent | Type$ Spell | Amount$ 1\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Spell.Buyback | Activator$ Player | Amount$ 2\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddStaticAbility$ GrantedTax\n\
         S:Mode$ Activations | ValidCard$ Creature.YouCtrl | ValidSA$ Activated.Boast | ValidPlayer$ You | MinLimit$ 2 | PlayerTurn$ You\n\
         S:Mode$ Continuous | Affected$ Player.Opponent | AddKeyword$ LimitSearchLibrary\n\
         SVar:GrantedTax:Mode$ RaiseCost | ValidTarget$ Card.Self | Activator$ Player.Opponent | Type$ Spell | Amount$ 2",
    );
    assert!(
        tax_and_grant.abilities[0].contains("Filter::SpellTargetingAffected")
            && tax_and_grant.abilities[0].contains("Modifier::SpellsCostMore(2)")
    );
    assert!(
        tax_and_grant.abilities[1].contains("Filter::TargetsPermanentYouControl")
            && tax_and_grant.abilities[1].contains("Modifier::SpellsCostMore(1)")
    );
    assert!(
        tax_and_grant.abilities[2].contains("Filter::SpellTargetingAffected")
            && tax_and_grant.abilities[2].contains("Modifier::SpellsCostLess(1)")
    );
    assert!(tax_and_grant.abilities[3].contains("Modifier::BuybackCostsLess(2)"));
    assert!(
        tax_and_grant.abilities[4].contains("Modifier::GrantStatic { filter: &GRANTED_STATIC")
            && tax_and_grant.abilities[4].contains("modifier: &Modifier::SpellsCostMore(2)")
    );
    assert!(tax_and_grant.abilities[5].contains("Modifier::BoastTwice"));
    assert!(tax_and_grant.abilities[6].contains("Modifier::OpponentsSearchTop(4)"));

    let stack_tax = read(
        "Name:StackTorch\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         S:Mode$ RaiseCost | ValidTarget$ Spell.Self | Activator$ Player | Type$ Spell | Amount$ 2 | EffectZone$ Stack\n\
         SVar:X:Count$xPaid",
    );
    assert!(stack_tax.abilities[1].contains(
        "static_ability!(Filter::SpellTargetingAffected, Modifier::SpellsCostMore(2), zone = ZoneRef::Stack)"
    ));

    let graveyard_rules = read(
        "Name:GraveKeeper\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | IsPresent$ Goblin.YouCtrl\n\
         S:Mode$ Continuous | Affected$ Instant.YouOwn,Sorcery.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Flashback | Condition$ PlayerTurn\n\
         S:Mode$ Continuous | Affected$ Card.YouOwn+nonLand | AffectedZone$ Graveyard | AddKeyword$ Escape:CardManaCost ExileFromGrave<3/Card.Other/other>",
    );
    assert!(
        graveyard_rules.abilities[0].contains(
            "static_ability!(Filter::This, Modifier::CastSpellsFromGraveyard, zone = ZoneRef::Graveyard, condition = Some("
        )
    );
    assert!(
        graveyard_rules.abilities[1].contains("Modifier::GrantsFlashback")
            && graveyard_rules.abilities[1].contains("Condition::YourTurn")
    );
    assert!(graveyard_rules.abilities[2].contains("Modifier::GrantsEscape(3)"));
}

#[test]
fn wave12b_refusal_boundaries() {
    for script in [
        "Name:BadPersistOther\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | Execute$ Give | TriggerZones$ Battlefield\n\
         SVar:Give:DB$ Mana | Defined$ TriggeredPlayer | Produced$ R | PersistentMana$ True",
        "Name:BadEmblemAlone\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Emblem | ValidTarget$ You | Prevent$ True",
        "Name:BadEmblemMismatch\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.YouCtrl,Emblem.OppCtrl | ValidTarget$ You | Prevent$ True",
        "Name:BadRaiseLifeCost\nTypes:Creature\nPT:5/4\n\
         S:Mode$ RaiseCost | ValidTarget$ Card.Self | Activator$ Player.Opponent | Type$ Spell | Cost$ PayLife<3>",
        "Name:BadRaiseNonYouTarget\nTypes:Creature\nPT:2/2\n\
         S:Mode$ RaiseCost | ValidTarget$ Creature.OppCtrl | Activator$ Opponent | Type$ Spell | Amount$ 2",
        "Name:BadReduceColorTarget\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | Activator$ You | Type$ Spell | Color$ U | Amount$ 1 | IgnoreGeneric$ True",
        "Name:BadReduceAllZoneTarget\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | ValidTarget$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ 2",
        "Name:BadGrantConditional\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddStaticAbility$ CondStatic\n\
         SVar:CondStatic:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | IsPresent$ Swamp.YouCtrl",
        "Name:BadGrantNested\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddStaticAbility$ Outer\n\
         SVar:Outer:Mode$ Continuous | Affected$ Creature.YouCtrl | AddStaticAbility$ Inner\n\
         SVar:Inner:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1",
        "Name:BadFlashbackCost\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Instant.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Flashback:1 R",
        "Name:BadPowerUpActivations\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Activations | ValidCard$ Permanent.YouCtrl | ValidSA$ Activated.PowerUp | ValidPlayer$ You | MinLimit$ 2",
    ] {
        assert!(refused(script), "expected refusal for:\n{script}");
    }
}

#[test]
fn wave13b_targets_and_filters() {
    let player_targets = read(
        "Name:DuelBolt\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player.Opponent | NumDmg$ 2\n\
         A:SP$ DealDamage | ValidTgts$ Player.attackedWithCreaturesThisTurn | NumDmg$ 3\n\
         A:SP$ DealDamage | ValidTgts$ Player.Opponent,Card.Planeswalker | NumDmg$ 4\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Player,Planeswalker,Battle | NumDmg$ 1\n\
         A:SP$ DealDamage | ValidTgts$ Creature.Other,Player,Planeswalker.Other,Battle.Other | NumDmg$ 1",
    );
    assert!(
        player_targets.abilities[0].contains("TargetSpec::AnyOpponent"),
        "expected AnyOpponent: {}",
        player_targets.abilities[0]
    );
    assert!(
        player_targets.abilities[1].contains("TargetSpec::PlayerWhoAttackedThisTurn"),
        "expected PlayerWhoAttackedThisTurn: {}",
        player_targets.abilities[1]
    );
    assert!(
        player_targets.abilities[2].contains("TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)"),
        "expected OpponentOrObject(&PLANESWALKER): {} / {}",
        player_targets.abilities[2],
        player_targets.statics
    );
    assert!(
        player_targets.abilities[3].contains("TargetSpec::AnyTarget"),
        "expected AnyTarget: {}",
        player_targets.abilities[3]
    );
    assert!(
        player_targets.abilities[4].contains("TargetSpec::PlayerOrObject(&TARGET"),
        "expected PlayerOrObject: {}",
        player_targets.abilities[4]
    );

    let opp_oracle = read(
        "Name:FoeStrike\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 2\n\
         Oracle:FoeStrike deals 2 damage to target opponent or planeswalker.",
    );
    assert!(
        opp_oracle.abilities[0].contains("TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)"),
        "expected OpponentOrObject from oracle: {}",
        opp_oracle.abilities[0]
    );

    let filter_atoms = read(
        "Name:FilterSuite\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Land,Enchantment,Planeswalker,Battle | AddKeyword$ Indestructible\n\
         S:Mode$ Continuous | Affected$ Card.nonPlaneswalker+nonBattle+nonInstant+nonSorcery | AddKeyword$ Hexproof\n\
         S:Mode$ Continuous | Affected$ Creature.!firstTurnControlled | AddPower$ 1\n\
         S:Mode$ CantBeCast | ValidCard$ Card.sharesNameWith ExiledWith | Caster$ Opponent\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.ColorlessSource,Emblem.ColorlessSource | ValidTarget$ You | Prevent$ True",
    );
    let rendered = format!(
        "{}\n{}",
        filter_atoms.statics,
        filter_atoms.abilities.join("\n")
    );
    assert!(
        rendered.contains("Filter::LAND")
            && rendered.contains("Filter::ENCHANTMENT")
            && rendered.contains("Filter::PLANESWALKER")
            && rendered.contains("TypeSet::BATTLE"),
        "expected type filters: {rendered}"
    );
    assert!(
        rendered.contains("LacksType(TypeSet::PLANESWALKER)")
            && rendered.contains("LacksType(TypeSet::BATTLE)")
            && rendered.contains("LacksType(TypeSet::INSTANT)")
            && rendered.contains("LacksType(TypeSet::SORCERY)"),
        "expected negated type filters: {rendered}"
    );
    assert!(
        rendered.contains("Filter::ControlledSinceTurnBegan"),
        "expected ControlledSinceTurnBegan: {rendered}"
    );
    assert!(
        rendered.contains("Filter::SameNameAsLinkedExile"),
        "expected SameNameAsLinkedExile: {rendered}"
    );
    assert!(
        rendered.contains("PreventDamageToYouFrom(&Filter::IsColorless)"),
        "expected PreventDamageToYouFrom(&Filter::IsColorless): {rendered}"
    );

    let gy_entered = read(
        "Name:FreshGrave\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouOwn+ThisTurnEntered | TgtZone$ Graveyard\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Creature.ThisTurnEntered",
    );
    assert!(
        gy_entered
            .statics
            .contains("Filter::PutIntoGraveyardThisTurn"),
        "expected PutIntoGraveyardThisTurn: {}",
        gy_entered.statics
    );
}

#[test]
fn wave13b_mana_counts_and_pay_life_x() {
    let counts = read(
        "Name:CountSuite\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ A\n\
         A:SP$ Draw | ValidTgts$ Player | NumCards$ B\n\
         A:SP$ Mill | ValidTgts$ Player | NumCards$ C\n\
         A:SP$ GainLife | ValidTgts$ Creature | LifeAmount$ D\n\
         SVar:A:TargetedPlayer$Valid Artifact.tapped+YouCtrl\n\
         SVar:B:ThisTargetedPlayer$CardsInHand/HalfUp\n\
         SVar:C:TargetedPlayer$ValidGraveyard Creature.YouOwn\n\
         SVar:D:Targeted$CardSumPT",
    );
    assert!(
        counts.abilities[0].contains("Amount::ControlledCount")
            && counts.abilities[0].contains("PlayerRel::Chosen"),
        "expected ControlledCount(Chosen): {}",
        counts.abilities[0]
    );
    assert!(
        counts.abilities[1].contains(
            "Amount::Half { base: &Amount::HandSize(PlayerRel::Chosen), round_up: true }"
        ),
        "expected Half(HandSize(Chosen), round_up: true): {}",
        counts.abilities[1]
    );
    assert!(
        counts.abilities[2].contains("ZoneSel::GraveyardOf(PlayerRel::Chosen)"),
        "expected GraveyardOf(Chosen): {}",
        counts.abilities[2]
    );
    assert!(
        counts.abilities[3].contains(
            "Amount::Sum { left: &Amount::TargetPower, right: &Amount::TargetToughness }"
        ),
        "expected TargetPower + TargetToughness: {}",
        counts.abilities[3]
    );

    let aura_count = read(
        "Name:AuraCount\nManaCost:G\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ X\n\
         SVar:X:Count$EnchantedControllerCreatures",
    );
    assert!(
        aura_count.abilities.iter().any(
            |a| a.contains("PlayerRel::ControllerOfAttached") && a.contains("Filter::CREATURE")
        ),
        "expected ControllerOfAttached CREATURE: {:?}",
        aura_count.abilities
    );

    let pay_life_x = read(
        "Name:BloodPact\nManaCost:3 B\nTypes:Instant\n\
         A:SP$ Pump | Cost$ 3 B PayLife<X> Discard<0/Hand> | ValidTgts$ Creature | NumAtt$ +X\n\
         SVar:X:Count$xPaid\n\
         Oracle:As an additional cost to cast this spell, pay X life and discard your hand. Target creature gets +X/+0 until end of turn.",
    );
    assert!(
        pay_life_x
            .mandatory_additional_costs
            .iter()
            .any(|c| c == "CostPart::PayLifeX")
            && pay_life_x
                .mandatory_additional_costs
                .iter()
                .any(|c| c == "CostPart::DiscardHand"),
        "expected PayLifeX + DiscardHand: {:?}",
        pay_life_x.mandatory_additional_costs
    );
}

#[test]
fn wave13b_refusal_boundaries() {
    for script in [
        "Name:BadCounterMixed\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | ValidTgts$ Player,Creature | TargetType$ Spell",
        "Name:BadPutCounterMixed\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Player,Creature | CounterType$ P1P1 | CounterNum$ 1",
        "Name:BadPlayerScopeOpp\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ X\n\
         SVar:X:TargetedPlayer$Valid Creature.OppCtrl",
        "Name:BadGraveScopeOpp\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ X\n\
         SVar:X:TargetedPlayer$ValidGraveyard Creature.OppOwn",
        "Name:BadEnteredFromBf\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Creature.ThisTurnEnteredFrom_Battlefield",
        "Name:BadActivatedPayLifeX\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ PayLife<X> | NumCards$ 1\n\
         SVar:X:Count$xPaid",
        "Name:BadPayLifeXComputed\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Draw | Cost$ 1 B PayLife<X> Discard<0/Hand> | NumCards$ 1\n\
         SVar:X:Count$xPaid\n\
         Oracle:As an additional cost to cast this spell, pay X life and discard your hand, where X is the number of creatures you control.",
    ] {
        assert!(refused(script), "expected refusal for:\n{script}");
    }
}

#[test]
fn wave13b_player_rels_and_svar_depth() {
    let other_and_parent = read(
        "Name:SharedDrain\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | Defined$ Player.Other | LifeAmount$ C\n\
         A:SP$ Draw | ValidTgts$ Opponent | NumCards$ 2 | SubAbility$ DrainTarget\n\
         SVar:DrainTarget:DB$ LoseLife | Defined$ ParentTarget | LifeAmount$ 2\n\
         SVar:A:Count$Valid Creature.YouCtrl\n\
         SVar:B:SVar$A/Times.2\n\
         SVar:C:SVar$B/Plus.1",
    );
    assert!(
        other_and_parent.abilities[0].contains("PlayerRel::EachOtherPlayer")
            && other_and_parent.abilities[0].contains("Amount::Plus")
            && other_and_parent.abilities[0].contains("Amount::Times"),
        "expected EachOtherPlayer + nested arithmetic: {}",
        other_and_parent.abilities[0]
    );
    assert!(
        other_and_parent.abilities[1].contains("PlayerRel::Chosen")
            && other_and_parent.abilities[1].contains("TargetSpec::AnyOpponent"),
        "expected ParentTarget -> Chosen on AnyOpponent: {}",
        other_and_parent.abilities[1]
    );

    let trigger_players = read(
        "Name:WardCurse\nManaCost:1 B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ Attacks | ValidCard$ Card.AttachedBy | TriggerZones$ Battlefield | Execute$ HitAtk\n\
         T:Mode$ BecomesTarget | ValidTarget$ Card.Self | ValidSource$ SpellAbility.OppCtrl | TriggerZones$ Battlefield | Execute$ HitCaster\n\
         SVar:HitAtk:DB$ LoseLife | Defined$ TriggeredAttackerController | LifeAmount$ 2\n\
         SVar:HitCaster:DB$ LoseLife | Defined$ TriggeredSourceSAController | LifeAmount$ 3",
    );
    assert!(
        trigger_players
            .abilities
            .iter()
            .any(|a| a.contains("PlayerRel::ControllerOfEvent")),
        "expected ControllerOfEvent on Attacks: {:?}",
        trigger_players.abilities
    );
    assert!(
        trigger_players
            .abilities
            .iter()
            .any(|a| a.contains("PlayerRel::EventPlayer")),
        "expected EventPlayer on BecomesTarget: {:?}",
        trigger_players.abilities
    );
}

#[test]
fn wave14b_noncombat_prevention_and_damage_scaling() {
    let sanctuary = read(
        "Name:QuietWard\nManaCost:3 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ You,Creature.YouCtrl | IsCombat$ False\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ Player | IsCombat$ False | PlayerTurn$ True",
    );
    assert_eq!(
        sanctuary.abilities,
        [
            "static_ability!(Filter::This, Modifier::PreventNoncombatDamageToPlayers { who: PlayerRel::You, from: &Filter::Any })",
            "static_ability!(Filter::YOUR_CREATURE, Modifier::PreventNoncombatDamageFrom(&Filter::Any))",
            "static_ability!(Filter::This, Modifier::PreventNoncombatDamageToPlayers { who: PlayerRel::EachPlayer, from: &Filter::Any }, condition = Some(Condition::YourTurn))",
        ]
    );

    let scales = read(
        "Name:EmberEngine\nManaCost:3 R R\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card,Emblem | ValidTarget$ Permanent,Player | ReplaceWith$ DoubleDmg\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.YouCtrl,Emblem.YouCtrl | ValidTarget$ Permanent.OppCtrl,Opponent | IsCombat$ False | ReplaceWith$ TripleInline\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.RedSource+YouCtrl,Artifact.YouCtrl | ValidTarget$ Player.Opponent,Permanent.OppCtrl | ReplaceWith$ PlusTwo\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Creature.YouCtrl+inZoneBattlefield | ValidTarget$ Creature.OppCtrl | IsCombat$ True | ReplaceWith$ TimesFour\n\
         SVar:DoubleDmg:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ D2\n\
         SVar:D2:ReplaceCount$DamageAmount/Twice\n\
         SVar:TripleInline:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Thrice\n\
         SVar:PlusTwo:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ P2\n\
         SVar:P2:ReplaceCount$DamageAmount/Plus.2\n\
         SVar:TimesFour:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ T4\n\
         SVar:T4:ReplaceCount$DamageAmount/Times.4",
    );
    assert_eq!(
        scales.abilities[0],
        "static_ability!(Filter::Any, Modifier::MultiplyDamageFrom { sources: &Filter::Any, factor: 2, combat_only: false, noncombat_only: false, recipients: DamageRecipients::Any })"
    );
    assert_eq!(
        scales.abilities[1],
        "static_ability!(Filter::Any, Modifier::MultiplyDamageFrom { sources: &Filter::ControlledByYou, factor: 3, combat_only: false, noncombat_only: true, recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::Opponent) })"
    );
    assert!(
        scales.abilities[2].contains("Modifier::AddDamageFrom")
            && scales.abilities[2].contains("extra: 2")
            && scales.abilities[2].contains(
                "recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::Opponent)"
            ),
        "{}",
        scales.abilities[2]
    );
    assert!(
        scales.abilities[3].contains("Modifier::MultiplyDamageFrom")
            && scales.abilities[3].contains("factor: 4")
            && scales.abilities[3].contains("combat_only: true")
            && scales.abilities[3]
                .contains("recipients: DamageRecipients::Permanents(&Filter::OPPONENT_CREATURE)"),
        "{}",
        scales.abilities[3]
    );
}

#[test]
fn wave14b_soulbond_renown_snow_and_costs() {
    let pair = read(
        "Name:BondKnight\nManaCost:2 W\nTypes:Creature Human Knight\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.PairedWith,Creature.Self+Paired | AddPower$ 2 | AddToughness$ 2 | AddKeyword$ Double Strike\n\
         S:Mode$ Continuous | Affected$ Creature.PairedWith,Creature.Self+Paired | AddAbility$ TapSelf\n\
         SVar:TapSelf:AB$ GainLife | Cost$ T | LifeAmount$ 1",
    );
    assert_eq!(
        pair.abilities[0],
        "static_ability!(Filter::SOULBOND_PAIR, Modifier::ModifyPT(2, 2), condition = Some(Condition::IsPaired))"
    );
    assert_eq!(
        pair.abilities[1],
        "static_ability!(Filter::SOULBOND_PAIR, Modifier::AddKeyword(KeywordSet::DOUBLE_STRIKE), condition = Some(Condition::IsPaired))"
    );
    assert!(
        pair.abilities[2].contains("Filter::SOULBOND_PAIR")
            && pair.abilities[2].contains("Modifier::GrantActivated")
            && pair.abilities[2].contains("condition = Some(Condition::IsPaired)"),
        "{}",
        pair.abilities[2]
    );

    let renowned = read(
        "Name:FamedScout\nManaCost:G\nTypes:Creature Human Druid\nPT:1/1\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.IsRenowned | AddKeyword$ Vigilance | AddAbility$ TapMana\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | IsPresent$ Card.Self+IsRenowned\n\
         SVar:TapMana:AB$ Mana | Cost$ T | Produced$ Any",
    );
    assert_eq!(
        renowned.abilities[0],
        "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::VIGILANCE), condition = Some(Condition::IsRenowned))"
    );
    assert!(
        renowned.abilities[1].contains("Modifier::GrantActivated")
            && renowned.abilities[1].contains("condition = Some(Condition::IsRenowned)"),
        "{}",
        renowned.abilities[1]
    );
    assert_eq!(
        renowned.abilities[2],
        "static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(1, 0), condition = Some(Condition::IsRenowned))"
    );

    let costs = read(
        "Name:FrostForager\nManaCost:2 G\nTypes:Snow Creature Squirrel\nPT:2/2\n\
         A:AB$ Pump | Cost$ 1 S | Defined$ Self | NumAtt$ +1 | NumDef$ +1\n\
         A:AB$ Mana | Cost$ T Forage | Produced$ Combo Any | Amount$ 2\n\
         A:AB$ Draw | Cost$ DamageYou<2> | NumCards$ 1\n\
         A:AB$ Pump | Cost$ B Discard<X/Card> | Defined$ Self | NumAtt$ +X | NumDef$ +X\n\
         A:AB$ Pump | Cost$ T ExileFromGrave<X/Creature> | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        costs.abilities[0].contains("cost!(\"{1}{S}\")"),
        "{}",
        costs.abilities[0]
    );
    assert!(
        costs.abilities[1].contains("Forage"),
        "{}",
        costs.abilities[1]
    );
    assert!(
        costs.abilities[2].contains("DealDamageToYou(2)"),
        "{}",
        costs.abilities[2]
    );
    assert!(
        costs.abilities[3].contains("DiscardX(&"),
        "{}",
        costs.abilities[3]
    );
    assert!(
        costs.abilities[4].contains("ExileFromGraveyardX(&"),
        "{}",
        costs.abilities[4]
    );
}

#[test]
fn wave14b_refusal_boundaries() {
    for script in [
        "Name:BadNoncombatNoTarget\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidSource$ Creature | IsCombat$ False",
        "Name:BadMinusScale\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Spell | ValidTarget$ Permanent,Player | ReplaceWith$ Minus0\n\
         SVar:Minus0:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\n\
         SVar:X:ReplaceCount$DamageAmount/Minus.0",
        "Name:BadYouDontCtrlRecip\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Opponent,Permanent.YouDontCtrl | ReplaceWith$ Dbl\n\
         SVar:Dbl:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Twice",
        "Name:BadDamageYouZero\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ DamageYou<0> | NumCards$ 1",
        "Name:BadDiscardXRandom\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ Discard<X/Random> | NumCards$ X\n\
         SVar:X:Count$xPaid",
        "Name:BadExileGraveXCtrl\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ ExileFromGrave<X/Card.YouCtrl> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    ] {
        assert!(refused(script), "expected refusal for:\n{script}");
    }
}

#[test]
fn wave15b_pass_e8_targets_gift_spell_base_and_secondary() {
    // 1. CR 110.4b: `Permanent.YouCtrl` on a stack spell target preserves the
    // full permanent type union instead of collapsing to `ControlledByYou`.
    let copy_perm = read(
        "Name:MirrorRelic\nManaCost:4\nTypes:Artifact\n\
         A:AB$ CopySpellAbility | Cost$ 4 T | ValidTgts$ Permanent.YouCtrl | TargetType$ Spell",
    );
    assert!(
        copy_perm
            .statics
            .contains("TypeSet::ARTIFACT.union(TypeSet::BATTLE)")
            && copy_perm.statics.contains("Filter::ControlledByYou")
            && copy_perm.abilities[0].contains("TargetSpec::Spell(&TARGET"),
        "expected permanent type union on stack target: {} / {}",
        copy_perm.statics,
        copy_perm.abilities[0]
    );

    // 2. Combat target omitting `+YouCtrl` when every Oracle mention says
    // "target blocking creature you control" keeps `Filter::ControlledByYou`.
    let combat_scope = read(
        "Name:GuardTrick\nManaCost:W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.blocking | NumAtt$ +2 | NumDef$ +2\n\
         Oracle:Target blocking creature you control gets +2/+2 until end of turn.",
    );
    assert!(
        combat_scope.statics.contains("Filter::Blocking")
            && combat_scope.statics.contains("Filter::ControlledByYou"),
        "expected ControlledByYou from oracle: {}",
        combat_scope.statics
    );

    // Mixed oracle ("target blocking creature" without "you control") must NOT add ControlledByYou.
    let combat_any = read(
        "Name:OpenGuard\nManaCost:W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.blocking | NumAtt$ +1 | NumDef$ +1\n\
         Oracle:Target blocking creature gets +1/+1 until end of turn.",
    );
    assert!(
        combat_any.statics.contains("Filter::Blocking")
            && !combat_any.statics.contains("ControlledByYou"),
        "must not add ControlledByYou: {}",
        combat_any.statics
    );

    // 3. `Spell` filter base lowers to `Filter::InZone(ZoneRef::Stack)`.
    let spell_dmg = read(
        "Name:EmberAdept\nManaCost:1 R\nTypes:Creature Human Wizard\nPT:2/2\n\
         T:Mode$ DamageDealtOnce | ValidSource$ Spell.YouCtrl | TriggerZones$ Battlefield | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        spell_dmg.statics.contains("Filter::InZone(ZoneRef::Stack)")
            && spell_dmg.statics.contains("Filter::ControlledByYou"),
        "expected InZone(Stack) + ControlledByYou: {}",
        spell_dmg.statics
    );

    // 4. `Continuous.Secondary$ True` + `GiftWasPromised` + `PlayerRel::GiftRecipient`.
    let gift_and_secondary = read(
        "Name:SplitBoon\nManaCost:1 B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature.Goblin | AddPower$ 2 | AddToughness$ 2\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature.nonGoblin | AddPower$ -2 | AddToughness$ -2 | Secondary$ True\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.PromisedGift | AddKeyword$ Indestructible\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | CheckSVar$ GiftFlag | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 1 | IsPresent$ Card.Self+PromisedGift | PresentCompare$ EQ0\n\
         A:AB$ Draw | Cost$ T | Defined$ Promised | NumCards$ 1\n\
         SVar:GiftFlag:Count$PromisedGift.1.0",
    );
    assert!(
        gift_and_secondary.abilities.len() == 7
            && gift_and_secondary.abilities[3].contains("Condition::GiftWasPromised")
            && gift_and_secondary.abilities[4].contains("Condition::GiftWasPromised")
            && gift_and_secondary.abilities[5]
                .contains("Condition::Not(&Condition::GiftWasPromised)")
            && gift_and_secondary.abilities[6].contains("PlayerRel::GiftRecipient"),
        "{:?}",
        gift_and_secondary.abilities
    );

    // Refusal: unexpected `Continuous.Secondary$` value stays refused.
    assert!(refused(
        "Name:BadSecondary\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | Secondary$ Maybe"
    ));
}

#[test]
fn wave9_10b_combat_damage_toughness_and_unblocked_rules() {
    // 1. `CombatDamageToughness` -> `AssignCombatDamageUsingToughness` and
    //    `AssignCombatDamageUsingToughnessIfToughnessGreater`, preserving
    //    `Secondary$ True`, `EffectZone$`, and static conditions.
    let all_tough = read(
        "Name:GraniteBulwark\nManaCost:1 W G\nTypes:Creature Wall\nPT:0/5\n\
         S:Mode$ CombatDamageToughness | ValidCard$ Creature | Secondary$ True",
    );
    assert_eq!(
        all_tough.abilities,
        ["static_ability!(Filter::CREATURE, Modifier::AssignCombatDamageUsingToughness)"]
    );

    let turn_tough = read(
        "Name:SunlitKeep\nManaCost:2 W\nTypes:Enchantment\n\
         S:Mode$ CombatDamageToughness | Condition$ PlayerTurn | ValidCard$ Creature.YouCtrl",
    );
    assert_eq!(
        turn_tough.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::AssignCombatDamageUsingToughness, condition = Some(Condition::YourTurn))"
        ]
    );

    let gt_tough = read(
        "Name:MossColossus\nManaCost:3 G\nTypes:Creature Treefolk\nPT:2/6\n\
         S:Mode$ CombatDamageToughness | ValidCard$ Creature.powerLTtoughness+YouCtrl\n\
         S:Mode$ CombatDamageToughness | ValidCard$ Creature.EquippedBy+powerLTtoughness | EffectZone$ Command",
    );
    assert_eq!(
        gt_tough.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater)",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater, zone = ZoneRef::Command)",
        ]
    );

    // 2. `AssignCombatDamageAsUnblocked` -> `MayAssignCombatDamageAsThoughUnblocked`.
    let sneaky = read(
        "Name:PhantomProwler\nManaCost:2 G\nTypes:Creature Wolf\nPT:2/2\n\
         S:Mode$ AssignCombatDamageAsUnblocked | ValidCard$ Card.Self | Optional$ True | Secondary$ True",
    );
    assert_eq!(
        sneaky.abilities,
        ["static_ability!(Filter::This, Modifier::MayAssignCombatDamageAsThoughUnblocked)"]
    );

    let pack_leader = read(
        "Name:PackAlpha\nManaCost:5 G G\nTypes:Creature Beast\nPT:6/6\n\
         S:Mode$ AssignCombatDamageAsUnblocked | IsPresent$ Card.Self+attacking | ValidCard$ Creature.YouCtrl | Optional$ True",
    );
    assert!(
        pack_leader.abilities[0].contains("Modifier::MayAssignCombatDamageAsThoughUnblocked")
            && pack_leader.abilities[0].contains("Condition::SourceMatches(&CHECK1)")
            && pack_leader.statics.contains("Filter::Attacking"),
        "{:?} / {}",
        pack_leader.abilities,
        pack_leader.statics
    );

    for bad in [
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CombatDamageToughness | ValidCard$ Creature.!powerLTtoughness",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CombatDamageToughness | ValidCard$ Creature.powerLTtoughness,Creature.YouCtrl",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CombatDamageToughness | ValidCard$ powerLTtoughness",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CombatDamageToughness | Secondary$ True",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CombatDamageToughness | ValidCard$ Creature | Secondary$ False",
        "Name:Bad\nTypes:Creature\nPT:2/2\nS:Mode$ AssignCombatDamageAsUnblocked | ValidCard$ Card.Self",
        "Name:Bad\nTypes:Creature\nPT:2/2\nS:Mode$ AssignCombatDamageAsUnblocked | ValidCard$ Card.Self | Optional$ False",
        "Name:Bad\nTypes:Creature\nPT:2/2\nS:Mode$ AssignCombatDamageAsUnblocked | Optional$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave9_10b_cant_gain_life_and_cant_sacrifice_rules() {
    // 1. `CantGainLife` -> `PlayersCantGainLife(PlayerRel::...)`.
    for (line, expected) in [
        (
            "S:Mode$ CantGainLife | Secondary$ True",
            "Modifier::PlayersCantGainLife(PlayerRel::EachPlayer)",
        ),
        (
            "S:Mode$ CantGainLife | ValidPlayer$ Player",
            "Modifier::PlayersCantGainLife(PlayerRel::EachPlayer)",
        ),
        (
            "S:Mode$ CantGainLife | ValidPlayer$ You | EffectZone$ Command",
            "Modifier::PlayersCantGainLife(PlayerRel::You), zone = ZoneRef::Command",
        ),
        (
            "S:Mode$ CantGainLife | ValidPlayer$ Player.Opponent",
            "Modifier::PlayersCantGainLife(PlayerRel::EachOpponent)",
        ),
        (
            "S:Mode$ CantGainLife | ValidPlayer$ EnchantedPlayer",
            "Modifier::PlayersCantGainLife(PlayerRel::EnchantedPlayer)",
        ),
    ] {
        let script = format!("Name:PaleCurse\nManaCost:2 B\nTypes:Enchantment\n{line}");
        let body = read(&script);
        assert!(
            body.abilities[0].contains(expected),
            "expected {expected} for `{line}`, got {:?}",
            body.abilities
        );
    }

    // 2. `CantSacrifice` -> `Modifier::CantBeSacrificed { opponents_only }`.
    let sac_lock = read(
        "Name:IronOath\nManaCost:3 W G\nTypes:Creature Angel\nPT:5/5\n\
         S:Mode$ CantSacrifice | ValidCard$ Card.Self | Secondary$ True\n\
         S:Mode$ CantSacrifice | ValidCard$ Artifact.token+YouCtrl\n\
         S:Mode$ CantSacrifice | ValidCard$ Card.YouCtrl | ValidCause$ SpellAbility.OppCtrl | ForCost$ False",
    );
    assert_eq!(
        sac_lock.abilities,
        [
            "static_ability!(Filter::This, Modifier::CantBeSacrificed { opponents_only: false })",
            "static_ability!(Filter::And(&[Filter::ARTIFACT, Filter::IsToken, Filter::ControlledByYou]), Modifier::CantBeSacrificed { opponents_only: false })",
            "static_ability!(Filter::ControlledByYou, Modifier::CantBeSacrificed { opponents_only: true })",
        ]
    );

    for bad in [
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CantGainLife | ValidPlayer$ Player.IsRemembered",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CantGainLife | ValidPlayer$ You | EffectZone$ Graveyard",
        "Name:Bad\nTypes:Creature\nPT:4/4\nS:Mode$ CantSacrifice | ValidCard$ Creature | ValidCause$ Spell,Activated | ForCost$ True",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CantSacrifice | ValidCard$ Card.Self | ValidCause$ SpellAbility.OppCtrl | ForCost$ False",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CantSacrifice | ValidCard$ Creature.YouCtrl | ValidCause$ SpellAbility.OppCtrl",
        "Name:Bad\nTypes:Enchantment\nS:Mode$ CantSacrifice | ValidCard$ Creature.YouCtrl | ValidCause$ Triggered.OppCtrl | ForCost$ False",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn cast_with_flash_supports_ability_words_paired_svars_and_refuses_non_typeset() {
    let conditional_flash = read(
        "Name:TempestHermit\nManaCost:2 U\nTypes:Creature Merfolk\nPT:2/3\n\
         S:Mode$ CastWithFlash | ValidCard$ Sorcery | ValidSA$ Spell | Caster$ You | Delirium$ True | CheckSVar$ FirstCast | SVarCompare$ EQ0 | CheckSecondSVar$ Gate | SecondSVarCompare$ GE2\n\
         SVar:FirstCast:Count$ThisTurnCast_Card.YouCtrl\n\
         SVar:Gate:Count$Valid Island.YouCtrl",
    );
    assert!(
        conditional_flash.abilities.len() == 1
            && conditional_flash.abilities[0].contains("Modifier::SorceriesHaveFlash")
            && conditional_flash.abilities[0].contains("Condition::GraveyardTypesAtLeast(4)")
            && conditional_flash.abilities[0].contains("Condition::ControlCount")
            && conditional_flash.abilities[0].contains("Condition::YouCastNoSpellThisTurn"),
        "{:?}",
        conditional_flash.abilities
    );

    let bad = "Name:Bad\nTypes:Creature\nPT:1/1\nS:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | Caster$ You | EffectZone$ All";
    assert!(refused(bad), "must refuse:\n{bad}");
}

#[test]
fn wave19d_command_library_and_highest_gy_conditions() {
    let cmd_reveal = read(
        "Name:MistPlane\nTypes:Plane Skybreen\n\
         S:Mode$ Continuous | EffectZone$ Command | Affected$ Card.TopLibrary | AffectedZone$ Library | MayLookAt$ Player",
    );
    assert_eq!(
        cmd_reveal.abilities,
        ["static_ability!(Filter::Any, Modifier::RevealAllLibraryTops, zone = ZoneRef::Command)"]
    );

    let cmd_look_yours = read(
        "Name:LensEmblem\nTypes:Emblem\n\
         S:Mode$ Continuous | EffectZone$ Battlefield,Command | Affected$ Card.TopLibrary+YouCtrl | AffectedZone$ Library | MayLookAt$ You | Condition$ PlayerTurn",
    );
    assert_eq!(
        cmd_look_yours.abilities,
        [
            "static_ability!(Filter::Any, Modifier::LookAtLibraryTop, zone = ZoneRef::BattlefieldOrCommand, condition = Some(Condition::YourTurn))"
        ]
    );

    let any_gy = read(
        "Name:DeepDweller\nManaCost:2 U\nTypes:Creature Fish\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 3 | CheckSVar$ X | SVarCompare$ GE10\n\
         SVar:X:PlayerCountPlayers$HighestCardsInGraveyard",
    );
    assert!(
        any_gy.abilities[0].contains("Condition::AnyGraveyardCountAtLeast(10)"),
        "{:?}",
        any_gy.abilities
    );

    let dual_inline = read(
        "Name:GroveSentinel\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddKeyword$ Vigilance | CheckSVar$ P | SVarCompare$ LE3 | CheckSecondSVar$ Count$Valid Forest.YouCtrl\n\
         SVar:P:PlayerCountPlayers$HighestCardsInGraveyard",
    );
    assert!(
        dual_inline.abilities[0].contains("Condition::All(&[Condition::Not(&Condition::AnyGraveyardCountAtLeast(4)), Condition::ControlCount("),
        "{:?}",
        dual_inline.abilities
    );

    let bad_cmd_play = "Name:BadCmdPlay\nTypes:Plane\n\
         S:Mode$ Continuous | EffectZone$ Command | Affected$ Card.TopLibrary | AffectedZone$ Library | MayPlay$ You";
    assert!(refused(bad_cmd_play), "must refuse:\n{bad_cmd_play}");
}

#[test]
fn wave19d_moved_tap_counters_and_kicked_etb() {
    let tap_counters = read(
        "Name:ChargedSand\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ETBTap | ReplacementResult$ Updated\n\
         SVar:ETBTap:DB$ Tap | Defined$ Self | ETB$ True | SubAbility$ ETBCtr\n\
         SVar:ETBCtr:DB$ PutCounter | Defined$ Self | ETB$ True | CounterType$ CHARGE | CounterNum$ 2",
    );
    assert_eq!(
        tap_counters.enter_modifiers,
        [
            "EnterModifier::Tapped",
            "EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(2) }"
        ]
    );

    let ctr_then_tap = read(
        "Name:SlumberStone\nManaCost:3\nTypes:Artifact\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ETBCtr | ReplacementResult$ Updated\n\
         SVar:ETBCtr:DB$ PutCounter | CounterType$ STUN | CounterNum$ 3 | SubAbility$ ETBTap\n\
         SVar:ETBTap:DB$ Tap | Defined$ Self | ETB$ True",
    );
    assert_eq!(
        ctr_then_tap.enter_modifiers,
        [
            "EnterModifier::Tapped",
            "EnterModifier::WithCounters { kind: CounterKind::Stun, amount: Amount::Fixed(3) }"
        ]
    );

    let kicked_haste = read(
        "Name:RushBeast\nManaCost:3 G\nTypes:Creature Beast\nPT:3/3\n\
         K:Kicker:2 G\n\
         R:Event$ Moved | ValidCard$ Card.Self+kicked | Destination$ Battlefield | ReplaceWith$ Boost | ReplacementResult$ Updated\n\
         SVar:Boost:DB$ PutCounter | ETB$ True | Defined$ Self | CounterType$ P1P1 | CounterNum$ 3 | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Self | Keywords$ Haste & Trample | Duration$ Permanent",
    );
    assert_eq!(
        kicked_haste.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(3), condition: Condition::KickedWith(0) }"
        ]
    );
    assert!(
        kicked_haste.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::HASTE.union(KeywordSet::TRAMPLE)), condition = Some(Condition::KickedWith(0)))"
        }),
        "{:?}",
        kicked_haste.abilities
    );

    let kicked_wall = read(
        "Name:MobileRampart\nManaCost:1 W\nTypes:Creature Wall\nPT:1/3\n\
         K:Defender\n\
         K:Kicker:1 W\n\
         R:Event$ Moved | ValidCard$ Card.Self+kicked | Destination$ Battlefield | ReplaceWith$ Boost | ReplacementResult$ Updated\n\
         SVar:Boost:DB$ PutCounter | CounterType$ P1P1 | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Self | staticAbilities$ CanSwing | Duration$ Permanent\n\
         SVar:CanSwing:Mode$ CanAttackDefender | ValidCard$ Card.Self",
    );
    assert_eq!(
        kicked_wall.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(1), condition: Condition::KickedWith(0) }"
        ]
    );
    assert!(
        kicked_wall.abilities.iter().any(|a| {
            a == "static_ability!(Filter::This, Modifier::AttacksDespiteDefender, condition = Some(Condition::KickedWith(0)))"
        }),
        "{:?}",
        kicked_wall.abilities
    );

    let kicked_count = read(
        "Name:ScaleGolem\nManaCost:4\nTypes:Artifact Creature Golem\nPT:4/4\n\
         K:Kicker:3\n\
         R:Event$ Moved | ValidCard$ Card.Self+kicked | Destination$ Battlefield | ReplaceWith$ Boost | ReplacementResult$ Updated\n\
         SVar:Boost:DB$ PutCounter | ETB$ True | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:Count$Valid Land.nonBasic+OppCtrl",
    );
    assert!(
        kicked_count.enter_modifiers[0].contains("EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::CountOf { filter: &"),
        "{:?}",
        kicked_count.enter_modifiers
    );

    for bad in [
        "Name:CondTapSub\nManaCost:3\nTypes:Creature\nPT:2/2\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ETBCtr | ReplacementResult$ Updated\n\
         SVar:ETBCtr:DB$ PutCounter | ETB$ True | CounterType$ STUN | CounterNum$ 2 | SubAbility$ ETBTap\n\
         SVar:ETBTap:DB$ Tap | ETB$ True | ConditionCheckSVar$ X | ConditionSVarCompare$ LE2\n\
         SVar:X:Count$Valid Land.YouCtrl",
        "Name:TempKickedAnim\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Kicker:1 R\n\
         R:Event$ Moved | ValidCard$ Card.Self+kicked | Destination$ Battlefield | ReplaceWith$ Boost | ReplacementResult$ Updated\n\
         SVar:Boost:DB$ PutCounter | ETB$ True | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Self | Keywords$ Haste",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

fn wave20d_cats_and_tokens() -> (SubtypeCatalogs, TokenLookup) {
    let mut c = cats();
    c.creature.push("Angel".into());
    c.creature.push("Soldier".into());
    c.artifact.push("Food".into());
    c.artifact.push("Treasure".into());
    c.normalize();
    let tok = TokenLookup::held(&[
        (
            "tok_clue",
            "Name:Clue Token\nTypes:Artifact Clue\nA:AB$ Draw | Cost$ 2 Sac<1/CARDNAME> | NumCards$ 1",
        ),
        (
            "tok_food",
            "Name:Food Token\nTypes:Artifact Food\nA:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3",
        ),
        (
            "tok_treasure",
            "Name:Treasure Token\nTypes:Artifact Treasure\nA:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any",
        ),
        (
            "tok_angel",
            "Name:Angel\nTypes:Creature Angel\nColors:white\nPT:4/4\nK:Flying",
        ),
        (
            "tok_soldier",
            "Name:Soldier\nTypes:Creature Soldier\nColors:white\nPT:1/1",
        ),
    ]);
    (c, tok)
}

#[test]
fn wave20d_counter_replacement_variants() {
    let halver = read(
        "Name:HalfBoth\nTypes:Creature\nPT:6/6\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidSource$ Opponent | \
         ValidObject$ Permanent.inZoneBattlefield,Player | ReplaceWith$ HalfC\n\
         SVar:HalfC:DB$ ReplaceCounter | Amount$ H\n\
         SVar:H:ReplaceCount$CounterNum/HalfDown",
    );
    assert_eq!(
        halver.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::Any, kind: None, placer: Some(PlayerRel::EachOpponent), \
             effect_only: false, op: CounterReplaceOp::HalveFloor })",
            "AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement { \
             recipient: PlayerRel::EachPlayer, kind: None, placer: Some(PlayerRel::EachOpponent), \
             op: CounterReplaceOp::HalveFloor })",
        ]
    );

    let anti_infect = read(
        "Name:NoNegCounters\nTypes:Creature\nPT:1/1\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCard$ Creature.YouCtrl+inZoneBattlefield | \
         ValidCounterType$ M1M1 | Prevent$ True | Layer$ CantHappen",
    );
    assert_eq!(
        anti_infect.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::YOUR_CREATURE, kind: Some(CounterKind::M1M1), placer: None, \
             effect_only: false, op: CounterReplaceOp::Sub(u16::MAX) })"
        ]
    );

    let player_plus = read(
        "Name:EnergyAmp\nTypes:Artifact\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidPlayer$ You | \
         ValidCounterType$ ENERGY | ReplaceWith$ AddOne\n\
         SVar:AddOne:DB$ ReplaceCounter | Amount$ ReplaceCount$CounterNum/Plus.1",
    );
    assert_eq!(
        player_plus.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement { \
             recipient: PlayerRel::You, kind: Some(CounterKind::Energy), placer: None, \
             op: CounterReplaceOp::Add(1) })"
        ]
    );

    let self_minus = read(
        "Name:ShieldTrim\nTypes:Creature\nPT:2/2\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCard$ Card.Self | \
         ValidCounterType$ P1P1 | ReplaceWith$ SubOne\n\
         SVar:SubOne:DB$ ReplaceCounter | Amount$ D\n\
         SVar:D:ReplaceCount$CounterNum/Minus.1",
    );
    assert_eq!(
        self_minus.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement { \
             object_filter: &Filter::This, kind: Some(CounterKind::P1P1), placer: None, \
             effect_only: false, op: CounterReplaceOp::Sub(1) })"
        ]
    );

    for bad in [
        "Name:PlayerEffectOnly\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | EffectOnly$ True | ValidPlayer$ You | ReplaceWith$ X\n\
         SVar:X:DB$ ReplaceCounter | Amount$ ReplaceCount$CounterNum/Plus.1",
        "Name:HalfUpBad\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCard$ Creature.inZoneBattlefield | ReplaceWith$ X\n\
         SVar:X:DB$ ReplaceCounter | Amount$ ReplaceCount$CounterNum/HalfUp",
        "Name:TwoKinds\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCounterType$ P1P1,M1M1 | \
         ValidCard$ Creature.inZoneBattlefield | ReplaceWith$ X\n\
         SVar:X:DB$ ReplaceCounter | Amount$ ReplaceCount$CounterNum/Plus.1",
        "Name:MissingTarget\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ReplaceWith$ X\n\
         SVar:X:DB$ ReplaceCounter | Amount$ ReplaceCount$CounterNum/Plus.1",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave20d_token_replacement_variants() {
    let (c, tok) = wave20d_cats_and_tokens();
    let read_tok = |text: &str| {
        let p = parse(text);
        transcode(&p, &c, Some(&tok))
            .unwrap_or_else(|| panic!("refused {:?}: {text}", refusal_reason(&p, &c, Some(&tok))))
    };
    let refused_tok = |text: &str| transcode(&parse(text), &c, Some(&tok)).is_none();

    let bonus_match = read_tok(
        "Name:ExtraArmy\nTypes:Creature\nPT:3/3\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Creature.YouCtrl | \
         Metalcraft$ True | ReplaceWith$ PlusSol\n\
         SVar:PlusSol:DB$ ReplaceToken | Type$ AddToken | TokenScript$ tok_soldier | Amount$ X\n\
         SVar:X:ReplaceCount$TokenNum",
    );
    assert_eq!(
        bonus_match.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::AdditionalTokenCreation { \
             controller_filter: &Filter::ControlledByYou, token_filter: &Filter::CREATURE, \
             bonus_token: &generated_tokens::SOLDIER_1_1_WHITE, match_count: true, \
             condition: Some(Condition::ControlCount(&Filter::ARTIFACT, 3)) })"
        ]
    );

    let bonus_flat = read_tok(
        "Name:TreasureSmith\nTypes:Creature\nPT:2/2\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Treasure.YouCtrl | ReplaceWith$ PlusTr\n\
         SVar:PlusTr:DB$ ReplaceToken | Type$ AddToken | TokenScript$ tok_treasure | Amount$ 1",
    );
    assert!(
        bonus_flat.abilities[0]
            .contains("bonus_token: &generated_tokens::TREASURE, match_count: false"),
        "{:?}",
        bonus_flat.abilities
    );

    let substitute = read_tok(
        "Name:AngelMirror\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Creature.nonAngel+YouCtrl | ReplaceWith$ SubAng\n\
         SVar:SubAng:DB$ ReplaceToken | Type$ ReplaceToken | TokenScript$ tok_angel",
    );
    assert!(
        substitute.abilities[0].contains(
            "ReplacementRule::SubstituteTokenCreation { controller_filter: &Filter::ControlledByYou"
        ) && substitute.abilities[0]
            .contains("replacement: &generated_tokens::ANGEL_4_4_WHITE_FLYING, condition: None"),
        "{:?}",
        substitute.abilities
    );

    let manufactor = read_tok(
        "Name:TripleGadget\nTypes:Artifact Creature\nPT:1/3\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | \
         ValidToken$ Clue.YouCtrl,Food.YouCtrl,Treasure.YouCtrl | ReplaceWith$ MakeThree\n\
         SVar:MakeThree:DB$ ReplaceToken | Type$ MultipleTokens | TokenScript$ tok_clue,tok_food,tok_treasure",
    );
    assert_eq!(
        manufactor.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ManufactorTokenCreation { \
             controller_filter: &Filter::ControlledByYou, clue: &generated_tokens::CLUE, \
             food: &generated_tokens::FOOD, treasure: &generated_tokens::TREASURE })"
        ]
    );

    for bad in [
        "Name:OptAdd\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | Optional$ True | ValidToken$ Card.YouCtrl | ReplaceWith$ A\n\
         SVar:A:DB$ ReplaceToken | Type$ AddToken | TokenScript$ tok_food | Amount$ 1",
        "Name:EffectOnlyAdd\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | EffectOnly$ True | ValidToken$ Card.YouCtrl | ReplaceWith$ A\n\
         SVar:A:DB$ ReplaceToken | Type$ AddToken | TokenScript$ tok_food | Amount$ 1",
        "Name:ChoiceSub\nTypes:Creature\nPT:2/2\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Card.YouCtrl | ReplaceWith$ C\n\
         SVar:C:DB$ GenericChoice",
        "Name:TwoTokens\nTypes:Creature\nPT:2/2\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Food.YouCtrl,Treasure.YouCtrl | ReplaceWith$ M\n\
         SVar:M:DB$ ReplaceToken | Type$ MultipleTokens | TokenScript$ tok_food,tok_treasure",
        "Name:CondManu\nTypes:Artifact Creature\nPT:1/3\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | Metalcraft$ True | \
         ValidToken$ Clue.YouCtrl,Food.YouCtrl,Treasure.YouCtrl | ReplaceWith$ M\n\
         SVar:M:DB$ ReplaceToken | Type$ MultipleTokens | TokenScript$ tok_clue,tok_food,tok_treasure",
        "Name:AddTwo\nTypes:Enchantment\n\
         R:Event$ CreateToken | ActiveZones$ Battlefield | ValidToken$ Card.YouCtrl | ReplaceWith$ A\n\
         SVar:A:DB$ ReplaceToken | Type$ AddToken | TokenScript$ tok_food | Amount$ 2",
    ] {
        assert!(refused_tok(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave20d_draw_replacement_variants() {
    let double_draw = read(
        "Name:ReflectDraws\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | NotFirstCardInDrawStep$ True | \
         Hellbent$ True | ReplaceWith$ DrawTwo\n\
         SVar:DrawTwo:DB$ Draw | Defined$ ReplacedPlayer | NumCards$ 2",
    );
    assert_eq!(
        double_draw.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawWithDraws { \
             who: PlayerRel::You, skip_first_in_draw_step: true, \
             condition: Some(Condition::HandSizeAtMost(0)), count: 2 })"
        ]
    );

    let thief = read(
        "Name:StealExtraDraw\nTypes:Creature\nPT:3/1\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Opponent | \
         NotFirstCardInDrawStep$ True | ReplaceWith$ DrawYou\n\
         SVar:DrawYou:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(
        thief.abilities,
        ["AbilityDef::Replacement(ReplacementRule::OpponentsExtraDrawsYouDraw)"]
    );

    let skip_empty = read(
        "Name:EmptyGuard\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | \
         IsPresent$ Card.YouOwn | PresentZone$ Library | PresentCompare$ EQ0 | Optional$ True | Prevent$ True",
    );
    assert_eq!(
        skip_empty.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::SkipDrawInstead { \
             who: PlayerRel::You, optional: true, empty_library_only: true })"
        ]
    );

    let win_empty = read(
        "Name:EmptyWin\nTypes:Creature\nPT:2/2\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | \
         CheckSVar$ Lib | SVarCompare$ EQ0 | ReplaceWith$ Win\n\
         SVar:Lib:Count$ValidLibrary Card.YouOwn\n\
         SVar:Win:DB$ WinsGame | Defined$ You",
    );
    assert_eq!(
        win_empty.abilities,
        ["AbilityDef::Replacement(ReplacementRule::WinOnEmptyLibraryDraw)"]
    );

    let look_top = read(
        "Name:GraveLook\nTypes:Creature\nPT:4/3\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ LookThree\n\
         SVar:LookThree:DB$ Dig | DigNum$ 3 | ChangeNum$ 1 | DestinationZone$ Hand | DestinationZone2$ Graveyard",
    );
    assert_eq!(
        look_top.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawLookTop { \
             count: 3, rest_to_graveyard: true })"
        ]
    );
}

#[test]
fn wave20d_draw_exile_and_refusals() {
    let uba_paired = read(
        "Name:ExileMask\nTypes:Artifact\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Player | ReplaceWith$ ExTop\n\
         S:Mode$ Continuous | AffectedZone$ Exile | Affected$ Card.ExiledWithSource+ThisTurnEntered | \
         MayPlay$ True | MayPlayPlayer$ CardOwner\n\
         SVar:ExTop:DB$ Dig | Defined$ ReplacedPlayer | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile",
    );
    assert_eq!(
        uba_paired.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop { \
             who: PlayerRel::EachPlayer, face_down: false, may_play_this_turn: true })"
        ]
    );

    let exile_facedown_cz = read(
        "Name:DarkExileDraw\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ ExDown\n\
         SVar:ExDown:DB$ ChangeZone | Defined$ ReplacedPlayer | Origin$ Library | \
         Destination$ Exile | LibraryPosition$ 0 | FaceDown$ True",
    );
    assert_eq!(
        exile_facedown_cz.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ReplaceDrawExileTop { \
             who: PlayerRel::You, face_down: true, may_play_this_turn: false })"
        ]
    );

    for bad in [
        "Name:DrawCardsBad\nTypes:Enchantment\n\
         R:Event$ DrawCards | ActiveZones$ Battlefield | ValidPlayer$ You | Prevent$ True",
        "Name:RevealLookBad\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ D\n\
         SVar:D:DB$ Dig | DigNum$ 3 | Reveal$ True | DestinationZone2$ Graveyard",
        "Name:TopLookBad\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ D\n\
         SVar:D:DB$ Dig | DigNum$ 3 | DestinationZone2$ Library | LibraryPosition2$ 0",
        "Name:EmptyOppSkipBad\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Player | \
         IsPresent$ Card.YouOwn | PresentZone$ Library | PresentCompare$ EQ0 | Prevent$ True",
        "Name:WinNoEmptyBad\nTypes:Enchantment\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ W\n\
         SVar:W:DB$ WinsGame | Defined$ You",
        "Name:UnpairedExilePlayBad\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Exile | Affected$ Card.ExiledWithSource+ThisTurnEntered | \
         MayPlay$ True | MayPlayPlayer$ CardOwner",
        "Name:MismatchMayPlayBad\nTypes:Artifact\n\
         R:Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ Player | ReplaceWith$ ExTop\n\
         S:Mode$ Continuous | AffectedZone$ Exile | Affected$ Card.ExiledWithSource+ThisTurnEntered | \
         MayPlay$ True | MayPlayPlayer$ You\n\
         SVar:ExTop:DB$ Dig | Defined$ ReplacedPlayer | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave20d_flash_exile_prepared_misc() {
    let omit_sa_flash = read(
        "Name:OmittedSaFlash\nTypes:Instant\n\
         S:Mode$ CastWithFlash | ValidCard$ Sorcery | Caster$ You",
    );
    assert_eq!(
        omit_sa_flash.abilities,
        ["static_ability!(Filter::Any, Modifier::SorceriesHaveFlash)"]
    );

    let linked_cond = read(
        "Name:LinkedExileGate\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ ExCount | SVarCompare$ GE1\n\
         SVar:ExCount:Count$ValidExile Card.ExiledWithSource",
    );
    assert!(
        linked_cond.abilities[0].contains(
            "condition = Some(Condition::AmountAtLeast(Amount::CountOf { \
             filter: &Filter::Any, zone: ZoneSel::LinkedExile }, 1))"
        ),
        "{:?}",
        linked_cond.abilities
    );

    let moved_prep = read(
        "Name:PrepFrontMoved\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ DBPrep\n\
         SVar:DBPrep:DB$ AlterAttribute | Attributes$ Prepared\n\
         AlternateMode:Prepare\n\
         ALTERNATE\n\
         Name:PrepSpell\nManaCost:R\nTypes:Instant\n\
         A:SP$ LoseLife | ValidTgts$ Opponent | LifeAmount$ 2",
    );
    assert!(moved_prep.back_is_prepare);
    assert_eq!(moved_prep.enter_modifiers, ["EnterModifier::Prepared"]);

    let bad = "Name:BadAttr\nTypes:Creature\nPT:1/1\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ M\n\
         SVar:M:DB$ AlterAttribute | Attributes$ Monstrous";
    assert!(refused(bad), "must refuse:\n{bad}");
}

#[test]
fn wave21d_continuous_become_types_and_votes() {
    let zero_sub = read(
        "Name:Starlit Veil\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddType$ Enchantment | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True",
    );
    assert!(
        zero_sub.abilities.iter().any(|a| {
            a.contains("Modifier::BecomeTypes { types: TypeSet::ENCHANTMENT, subtypes: &[] }")
        }),
        "{:?}",
        zero_sub.abilities
    );

    let two_sub = read(
        "Name:Guild Morph\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddType$ Creature & Goblin & Wizard | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True | SetPower$ 3 | SetToughness$ 3 | RemoveAllAbilities$ True",
    );
    assert!(two_sub.abilities.iter().any(|a| {
        a.contains("Modifier::BecomeTypes { types: TypeSet::CREATURE, subtypes: &[subtypes::creature::GOBLIN, subtypes::creature::WIZARD] }")
    }));
    assert!(
        two_sub
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::LoseAllAbilities"))
    );
    assert!(
        two_sub
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SetPT(3, 3)"))
    );

    let colorless_land = read(
        "Name:Dryad Song\nTypes:Enchantment Aura\nK:Enchant:Permanent\n\
         S:Mode$ Continuous | Affected$ Permanent.EnchantedBy | AddType$ Land & Forest | \
         SetColor$ Colorless | RemoveCardTypes$ True | RemoveLandTypes$ True | RemoveAllAbilities$ True",
    );
    assert!(colorless_land.abilities.iter().any(|a| {
        a.contains(
            "Modifier::BecomeTypes { types: TypeSet::LAND, subtypes: &[subtypes::land::FOREST] }",
        )
    }));
    assert!(
        colorless_land
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SetColor(ColorSet::EMPTY)"))
    );

    let legend_body = read(
        "Name:Crown Ascension\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddType$ Creature & Legendary & Goblin | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True",
    );
    assert!(legend_body.abilities.iter().any(|a| {
        a.contains("Modifier::BecomeTypes { types: TypeSet::CREATURE, subtypes: &[subtypes::creature::GOBLIN] }")
    }));
    assert!(
        legend_body
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::AddSupertype(SupertypeSet::LEGENDARY)"))
    );

    let rep_vote = read(
        "Name:Council Aide\nManaCost:1 W\nTypes:Creature\nPT:1/4\n\
         S:Mode$ Continuous | Affected$ You | AdditionalVote$ 1",
    );
    assert_eq!(
        rep_vote.abilities,
        ["static_ability!(Filter::This, Modifier::AdditionalVotes { count: 1, optional: false })"]
    );

    let opt_vote = read(
        "Name:Crown Steward\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ You | AdditionalOptionalVote$ 1 | Condition$ Monarch",
    );
    assert_eq!(
        opt_vote.abilities,
        [
            "static_ability!(Filter::This, Modifier::AdditionalVotes { count: 1, optional: true }, condition = Some(Condition::IsMonarch(PlayerRel::You)))"
        ]
    );

    let full_strip = read(
        "Name:Ascended Form\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddType$ Enchantment & Creature | \
         RemoveCardTypes$ True | RemoveSubTypes$ True",
    );
    assert!(full_strip.abilities.iter().any(|a| {
        a.contains(
            "Modifier::BecomeTypes { types: TypeSet::ENCHANTMENT.union(TypeSet::CREATURE), subtypes: &[] }",
        )
    }));
}

#[test]
fn wave21d_wave11c_monarch_initiative_dungeon_conditions() {
    let wave11c_conds = read(
        "Name:Throne Relic\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | Condition$ Monarch\n\
         S:Mode$ CantAttack | ValidCard$ Creature.OppCtrl | Condition$ Initiative\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1 | Condition$ CompletedADungeon\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Hexproof | Condition$ Monarch\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 1 | CheckSVar$ Mon | SVarCompare$ EQ0\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 2 | CheckSVar$ MonInv | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Vigilance | CheckSVar$ Init | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Reach | CheckSVar$ Dung | SVarCompare$ GE1\n\
         SVar:Mon:Count$Monarch.1.0\n\
         SVar:MonInv:Count$Monarch.0.1\n\
         SVar:Init:Count$Initiative.1.0\n\
         SVar:Dung:DungeonsCompleted$Amount",
    );
    assert!(
        wave11c_conds.abilities[0].contains("Condition::IsMonarch(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[0]
    );
    assert!(
        wave11c_conds.abilities[1].contains("Condition::HasInitiative(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[1]
    );
    assert!(
        wave11c_conds.abilities[2].contains("Condition::CompletedADungeon(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[2]
    );
    assert!(
        wave11c_conds.abilities[3].contains("Condition::IsMonarch(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[3]
    );
    assert!(
        wave11c_conds.abilities[4]
            .contains("Condition::Not(&Condition::IsMonarch(PlayerRel::You))"),
        "{:?}",
        wave11c_conds.abilities[4]
    );
    assert!(
        wave11c_conds.abilities[5]
            .contains("Condition::Not(&Condition::IsMonarch(PlayerRel::You))"),
        "{:?}",
        wave11c_conds.abilities[5]
    );
    assert!(
        wave11c_conds.abilities[6].contains("Condition::HasInitiative(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[6]
    );
    assert!(
        wave11c_conds.abilities[7].contains("Condition::CompletedADungeon(PlayerRel::You)"),
        "{:?}",
        wave11c_conds.abilities[7]
    );

    for bad in [
        "Name:BadVotePlayer\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Player | AdditionalVote$ 1",
        "Name:BadVoteZero\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ You | AdditionalVote$ 0",
        "Name:BadDungeonTwo\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ Flying | CheckSVar$ D | SVarCompare$ GE2\n\
         SVar:D:DungeonsCompleted$Amount",
        "Name:BadDungeonEq1\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ Flying | CheckSVar$ D | SVarCompare$ EQ1\n\
         SVar:D:DungeonsCompleted$Amount",
        "Name:BadBecomeMixed\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddType$ Enchantment & Creature & Goblin | \
         RemoveCardTypes$ True | RemoveCreatureTypes$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn non_mana_activated_ability_filter_lowers_in_untap_statics() {
    let card = read(
        "Name:Utility Web\nManaCost:6\nTypes:Enchantment\n\
         R:Event$ Untap | Layer$ CantHappen | ValidCard$ Land.hasNonManaActivatedAbility | ValidStepTurnToController$ You",
    );
    assert!(
        card.abilities.iter().any(|ability| {
            ability.contains(
                "static_ability!(Filter::And(&[Filter::LAND, Filter::HasNonManaActivatedAbility]), Modifier::DoesNotUntap)",
            )
        }),
        "abilities: {:?}",
        card.abilities
    );
}
