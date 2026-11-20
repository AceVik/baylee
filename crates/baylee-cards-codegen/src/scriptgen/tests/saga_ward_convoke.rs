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
         SVar:X:Count$Valid Creature.YouCtrl$CardPower"
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
        "S:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell | Caster$ Player",
        "S:Mode$ CastWithFlash | ValidSA$ Activated.Equip | Caster$ You",
        "S:Mode$ CastWithFlash | ValidCard$ Card.Self | ValidSA$ Spell | EffectZone$ All | Caster$ You",
        "S:Mode$ CastWithFlash | ValidCard$ Goblin | ValidSA$ Spell | Caster$ You",
        "S:Mode$ CastWithFlash | ValidCard$ Creature.Green | ValidSA$ Spell | Caster$ You",
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

    // Refusals: specific counter kind, unconditional doubler, effect-only additive, non-battlefield.
    assert!(refused(
        "Name:TypedCounter\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | ValidCounterType$ P1P1 | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ PlusC\n\
         SVar:PlusC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Plus.1"
    ));
    assert!(refused(
        "Name:UncondDoubler\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ DoubleC\n\
         SVar:DoubleC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Twice"
    ));
    assert!(refused(
        "Name:EffectOnlyPlus\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | EffectOnly$ True | \
         ValidCard$ Creature.YouCtrl+inZoneBattlefield | ReplaceWith$ PlusC\n\
         SVar:PlusC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Plus.1"
    ));
    assert!(refused(
        "Name:MissingZone\nTypes:Enchantment\n\
         R:Event$ AddCounter | ActiveZones$ Battlefield | \
         ValidCard$ Creature.YouCtrl | EffectOnly$ True | ReplaceWith$ DoubleC\n\
         SVar:DoubleC:DB$ ReplaceCounter | Amount$ X\n\
         SVar:X:ReplaceCount$CounterNum/Twice"
    ));
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

    // Refusals: Origin-scoped CantEnterBattlefield, opp-scoped EnterUntapped, non-!token GY exile.
    assert!(refused(
        "Name:CageOnly\nTypes:Artifact\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Graveyard | \
         Destination$ Battlefield | ValidCard$ Creature | Prevent$ True | Layer$ CantHappen"
    ));
    assert!(refused(
        "Name:OppUntap\nTypes:Enchantment\n\
         R:Event$ Moved | ValidCard$ Creature.OppCtrl | Destination$ Battlefield | \
         ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBUntapped\n\
         SVar:ETBUntapped:DB$ Untap | Defined$ ReplacedCard | ETB$ True"
    ));
    assert!(refused(
        "Name:TokenGyExile\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | ValidCard$ Card.YouOwn | \
         Destination$ Graveyard | ReplaceWith$ ExileYou\n\
         SVar:ExileYou:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ All | Destination$ Exile"
    ));
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

    // Subtype-filtered graveyard land permission stays refused.
    assert!(refused(
        "Name:ForestCrucible\nTypes:Artifact\n\
         S:Mode$ Continuous | AffectedZone$ Graveyard | Affected$ Forest.YouOwn | MayPlay$ True"
    ));
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

    // Refusals: per-attacker Max$, global IgnoreLegendRule, per-defender AttackRestrict.
    assert!(refused(
        "Name:SoloBlocker\nTypes:Creature\nPT:3/3\n\
         S:Mode$ MinMaxBlocker | ValidCard$ Card.Self | Max$ 1"
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
    assert!(refused(
        "Name:BadPair\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ G | RestrictValid$ Spell.Creature,Activated.Artifact"
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
         A:AB$ GainLife | Cost$ SubCounter<X/CHARGE> | LifeAmount$ X\n\
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
