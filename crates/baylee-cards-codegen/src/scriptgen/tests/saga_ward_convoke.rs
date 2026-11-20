//! Synthetic unit tests for `saga_ward_convoke.rs` keyword, entry-counter,
//! static, and replacement lowering rules and their refusal boundaries.

use super::*;

#[test]
fn suspend_lowers_sorcery_and_artifact_and_guards_creatures_and_flash() {
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

    // Creatures stay refused until post-suspend haste (CR 702.62a) is modeled.
    assert!(refused(
        "Name:SuspendedBeast\nManaCost:4 G\nTypes:Creature\nPT:4/4\nK:Suspend:3:1 G"
    ));
    // Instants and flash permanents stay refused while suspend is sorcery-speed only.
    assert!(refused(
        "Name:SuspendedInstant\nManaCost:1 U\nTypes:Instant\nK:Suspend:2:U\nA:SP$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:SuspendedFlashRelic\nManaCost:2\nTypes:Artifact\nK:Flash\nK:Suspend:2:1"
    ));
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
