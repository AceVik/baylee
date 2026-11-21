//! Synthetic tests for Wave 2 & Wave 3 scriptgen unlock rules.

use super::*;

#[test]
fn enforced_keywords_and_parameterized_keywords_lower_cleanly() {
    let kw = read(
        "Name:KeywordTest\nTypes:Creature\nPT:2/2\n\
         K:Infect\nK:Flanking\nK:Undying\nK:Persist\nK:Intimidate\n\
         K:Split second\nK:Phasing\nK:Rebound\nK:Ascend\nK:Daybound\nK:Nightbound",
    );
    for expected in [
        "KeywordSet::INFECT",
        "KeywordSet::FLANKING",
        "KeywordSet::UNDYING",
        "KeywordSet::PERSIST",
        "KeywordSet::INTIMIDATE",
        "KeywordSet::SPLIT_SECOND",
        "KeywordSet::PHASING",
        "KeywordSet::REBOUND",
        "KeywordSet::ASCEND",
        "KeywordSet::DAYBOUND",
        "KeywordSet::NIGHTBOUND",
    ] {
        assert!(
            kw.keywords.iter().any(|k| k == expected),
            "missing {expected}"
        );
    }

    let ward = read("Name:W\nTypes:Creature\nPT:2/2\nK:Ward:2");
    assert_eq!(ward.abilities, ["AbilityDef::Ward { mana: 2 }"]);
    assert!(refused("Name:W\nTypes:Creature\nPT:2/2\nK:Ward:11"));
    assert!(refused(
        "Name:W\nTypes:Creature\nPT:2/2\nK:Ward:Waterbend<4>"
    ));

    let crew = read("Name:V\nTypes:Artifact Vehicle\nPT:4/4\nK:Crew:3");
    assert_eq!(crew.abilities, ["crew!(3)"]);

    let saddle = read("Name:M\nTypes:Creature Mount\nPT:3/3\nK:Saddle:2");
    assert_eq!(saddle.abilities, ["saddle!(2)"]);

    let toxic = read("Name:T\nTypes:Creature\nPT:1/1\nK:Toxic:2");
    assert_eq!(toxic.abilities, ["AbilityDef::Toxic { poison: 2 }"]);
    assert!(refused("Name:T\nTypes:Creature\nPT:1/1\nK:Toxic:0"));

    let partner = read("Name:P\nTypes:Legendary Creature\nPT:2/2\nK:Partner");
    assert!(partner.has_partner);
    assert!(refused(
        "Name:P\nTypes:Legendary Creature\nPT:2/2\nK:Partner:Character select"
    ));

    let engines = read("Name:E\nTypes:Artifact\nK:Start your engines!");
    assert_eq!(engines.abilities, ["AbilityDef::StartYourEngines"]);
}

#[test]
fn face_cost_mechanics_and_ability_keywords() {
    let convoke = read("Name:C\nManaCost:3 G\nTypes:Sorcery\nK:Convoke\nA:SP$ Draw | NumCards$ 1");
    assert!(convoke.convoke);

    let delve = read("Name:D\nManaCost:5 U\nTypes:Sorcery\nK:Delve\nA:SP$ Draw | NumCards$ 2");
    assert!(delve.delve);

    let improvise =
        read("Name:I\nManaCost:4 U\nTypes:Instant\nK:Improvise\nA:SP$ Draw | NumCards$ 1");
    assert!(improvise.improvise);

    let rep =
        read("Name:R\nManaCost:1 U\nTypes:Instant\nK:Replicate:1 U\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(rep.replicate.as_deref(), Some("Some(mana!(\"{1}{U}\"))"));
    assert!(refused(
        "Name:R\nManaCost:1 U\nTypes:Instant\nK:Replicate:PayLife<2>\nA:SP$ Draw | NumCards$ 1"
    ));

    let disg = read("Name:Disg\nManaCost:2 G\nTypes:Creature\nPT:3/3\nK:Disguise:1 G");
    assert_eq!(disg.disguise.as_deref(), Some("Some(mana!(\"{1}{G}\"))"));

    let exalted = read("Name:Ex\nTypes:Creature\nPT:1/1\nK:Exalted");
    assert!(
        exalted.abilities[0].contains("Trigger::AttacksAlone(&Filter::YOUR_CREATURE)")
            && exalted.abilities[0].contains("Effect::PumpEventObject")
    );

    let cascade =
        read("Name:Cas\nManaCost:4 R\nTypes:Sorcery\nK:Cascade\nA:SP$ Draw | NumCards$ 1");
    assert!(
        cascade
            .abilities
            .iter()
            .any(|a| a == "triggered!(Trigger::SpellCast(&Filter::This), &[Effect::Cascade])")
    );

    let rampage = read("Name:Ramp\nTypes:Creature\nPT:3/3\nK:Rampage:2");
    assert!(
        rampage.abilities[0].contains("Trigger::Rampage")
            && rampage.abilities[0].contains("factor: 2")
    );

    let evoke = read(
        "Name:Ev\nManaCost:4 U\nTypes:Creature\nPT:2/2\nK:Evoke:2 U\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 2",
    );
    assert_eq!(evoke.alternative_costs.len(), 1);
    assert!(
        evoke
            .abilities
            .iter()
            .any(|a| a == "triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf])")
    );
    // Coexistence of Evoke with another AlternativeCost is refused.
    assert!(refused(
        "Name:Ev2\nManaCost:4 U\nTypes:Creature\nPT:2/2\nK:Evoke:2 U\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | Cost$ 0"
    ));
}

#[test]
fn saga_chapters_lower_in_order() {
    let saga = read(
        "Name:Story\nManaCost:2 W\nTypes:Enchantment Saga\n\
         K:Chapter:3:Ch1,Ch2,Ch3\n\
         SVar:Ch1:DB$ GainLife | LifeAmount$ 2\n\
         SVar:Ch2:DB$ Draw | NumCards$ 1\n\
         SVar:Ch3:DB$ Destroy | ValidTgts$ Creature",
    );
    assert_eq!(saga.abilities.len(), 3);
    assert_eq!(saga.abilities[0], "chapter!(1, &[Effect::gain_life(2)])");
    assert_eq!(saga.abilities[1], "chapter!(2, &[Effect::draw(1)])");
    assert!(
        saga.abilities[2].starts_with("chapter!(3, &[Effect::destroy(")
            && saga.abilities[2].contains("targets = Some(TargetReq::one(")
    );
    // Read Ahead + grouped multi-chapter sagas + second_targets lower cleanly.
    let read_ahead = read(
        "Name:Story2\nManaCost:2 W\nTypes:Enchantment Saga\n\
         K:Read ahead\nK:Chapter:3:Ch1,Ch1,Ch2\n\
         SVar:Ch1:DB$ Draw | NumCards$ 1\n\
         SVar:Ch2:DB$ Destroy | ValidTgts$ Artifact | SubAbility$ DBKill2\n\
         SVar:DBKill2:DB$ Destroy | ValidTgts$ Enchantment",
    );
    assert!(
        read_ahead
            .keywords
            .iter()
            .any(|k| k == "KeywordSet::READ_AHEAD"),
        "{:?}",
        read_ahead.keywords
    );
    assert_eq!(
        read_ahead.abilities[0],
        "chapters!(&[1, 2], &[Effect::draw(1)])"
    );
    assert!(
        read_ahead.abilities[1].starts_with("chapter!(3, &[Effect::destroy(")
            && read_ahead.abilities[1].contains("targets = Some(TargetReq::one(")
            && read_ahead.abilities[1].contains("second_targets = Some(TargetReq::one("),
        "{}",
        read_ahead.abilities[1]
    );
}

#[test]
fn modal_charms_activated_triggered_and_min_charm_num() {
    // MinCharmNum$ 1 | CharmNum$ 3 with <= 2 targeting modes (2nd is objects-only).
    let multi = read(
        "Name:Command\nManaCost:2 W W\nTypes:Sorcery\n\
         A:SP$ Charm | CharmNum$ 3 | MinCharmNum$ 1 | Choices$ M1,M2,M3\n\
         SVar:M1:DB$ GainLife | LifeAmount$ 3\n\
         SVar:M2:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 2\n\
         SVar:M3:DB$ Destroy | ValidTgts$ Artifact",
    );
    assert!(
        multi.abilities[0].contains("ModeCount { min: 1, max: 3 }"),
        "{}",
        multi.abilities[0]
    );

    // Multi-mode charm whose 2nd targeting mode targets a player violates `modes_fault` and refuses.
    assert!(refused(
        "Name:BadCommand\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Charm | CharmNum$ 2 | Choices$ M1,M2\n\
         SVar:M1:DB$ Destroy | ValidTgts$ Artifact\n\
         SVar:M2:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 2"
    ));

    // Modal activated ability (`AB$ Charm`).
    let act = read(
        "Name:Staff\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ T | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | LifeAmount$ 1\n\
         SVar:M2:DB$ Scry | ScryNum$ 1",
    );
    assert!(act.abilities[0].starts_with("modal_activated!(Cost::TAP, &["));

    // Modal triggered ability (`DB$ Charm` in `Execute$`).
    let trig = read(
        "Name:Beacon\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigCharm\n\
         SVar:TrigCharm:DB$ Charm | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | LifeAmount$ 2\n\
         SVar:M2:DB$ Draw | NumCards$ 1",
    );
    assert!(trig.abilities[0].starts_with("modal_triggered!(Trigger::ETB, &["));
}

#[test]
fn dig_impulse_draw_and_exile_play_permissions() {
    let impulse = read(
        "Name:ImpulseDraw\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | Duration$ UntilTheEndOfYourNextTurn | StaticAbilities$ MayPlay | SubAbility$ DBCleanup\n\
         SVar:MayPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        impulse.abilities[0].contains("Effect::ExileTopToPlay")
            && impulse.abilities[0].contains("count: 2")
            && impulse.abilities[0].contains("PlayWindow::ThroughYourNextTurn")
            && impulse.abilities[0].contains("spell_types: None"),
        "{}",
        impulse.abilities[0]
    );

    let nonland = read(
        "Name:NonlandCast\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ MayPlay\n\
         SVar:MayPlay:Mode$ Continuous | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile | MayPlay$ True",
    );
    assert!(
        nonland.abilities[0]
            .contains("Effect::ExileTopMayCast { who: PlayerRel::You, nonland_only: true }"),
        "{}",
        nonland.abilities[0]
    );
}

#[test]
fn counters_proliferate_transform_and_pump_extensions() {
    let put_all = read(
        "Name:Boost\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(put_all.abilities[0].contains("Effect::AddCounterFilter"));

    let energy = read(
        "Name:Charge\nManaCost:U\nTypes:Instant\n\
         A:SP$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 3",
    );
    assert!(
        energy.abilities[0]
            .contains("Effect::Energy { who: PlayerRel::You, amount: Amount::Fixed(3) }")
    );

    let prolif = read("Name:Spread\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Proliferate");
    assert_eq!(prolif.abilities, ["spell!(&[Effect::Proliferate])"]);

    let transform = read(
        "Name:Flipper\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ SetState | Cost$ 3 | Defined$ Self | Mode$ Transform",
    );
    assert!(transform.abilities[0].contains("Effect::TransformSource"));

    let switch_pt = read(
        "Name:Amoeba\nManaCost:1 U\nTypes:Creature\nPT:1/3\n\
         A:AB$ Pump | Cost$ Discard<1/Card> | Defined$ Self | KW$ HIDDEN CARDNAME's power and toughness are switched",
    );
    assert!(switch_pt.abilities[0].contains("Modifier::SwitchPT"));

    let pump_player = read(
        "Name:Day\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ PumpAll | ValidTgts$ Player | ValidCards$ Creature | NumAtt$ +1 | NumDef$ +1",
    );
    assert!(
        pump_player.abilities[0].contains("controlled_by: Some(PlayerRel::Chosen)"),
        "{}",
        pump_player.abilities[0]
    );

    let chaos = read(
        "Name:Chaos\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ NoBlock\n\
         SVar:NoBlock:Mode$ Continuous | Affected$ Creature | AddHiddenKeyword$ CARDNAME can't block.",
    );
    assert!(
        chaos.abilities[0].contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
        "{}",
        chaos.abilities[0]
    );

    let exile_gy = read(
        "Name:Crypt\nManaCost:0\nTypes:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ T Sac<1/CARDNAME> | ValidTgts$ Player | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card",
    );
    assert!(
        exile_gy.abilities[0].contains("Effect::ExileGraveyard { player: PlayerRel::Chosen }"),
        "{}",
        exile_gy.abilities[0]
    );
}

#[test]
fn new_triggers_and_svar_conditions() {
    let atk = read(
        "Name:Raid\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ You | ValidAttackers$ Creature.YouCtrl | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(atk.abilities[0].contains("Trigger::OneOrMoreAttack("));

    let atk3 = read(
        "Name:Battalion\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ You | ValidAttackersAmount$ GE3 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(atk3.abilities[0].contains("Trigger::AttackWithAtLeast(3)"));

    let life = read(
        "Name:Soul\nTypes:Creature\nPT:2/2\n\
         T:Mode$ LifeGained | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigCounter\n\
         SVar:TrigCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(life.abilities[0].contains("Trigger::GainsLife(PlayerRel::You)"));

    let disc = read(
        "Name:OppDiscard\nTypes:Creature\nPT:4/4\n\
         T:Mode$ Discarded | ValidCard$ Card.Self | ValidCause$ SpellAbility.OppCtrl | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(disc.abilities[0].contains("Trigger::DiscardedThisByOpponent"));

    let morbid = read(
        "Name:MorbidEnd\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | CheckSVar$ X | SVarCompare$ GE1 | Execute$ TrigDraw\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        morbid.abilities[0].contains("Condition::CreaturesDiedThisTurnAtLeast(1)"),
        "{}",
        morbid.abilities[0]
    );

    let metal = read(
        "Name:Metal\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 2 | Condition$ Metalcraft",
    );
    assert!(
        metal.abilities[0].contains("Condition::ControlCount(&Filter::ARTIFACT, 3)"),
        "{}",
        metal.abilities[0]
    );

    let x_cond = read(
        "Name:XCheck\nManaCost:X U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ Bonus\n\
         SVar:Bonus:DB$ GainLife | LifeAmount$ 3 | ConditionCheckSVar$ X | ConditionSVarCompare$ GE5\n\
         SVar:X:Count$xPaid",
    );
    assert!(x_cond.abilities[0].contains("Condition::XAtLeast(5)"));
}

#[test]
fn wave4_drift_fixes() {
    // 1) ControlledBy TriggeredPlayer under Phase trigger -> ControlledByActivePlayer.
    let phase_count = read(
        "Name:PhaseCheck\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:X:Count$Valid Mountain.ControlledBy TriggeredPlayer",
    );
    assert!(
        phase_count
            .statics
            .contains("Filter::ControlledByActivePlayer"),
        "{}",
        phase_count.statics
    );
    let phase_tgt = read(
        "Name:PhaseTap\nManaCost:4 W\nTypes:Creature\nPT:3/3\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | ValidTgts$ Creature.ControlledBy TriggeredPlayer",
    );
    assert!(
        phase_tgt
            .statics
            .contains("Filter::ControlledByActivePlayer"),
        "{}",
        phase_tgt.statics
    );
    // Refused outside Phase triggers.
    assert!(refused(
        "Name:BadControlledBy\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Tap | ValidTgts$ Creature.ControlledBy TriggeredPlayer"
    ));

    // 2) DB$ Bookkeeping | ForgetImprinted$ cleanup trigger is ignored cleanly.
    let imprint = read(
        "Name:Imprinter\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | OptionalDecider$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | Imprint$ True\n\
         T:Mode$ ChangesZone | ValidCard$ Card.IsImprinted+ExiledWithSource | Origin$ Exile | Static$ True | Execute$ DBForget\n\
         SVar:DBForget:DB$ Bookkeeping | ForgetImprinted$ TriggeredCard\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Any | Static$ True | Execute$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True",
    );
    assert_eq!(imprint.abilities.len(), 1);

    // 3) Standalone SP$ TargetOnly lowers to no-op PumpTarget (+0/+0).
    let target_only = read(
        "Name:PointAt\nManaCost:0\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Permanent",
    );
    assert!(
        target_only.abilities[0].contains("Effect::PumpTarget")
            && target_only.abilities[0].contains("TargetSpec::Object(&Filter::Any)"),
        "{}",
        target_only.abilities[0]
    );

    // 4) OptionalDecider$ You + DB$ TapOrUntap emits a single MayDo, never double-nested.
    let toggle = read(
        "Name:ToggleSpirit\nManaCost:3 U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Wizard | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigToggle\n\
         SVar:TrigToggle:DB$ TapOrUntap | ValidTgts$ Creature",
    );
    assert_eq!(
        toggle.abilities[0].matches("Effect::MayDo").count(),
        1,
        "{}",
        toggle.abilities[0]
    );
}

#[test]
fn wave4_fight_bite_and_poison() {
    // 5) Two-target Fight (with TargetUnique + up_to_one) and self-ETB Fight.
    let fight_two = read(
        "Name:DuelSpell\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature | TargetUnique$ True | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        fight_two.abilities[0]
            .contains("Effect::Fight { fighter: TargetSlot::First, foe: TargetSlot::Second }")
            && fight_two.abilities[0].contains("second_targets = Some(TargetReq::up_to_one(")
            && fight_two
                .statics
                .contains("Filter::Not(&Filter::Reference)"),
        "{}\n{}",
        fight_two.statics,
        fight_two.abilities[0]
    );

    let etb_fight = read(
        "Name:Fighter\nManaCost:3 G\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigFight\n\
         SVar:TrigFight:DB$ Fight | Defined$ TriggeredCardLKICopy | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        etb_fight.abilities[0]
            .contains("Effect::Fight { fighter: TargetSlot::This, foe: TargetSlot::First }")
            && etb_fight.abilities[0].contains("targets = Some(TargetReq::up_to_one("),
        "{}",
        etb_fight.abilities[0]
    );

    // 6) Bite spell + Poison follow-up + self-bite + EventObjectDealsDamage*.
    let bite = read(
        "Name:ToxicBite\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature.YouDontCtrl | DamageSource$ ParentTarget | NumDmg$ X | SubAbility$ DBPoison\n\
         SVar:DBPoison:DB$ Poison | Defined$ Player.Opponent | Num$ 1\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        bite.abilities[0].contains(
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::Second }"
        ) && bite.abilities[0]
            .contains("Effect::Poison { who: PlayerRel::Opponent, amount: Amount::Fixed(1) }"),
        "{}",
        bite.abilities[0]
    );

    let self_bite = read(
        "Name:SelfHit\nManaCost:R W\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | DamageSource$ Targeted | NumDmg$ X\n\
         SVar:X:Targeted$CardPower",
    );
    assert!(
        self_bite.abilities[0].contains(
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::First }"
        ),
        "{}",
        self_bite.abilities[0]
    );

    let surge = read(
        "Name:Surge\nManaCost:4 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | DamageSource$ TriggeredCard\n\
         SVar:X:TriggeredCard$CardPower",
    );
    assert!(
        surge.abilities[0].contains(
            "Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::AnyTarget }"
        ),
        "{}",
        surge.abilities[0]
    );

    let fixed_ev = read(
        "Name:Bow\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ Opponent | NumDmg$ 1 | DamageSource$ TriggeredCardLKICopy",
    );
    assert!(
        fixed_ev.abilities[0].contains(
            "Effect::EventObjectDealsDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::Opponent) }"
        ),
        "{}",
        fixed_ev.abilities[0]
    );

    let unblock_poison = read(
        "Name:Mosquito\nManaCost:1 B\nTypes:Creature\nPT:0/1\n\
         T:Mode$ AttackerUnblocked | ValidCard$ Card.Self | Execute$ TrigPoison\n\
         SVar:TrigPoison:DB$ Poison | Defined$ TriggeredDefendingPlayer | Num$ 1",
    );
    assert!(
        unblock_poison.abilities[0].contains(
            "Effect::Poison { who: PlayerRel::DefendingPlayer, amount: Amount::Fixed(1) }"
        ),
        "{}",
        unblock_poison.abilities[0]
    );
}

#[test]
fn wave5_amounts_statics_and_counters() {
    // 1) Arithmetic suffixes and new base counts.
    let double_x = read(
        "Name:Twice\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ Y\n\
         SVar:Y:Count$xPaid/Times.2",
    );
    assert!(double_x.abilities[0].contains("Amount::DoubleX"));

    let arith = read(
        "Name:HalfPlus\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ Z\n\
         SVar:X:Count$Valid Creature.YouCtrl/HalfDown\n\
         SVar:Z:SVar$X/Plus.1",
    );
    assert!(
        arith.abilities[0].contains("Amount::Plus { base: &Amount::Half {"),
        "{}",
        arith.abilities[0]
    );

    let players = read(
        "Name:EachOpp\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\n\
         SVar:X:PlayerCountOpponents$Amount",
    );
    assert!(
        players.abilities[0].contains("Amount::Players(PlayerRel::Opponent)"),
        "{}",
        players.abilities[0]
    );

    // 2) Single-sided CDA (`*/4` and `2/1+*`) and `CardTypesInAllGraveyards`.
    let half_cda = read(
        "Name:StarFour\nManaCost:2 G\nTypes:Creature\nPT:*/4\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        half_cda.abilities[0].contains("Modifier::CharacteristicPTSeparate")
            && half_cda.abilities[0].contains("toughness: PtCount::Fixed(4)"),
        "{}",
        half_cda.abilities[0]
    );

    let goyf = read(
        "Name:GraveStar\nManaCost:1 G\nTypes:Creature\nPT:*/1+*\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ Y\n\
         SVar:X:Count$ValidGraveyard Card$CardTypes\n\
         SVar:Y:SVar$X/Plus.1",
    );
    assert!(
        goyf.abilities[0].contains("PtCount::CardTypesInAllGraveyards")
            && goyf.abilities[0].contains("toughness_plus: 1"),
        "{}",
        goyf.abilities[0]
    );

    // 3) Dynamic static P/T bonus (positive & negative) + single-sided SetToughness.
    let dyn_anthem = read(
        "Name:DynamicLord\nManaCost:2 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ X | AddToughness$ -X\n\
         SVar:X:Count$Domain",
    );
    assert!(
        dyn_anthem.abilities[0].contains("Modifier::ModifyPTByAmount")
            && dyn_anthem.abilities[0].contains("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)")
            && dyn_anthem.abilities[0].contains("Amount::Negated("),
        "{}",
        dyn_anthem.abilities[0]
    );

    // 4) Stun / Shield / Finality / Defense counters + ValidTgtsDesc label guard.
    let stun = read(
        "Name:FreezeTap\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ PutCounter | ValidTgts$ Creature | ValidTgtsDesc$ target creature | CounterType$ STUN | CounterNum$ 1",
    );
    assert!(stun.abilities[0].contains("CounterKind::Stun"));
    assert!(refused(
        "Name:OrphanDesc\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | ValidTgtsDesc$ target creature"
    ));
}

#[test]
fn wave5_effects_blink_support_adapt_attach_and_reanimate() {
    // 1) SuppressUntap (Pump + PumpAll) and MustBeBlockedByAllAble + AsLongAsControl.
    let freeze = read(
        "Name:Chiller\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | Duration$ Permanent | KW$ HIDDEN This card doesn't untap during your next untap step.",
    );
    assert!(
        freeze.abilities[0].contains("Effect::SuppressUntap")
            && freeze.abilities[0].contains("steps: 1"),
        "{}",
        freeze.abilities[0]
    );

    let freeze_all = read(
        "Name:Blizzard\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ PumpAll | ValidCards$ Creature.OppCtrl | Duration$ Permanent | KW$ HIDDEN This card doesn't untap during your next untap step.",
    );
    assert!(freeze_all.abilities[0].contains("Effect::SuppressUntap"));

    let lure_pump = read(
        "Name:Taunt\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ HIDDEN All creatures able to block CARDNAME do so.",
    );
    assert!(lure_pump.abilities[0].contains("Modifier::MustBeBlockedByAllAble"));

    // 2) Support, Adapt, Attach.
    let support = read(
        "Name:BolsterAllies\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ PutCounter | Support$ 2",
    );
    assert!(
        support.abilities[0].contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)")
            && support.abilities[0].contains("CounterKind::P1P1"),
        "{}",
        support.abilities[0]
    );

    let adapt = read(
        "Name:Evolver\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ PutCounter | Cost$ 2 G | Adapt$ 2",
    );
    assert!(
        adapt.abilities[0]
            .contains("Condition::Not(&Condition::CountersOnSelf(CounterKind::P1P1, 1))")
            && adapt.abilities[0].contains("Amount::Fixed(2)"),
        "{}",
        adapt.abilities[0]
    );

    let attach = read(
        "Name:SnapOn\nManaCost:W\nTypes:Artifact Equipment\n\
         A:AB$ Attach | Cost$ 1 | ValidTgts$ Creature.YouCtrl",
    );
    assert!(attach.abilities[0].contains("Effect::AttachSelf"));

    // 3) Immediate single-target and multi-target Blink (`Effect::Blink` loops
    // over all surviving targets, CR 400.7 / CR 608.2c).
    let blink = read(
        "Name:Flicker\nManaCost:W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | GainControl$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        blink.abilities[0].contains("Effect::Blink")
            && blink.abilities[0].contains("owner_control: false"),
        "{}",
        blink.abilities[0]
    );
    let multi_blink = read(
        "Name:MultiFlicker\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 2 | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ All | Destination$ Battlefield",
    );
    assert!(
        multi_blink.abilities[0].contains("Effect::Blink")
            && multi_blink.abilities[0].contains("owner_control: true")
            && multi_blink.abilities[0].contains("TargetReq::up_to(")
            && multi_blink.abilities[0].contains(", 2)"),
        "{}",
        multi_blink.abilities[0]
    );

    // 4) Graveyard -> Battlefield tapped / counters / mass reanimate.
    let reanimate = read(
        "Name:RiseTapped\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature | TgtZone$ Graveyard | Tapped$ True | WithCountersType$ FINALITY | WithCountersAmount$ 1",
    );
    assert!(
        reanimate.abilities[0].contains("Effect::GraveyardToBattlefieldModified")
            && reanimate.abilities[0].contains("CounterKind::Finality"),
        "{}",
        reanimate.abilities[0]
    );

    let mass = read(
        "Name:MassRise\nManaCost:5 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Creature.YouOwn | Tapped$ True",
    );
    assert!(
        mass.abilities[0].contains("Effect::YourGraveyardToBattlefield")
            && mass.abilities[0].contains("tapped: true"),
        "{}",
        mass.abilities[0]
    );
}

#[test]
fn wave5_triggers_zones_and_svar_conditions() {
    // 1) TriggerZones$ Graveyard & Exile + self graveyard IsPresent without PresentPlayer.
    let gy_trig = read(
        "Name:CryptCrawler\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | TriggerZones$ Graveyard | IsPresent$ Card.StrictlySelf | PresentZone$ Graveyard | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        gy_trig.abilities[0].contains("zone = TriggerZone::Graveyard")
            && gy_trig.abilities[0].contains("ZoneRef::Graveyard"),
        "{}",
        gy_trig.abilities[0]
    );

    // 2) Drawn with Number$ 2 + SpellCast NthSpellCast & FirstNoncreatureSpellCast.
    let draw2 = read(
        "Name:SecondDraw\nManaCost:1 U\nTypes:Creature\nPT:1/3\n\
         T:Mode$ Drawn | ValidCard$ Card.YouCtrl | Number$ 2 | TriggerZones$ Battlefield | Execute$ TrigCounter\n\
         SVar:TrigCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        draw2.abilities[0].contains("Trigger::NthCardDrawnInTurn { who: PlayerRel::You, n: 2 }"),
        "{}",
        draw2.abilities[0]
    );

    let flurry = read(
        "Name:Flurry\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Card.YouCtrl | ValidActivatingPlayer$ You | ActivatorThisTurnCast$ EQ2 | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        flurry.abilities[0]
            .contains("Trigger::NthSpellCast { n: 2, filter: &Filter::ControlledByYou }"),
        "{}",
        flurry.abilities[0]
    );
    // Restricted ValidCard$ with ActivatorThisTurnCast$ EQ2 must refuse!
    assert!(refused(
        "Name:BadFlurry\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You | ActivatorThisTurnCast$ EQ2 | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));

    // 3) OneOrMoreDealCombatDamageToPlayer, DealsCombatDamageToPlayerOrPlaneswalker,
    //    TargetedByOpponent with You + permanents, CounterAddedOnce.
    let saboteur = read(
        "Name:TeamHit\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ DamageDoneOnce | CombatDamage$ True | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        saboteur.abilities[0].contains("Trigger::OneOrMoreDealCombatDamageToPlayer("),
        "{}",
        saboteur.abilities[0]
    );

    let leo = read(
        "Name:Guard\nManaCost:2 U\nTypes:Creature\nPT:3/3\n\
         T:Mode$ BecomesTarget | ValidTarget$ You,Permanent.YouCtrl+inZoneBattlefield | ValidSource$ SpellAbility.OppCtrl | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        leo.abilities[0].contains("Trigger::TargetedByOpponent")
            && leo.abilities[0].contains("you: true"),
        "{}",
        leo.abilities[0]
    );

    let counter_trig = read(
        "Name:GrowthWatch\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ CounterAddedOnce | ValidCard$ Card.Self | CounterType$ P1P1 | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        counter_trig.abilities[0].contains("Trigger::CountersPlaced")
            && counter_trig.abilities[0].contains("CounterKind::P1P1"),
        "{}",
        counter_trig.abilities[0]
    );

    // 4) Generic static_count_expr Condition::AmountAtLeast & CastInstantOrSorceryThisTurn.
    let cond = read(
        "Name:DomainTrigger\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | CheckSVar$ X | SVarCompare$ GE5 | Execute$ TrigDraw\n\
         SVar:X:Count$Domain\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        cond.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::BasicLandTypesAmong(&Filter::YOUR_LAND), 5)"
        ),
        "{}",
        cond.abilities[0]
    );

    // 5) Branch with svar_condition + Gond Gate Land.OppCtrl (`Filter::ControlledByOpponent`).
    let branch = read(
        "Name:CastCheck\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ D1 | FalseSubAbility$ D2\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl\n\
         SVar:D1:DB$ Draw | NumCards$ 2\n\
         SVar:D2:DB$ Draw | NumCards$ 1",
    );
    assert!(
        branch.abilities[0].contains("Condition::CastInstantOrSorceryThisTurn"),
        "{}",
        branch.abilities[0]
    );
}

#[test]
fn wave3_trigger_optional_flag_and_prose_you_may() {
    // Explicit `Optional$ Yes` on `ChangesZone` wraps the trigger body in `Effect::MayDo`.
    let opt_yes = read(
        "Name:OptYes\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
         | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield \
         | Optional$ Yes | Execute$ TrigBuf | TriggerDescription$ Whenever a creature enters, you may put two counters on it.\n\
         SVar:TrigBuf:DB$ PutCounter | Defined$ TriggeredCardLKICopy | CounterType$ P1P1 | CounterNum$ 2",
    );
    assert!(
        opt_yes.abilities[0].contains("Effect::MayDo { effects: &[Effect::AddCounterFilter"),
        "{}",
        opt_yes.abilities[0]
    );

    // Explicit `Optional$ No` does not wrap; non-boolean `Optional$` is honestly refused.
    let opt_no = read(
        "Name:OptNo\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
         | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield \
         | Optional$ No | Execute$ TrigBuf | TriggerDescription$ Whenever a creature enters, put a counter on it.\n\
         SVar:TrigBuf:DB$ PutCounter | Defined$ TriggeredCardLKICopy | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        !opt_no.abilities[0].contains("MayDo"),
        "{}",
        opt_no.abilities[0]
    );
    assert!(refused(
        "Name:OptBad\nManaCost:1 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
         | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield \
         | Optional$ Opponent | Execute$ TrigBuf\n\
         SVar:TrigBuf:DB$ GainLife | LifeAmount$ 1"
    ));

    // Single-clause prose ", you may ..." without `OptionalDecider$` wraps in `MayDo`.
    let prose_single = read(
        "Name:ProseSingle\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
         | ValidCard$ Creature.powerGE5+YouCtrl | TriggerZones$ Battlefield \
         | Execute$ TrigBuf | TriggerDescription$ Whenever a large creature enters, you may put two +1/+1 counters on it.\n\
         SVar:TrigBuf:DB$ PutCounter | Defined$ TriggeredCardLKICopy | CounterType$ P1P1 | CounterNum$ 2",
    );
    assert!(
        prose_single.abilities[0].contains("Effect::MayDo { effects: &[Effect::AddCounterFilter"),
        "{}",
        prose_single.abilities[0]
    );

    // Multi-clause / conditional prose ", you may ... If you do, ..." without `OptionalDecider$` refuses.
    assert!(refused(
        "Name:ProseMulti\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
         | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield \
         | Execute$ TrigDraw | TriggerDescription$ Whenever a creature enters, you may discard a card. If you do, draw a card.\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave3_effect_api_statics_and_dont_untap() {
    let unblockable = read(
        "Name:ShadowStep\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ NoBlock\n\
         SVar:NoBlock:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | ValidBlocker$ Creature.powerGE3",
    );
    assert!(
        unblockable.abilities[0]
            .contains("Effect::ModifyTargets { modifier: Modifier::CantBeBlockedBy(")
            && unblockable.abilities[0].contains("Duration::UntilEndOfTurn"),
        "{}",
        unblockable.abilities[0]
    );

    let goad_like = read(
        "Name:ForcedFight\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Effect | Cost$ R T | ValidTgts$ Creature | RememberObjects$ Targeted \
         | ForgetOnMoved$ Battlefield | StaticAbilities$ MustAtk\n\
         SVar:MustAtk:Mode$ MustAttack | ValidCreature$ Card.IsRemembered",
    );
    assert!(
        goad_like.abilities[0]
            .contains("Effect::ModifyTargets { modifier: Modifier::AttacksEachCombat"),
        "{}",
        goad_like.abilities[0]
    );

    let no_regen = read(
        "Name:Scythe\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ NR\n\
         SVar:NR:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered",
    );
    assert!(
        no_regen.abilities[0].contains("Modifier::CantBeRegenerated"),
        "{}",
        no_regen.abilities[0]
    );

    let no_max = read(
        "Name:OpenMind\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ Hand\n\
         SVar:Hand:Mode$ Continuous | Affected$ You | NoHandSize$ True",
    );
    assert!(
        no_max.abilities[0].contains(
            "Effect::continuous(&Filter::Any, Modifier::NoMaxHandSize, Duration::UntilYourNextTurn)"
        ),
        "{}",
        no_max.abilities[0]
    );

    let lock = read(
        "Name:FrostPin\nManaCost:2 U\nTypes:Creature\nPT:1/3\n\
         A:AB$ Effect | Cost$ T | ValidTgts$ Creature | RememberObjects$ Targeted \
         | Duration$ UntilUntaps | ReplacementEffects$ DontUntap\n\
         SVar:DontUntap:Event$ Untap | ValidCard$ Card.IsRemembered | ValidStepTurnToController$ You | Layer$ CantHappen",
    );
    assert!(
        lock.abilities[0].contains("Modifier::DoesNotUntap")
            && lock.abilities[0]
                .contains("Duration::WhileSourceTapped { control: false, power_limited: false }"),
        "{}",
        lock.abilities[0]
    );

    let freeze_self = read(
        "Name:Overexert\nManaCost:2 R\nTypes:Creature\nPT:3/3\n\
         A:AB$ Effect | Cost$ T | RememberObjects$ Self | Duration$ UntilTheEndOfYourNextUntap \
         | ReplacementEffects$ DontUntap\n\
         SVar:DontUntap:Event$ Untap | ValidCard$ Card.IsRemembered | ValidStepTurnToController$ Player.Activator | Layer$ CantHappen",
    );
    assert!(
        freeze_self.abilities[0].contains("Effect::ModifySource { modifier: Modifier::DoesNotUntap, duration: Duration::UntilYourNextUntapStep }"),
        "{}",
        freeze_self.abilities[0]
    );
}

#[test]
fn wave3_must_block_exile_until_and_trigger_prices() {
    let lure = read(
        "Name:LureBeast\nManaCost:3 G\nTypes:Creature\nPT:4/4\n\
         A:AB$ MustBlock | Cost$ 1 G | ValidTgts$ Creature | Duration$ UntilEndOfCombat",
    );
    assert!(
        lure.abilities[0].contains("Effect::MustBlockSourceIfAbleThisCombat"),
        "{}",
        lure.abilities[0]
    );

    let banisher = read(
        "Name:CageSpirit\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.OppCtrl | Origin$ Battlefield | Destination$ Exile | Duration$ UntilHostLeavesPlay",
    );
    assert!(
        banisher.abilities[0].contains("Effect::exile_until(")
            && banisher.abilities[0].contains("ExileUntil::SourceLeavesBattlefield"),
        "{}",
        banisher.abilities[0]
    );
    // Multi-target `UntilHostLeavesPlay` stays refused (`ExileLinked` stores one target).
    assert!(refused(
        "Name:BadCage\nManaCost:4 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.OppCtrl | TargetMin$ 2 | TargetMax$ 2 \
         | Origin$ Battlefield | Destination$ Exile | Duration$ UntilHostLeavesPlay"
    ));

    // Trigger-executed `AB$` with single object cost or `Sac<1/CARDNAME>` lowers to `PlayerMayPayCostThen`.
    let sac_trig = read(
        "Name:AltarSprite\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
         | OptionalDecider$ You | Execute$ TrigSac\n\
         SVar:TrigSac:AB$ Draw | Cost$ Sac<1/CARDNAME/this creature> | NumCards$ 2",
    );
    assert!(
        sac_trig.abilities[0].contains("Effect::PlayerMayPayCostThen")
            && sac_trig.abilities[0].contains("CostPart::Sacrifice(&Filter::This)"),
        "{}",
        sac_trig.abilities[0]
    );
    // Trailing mana after `Sac<1/CARDNAME/...>` must NOT be silently dropped.
    assert!(refused(
        "Name:BadSacTrig\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
         | OptionalDecider$ You | Execute$ TrigSac\n\
         SVar:TrigSac:AB$ Draw | Cost$ Sac<1/CARDNAME/this creature> 2 | NumCards$ 2"
    ));

    // Switched `UnlessCost$` with `PayLife` and `UnlessResolveSubs$ WhenPaid`.
    let life_unless = read(
        "Name:BloodChoice\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | UnlessCost$ PayLife<2> | UnlessPayer$ You | UnlessSwitched$ True \
         | UnlessResolveSubs$ WhenPaid | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | LifeAmount$ 1",
    );
    assert!(
        life_unless.abilities[0].contains("Effect::PlayerMayPayLifeThen"),
        "{}",
        life_unless.abilities[0]
    );
}

#[test]
fn wave3_mana_amounts_costs_and_activation_svar_checks() {
    let minus_amt = read(
        "Name:HandDiff\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Number$7/Minus.Y\n\
         SVar:Y:Count$CardsInYourHand",
    );
    assert!(
        minus_amt.abilities[0].contains(
            "Amount::ConstantMinus { constant: 7, subtract: &Amount::HandSize(PlayerRel::You) }"
        ),
        "{}",
        minus_amt.abilities[0]
    );

    let colors_amt = read(
        "Name:PrismGain\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$Colors",
    );
    assert!(
        colors_amt.abilities[0].contains("Amount::DistinctColorsAmong("),
        "{}",
        colors_amt.abilities[0]
    );

    let energy_act = read(
        "Name:SparkCell\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ PayEnergy<2> T | NumCards$ 1 | CheckSVar$ X | SVarCompare$ GE2\n\
         SVar:X:Count$YourEnergyCounters",
    );
    assert!(
        energy_act.abilities[0].contains("PayEnergy(2)")
            && energy_act.abilities[0].contains("Amount::CountersOnYou(CounterKind::Energy)"),
        "{}",
        energy_act.abilities[0]
    );

    let top_exile_spell = read(
        "Name:DeckBurn\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | Cost$ R ExileFromTop<2/Card> | ValidTgts$ Any | NumDmg$ 3",
    );
    assert_eq!(
        top_exile_spell.mandatory_additional_costs,
        ["CostPart::ExileTopOfLibrary(2)"]
    );

    let draw_both = read(
        "Name:SharedPage\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Draw | ValidTgts$ Player | Defined$ TargetedAndYou | NumCards$ 1",
    );
    assert!(
        draw_both.abilities[0].contains("Effect::draw(1)")
            && draw_both.abilities[0].contains(
                "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::Chosen }"
            ),
        "{}",
        draw_both.abilities[0]
    );

    let dmg_enchanted = read(
        "Name:BurnAura\nManaCost:1 R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigHit\n\
         SVar:TrigHit:DB$ DealDamage | Defined$ Enchanted | NumDmg$ 1",
    );
    let joined = dmg_enchanted.abilities.join("\n");
    assert!(joined.contains("Effect::DealDamageToAttached"), "{joined}");
}

#[test]
fn wave4_wired_spell_costs_and_prevention_helpers() {
    // Wired `kicker::parse_spell_cost_parts` unlocks `Sac<X/...>` and `Discard<2/...>` on spells.
    let sac_x = read(
        "Name:SacXSpell\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 B Sac<X/Creature> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        sac_x
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("CostPart::SacrificeX(")),
        "{:?}",
        sac_x.mandatory_additional_costs
    );

    // Wired `prevention::prevention_effect_shield` unlocks standalone `Effect | ReplacementEffects$` shields.
    let carpet = read(
        "Name:SkyCarpet\nManaCost:5\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ 5 T | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | ValidTarget$ You | ValidSource$ Creature.attacking",
    );
    assert!(
        carpet.abilities[0].contains("Effect::PreventAllDamageFromMatchingThisTurn"),
        "{}",
        carpet.abilities[0]
    );

    // Wired `prevention::prevent_from_chosen_source_chained` unlocks targeted `ChooseSource -> DB$ Effect`.
    let warder = read(
        "Name:TargetedShield\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         A:AB$ ChooseSource | Cost$ W T | Choices$ Card,Emblem | SubAbility$ DBShield\n\
         SVar:DBShield:DB$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | ForgetOnMoved$ Battlefield \
         | ReplacementEffects$ RPrev | ConditionDefined$ ChosenCard | ConditionPresent$ Card,Emblem | SubAbility$ DBCleanup\n\
         SVar:RPrev:Event$ DamageDone | ActiveZones$ Command | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | ValidTarget$ Card.IsRemembered | PreventionEffect$ True | ReplaceWith$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        warder.abilities[0].contains("Effect::PreventFromChosenSource")
            && warder.abilities[0].contains("target = Some(TargetSpec::Object(&Filter::CREATURE))"),
        "{}",
        warder.abilities[0]
    );
}

#[test]
fn wave4_triggers_secondary_controlled_blocked_and_multi_sac_price() {
    // Hoisted `Secondary$ True` works on `Attacks` / `ChangesZone` triggers.
    let dual = read(
        "Name:DualTitan\nManaCost:4 R\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Secondary$ True | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert_eq!(dual.abilities.len(), 2, "{:?}", dual.abilities);

    // `AttackerBlocked` on non-self lowers to `Trigger::ControlledBecomesBlocked`.
    let team_blk = read(
        "Name:PhalanxLeader\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackerBlocked | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackerLKICopy | NumAtt$ +1 | NumDef$ +1",
    );
    assert!(
        team_blk.abilities[0].contains("Trigger::ControlledBecomesBlocked(")
            && team_blk.abilities[0].contains("Effect::PumpEventObject"),
        "{}",
        team_blk.abilities[0]
    );

    // Multi-sacrifice trigger price lowers to `Effect::PlayerMaySacrificeNThen`.
    let altar = read(
        "Name:TwinAltar\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigSac\n\
         SVar:TrigSac:AB$ Draw | Cost$ Sac<2/Creature.YouCtrl> | NumCards$ 2",
    );
    assert!(
        altar.abilities[0].contains("Effect::PlayerMaySacrificeNThen")
            && altar.abilities[0].contains("count: 2"),
        "{}",
        altar.abilities[0]
    );

    // `AttackerBlockedByCreature` paired damage to blocker/attacker + controller + `DamageResolve`.
    let hound = read(
        "Name:BlockHound\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature | ValidBlocker$ Card.Self | Execute$ TrigAtk\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Secondary$ True | Execute$ TrigBlk\n\
         SVar:TrigAtk:DB$ DealDamage | Defined$ TriggeredAttackerLKICopy | NumDmg$ 2 | DamageMap$ True | SubAbility$ DBAtkCtrl\n\
         SVar:TrigBlk:DB$ DealDamage | Defined$ TriggeredBlockerLKICopy | NumDmg$ 2 | DamageMap$ True | SubAbility$ DBBlkCtrl\n\
         SVar:DBAtkCtrl:DB$ DealDamage | Defined$ TriggeredAttackerController | NumDmg$ 2 | SubAbility$ DBRes\n\
         SVar:DBBlkCtrl:DB$ DealDamage | Defined$ TriggeredBlockerController | NumDmg$ 2 | SubAbility$ DBRes\n\
         SVar:DBRes:DB$ DamageResolve",
    );
    assert_eq!(hound.abilities.len(), 1, "{:?}", hound.abilities);
    assert!(
        hound.abilities[0].contains("Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE)")
            && hound.abilities[0].contains("TargetSpec::EventObject")
            && hound.abilities[0].contains("PlayerRel::ControllerOfEvent"),
        "{}",
        hound.abilities[0]
    );
}

#[test]
fn wave4_copy_reveal_dig_until_and_hand_graveyard_zones() {
    let copy_spell = read(
        "Name:TwincastFixture\nManaCost:U U\nTypes:Instant\n\
         A:SP$ CopySpellAbility | ValidTgts$ Instant,Sorcery | TargetType$ Spell | MayChooseTarget$ True",
    );
    assert!(
        copy_spell.abilities[0].contains("Effect::CopyTargetSpell { mods: &[] }")
            && copy_spell.abilities[0].contains("TargetSpec::Spell("),
        "{}",
        copy_spell.abilities[0]
    );

    let copy_event = read(
        "Name:CopyEnch\nManaCost:4 R R\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
    );
    assert!(
        copy_event.abilities[0].contains("Effect::CopyEventSpellOrAbility"),
        "{}",
        copy_event.abilities[0]
    );

    let spy = read(
        "Name:MillSpy\nManaCost:3 B\nTypes:Creature\nPT:2/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | ValidTgts$ Player | IsCurse$ True | Amount$ 1 | Valid$ Land | ValidDescription$ land | RevealedDestination$ Graveyard",
    );
    assert!(
        spy.abilities[0].contains("Effect::RevealUntilMatching { player: PlayerRel::Chosen")
            && spy.abilities[0]
                .contains("matched: SearchDest::Graveyard, otherwise: SearchDest::Graveyard"),
        "{}",
        spy.abilities[0]
    );

    let grave_pick = read(
        "Name:BonesReturn\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Hidden$ True | Mandatory$ True | ChangeType$ Creature.YouOwn",
    );
    assert!(
        grave_pick.abilities[0].contains("Effect::ChooseFromYourGraveyard")
            && grave_pick.abilities[0]
                .contains("destination: SearchDest::Battlefield, tapped: false, optional: false"),
        "{}",
        grave_pick.abilities[0]
    );

    let hand_top = read(
        "Name:TopPut\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ 0 | Mandatory$ True | ChangeNum$ 2",
    );
    assert!(
        hand_top.abilities[0].contains("Effect::PutFromHandOnTop { count: 2 }"),
        "{}",
        hand_top.abilities[0]
    );

    let reveal = read(
        "Name:PeekHand\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent\n\
         Oracle:Target opponent reveals their hand.",
    );
    assert!(
        reveal.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }"),
        "{}",
        reveal.abilities[0]
    );
}

#[test]
fn wave4_svar_conditions_damage_pump_and_second_destroy() {
    // Active-player upkeep hand check (`TriggeredPlayer$CardsInHand` + `EQ0`) and `Count$Raid`.
    let asylum = read(
        "Name:EmptyHandWatcher\nManaCost:1 B\nTypes:Creature\nPT:3/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | CheckSVar$ H | SVarCompare$ EQ0 | Execute$ TrigDraw\n\
         SVar:H:TriggeredPlayer$CardsInHand\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        asylum.abilities[0].contains("Condition::Not(&CHECK1)")
            && asylum
                .statics
                .contains("Amount::HandSize(PlayerRel::ActivePlayer)"),
        "{}\n{}",
        asylum.statics,
        asylum.abilities[0]
    );

    let raid = read(
        "Name:RaidScout\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ R | SVarCompare$ GE1 | Execute$ TrigDraw\n\
         SVar:R:Count$Raid.1.0\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        raid.abilities[0].contains("Condition::AttackedWithAtLeast(1)"),
        "{}",
        raid.abilities[0]
    );

    // `ParentTargeted$CardPower` with `Defined$ TargetedController`, `Pump` hexproof & `Wither`.
    let burn_ctrl = read(
        "Name:SearingHit\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ DBBurn\n\
         SVar:DBBurn:DB$ DealDamage | Defined$ TargetedController | NumDmg$ X\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        burn_ctrl.abilities[0].contains(
            "Effect::DealDamage { amount: Amount::TargetPower, target: TargetSpec::Player(PlayerRel::ControllerOfTarget) }"
        ),
        "{}",
        burn_ctrl.abilities[0]
    );

    let calm = read(
        "Name:CalmShield\nManaCost:W\nTypes:Instant\n\
         A:SP$ Pump | Defined$ You | KW$ Hexproof | Duration$ UntilYourNextTurn",
    );
    assert!(
        calm.abilities[0].contains(
            "Effect::continuous(&Filter::Any, Modifier::PlayerHexproof { who: PlayerRel::You }, Duration::UntilYourNextTurn)"
        ),
        "{}",
        calm.abilities[0]
    );

    let deathtouch_pump = read(
        "Name:BlightBlade\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Deathtouch | Duration$ UntilEndOfCombat",
    );
    assert!(
        deathtouch_pump.abilities[0].contains("KeywordSet::DEATHTOUCH")
            && deathtouch_pump.abilities[0].contains("Duration::UntilEndOfCombat"),
        "{}",
        deathtouch_pump.abilities[0]
    );
    for (kw, expected) in [
        ("Wither", "KeywordSet::WITHER"),
        ("Horsemanship", "KeywordSet::HORSEMANSHIP"),
        ("Skulk", "KeywordSet::SKULK"),
        ("Enlist", "KeywordSet::ENLIST"),
        ("Read ahead", "KeywordSet::READ_AHEAD"),
    ] {
        let pumped = read(&format!(
            "Name:GateTest\nManaCost:1 B\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | KW$ {kw} | Duration$ UntilEndOfCombat"
        ));
        assert!(
            pumped.abilities[0].contains(expected)
                && pumped.abilities[0].contains("Duration::UntilEndOfCombat"),
            "{}",
            pumped.abilities[0]
        );
    }

    // Second-target `Destroy` emits `DestroyTargetSlots { targets: &[TargetSlot::Second], .. }`.
    let twin_kill = read(
        "Name:DoubleBreak\nManaCost:3 W B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Artifact | SubAbility$ DBKill2\n\
         SVar:DBKill2:DB$ Destroy | ValidTgts$ Enchantment | NoRegen$ True",
    );
    assert!(
        twin_kill.abilities[0].contains(
            "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: true }"
        ),
        "{}",
        twin_kill.abilities[0]
    );
}

#[test]
fn wave5_blocked_attacker_and_graveyard_may_play() {
    let blocked_pack = read(
        "Name:PackAlpha\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackerBlocked | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttacker | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Valid Creature.blockingTriggeredAttacker",
    );
    assert!(
        blocked_pack.statics.contains("Filter::BlockingReference")
            && !blocked_pack.statics.contains("Filter::BlockingSource"),
        "{}",
        blocked_pack.statics
    );

    let crucible = read(
        "Name:LoamVessel\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn | AffectedZone$ Graveyard | MayPlay$ True",
    );
    assert_eq!(
        crucible.abilities,
        ["static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard)"]
    );
    let seasonal_loam = read(
        "Name:TurnVessel\nManaCost:2 G\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn | AffectedZone$ Graveyard | Condition$ PlayerTurn | MayPlay$ True",
    );
    assert_eq!(
        seasonal_loam.abilities,
        [
            "static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard, condition = Some(Condition::YourTurn))"
        ]
    );
    assert!(refused(
        "Name:BadLoam\nManaCost:3\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn | AffectedZone$ Graveyard | MayPlay$ True | MayPlayLimit$ 1"
    ));
    let all_grave = read(
        "Name:GraveHall\nManaCost:3 B B\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl | AffectedZone$ Graveyard | MayPlay$ True",
    );
    assert_eq!(
        all_grave.abilities,
        [
            "static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard)",
            "static_ability!(Filter::Any, Modifier::CastSpellsFromGraveyard)",
        ]
    );
    assert!(refused(
        "Name:SubtypeGrave\nManaCost:2 B\nTypes:Creature\nPT:3/3\n\
         S:Mode$ Continuous | Affected$ Knight.YouCtrl | AffectedZone$ Graveyard | MayPlay$ True"
    ));

    let kess = read(
        "Name:ArcaneEcho\nManaCost:1 U B R\nTypes:Creature\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | Condition$ PlayerTurn | ReplaceGraveyard$ Exile | MayPlay$ True | MayPlayLimit$ 1 | EffectZone$ Battlefield | AffectedZone$ Graveyard",
    );
    assert_eq!(
        kess.abilities,
        ["static_ability!(Filter::Any, Modifier::GraveyardInstantSorceryOnce)"]
    );
    assert!(refused(
        "Name:BadKess\nManaCost:1 U B R\nTypes:Creature\nPT:3/4\n\
         S:Mode$ Continuous | Affected$ Instant.YouCtrl,Sorcery.YouCtrl | Condition$ PlayerTurn | MayPlay$ True | MayPlayLimit$ 1 | AffectedZone$ Graveyard"
    ));
}

#[test]
fn wave5_reduce_cost_and_spell_prevention() {
    let smith = read(
        "Name:SteelSmith\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 1",
    );
    assert_eq!(
        smith.abilities,
        ["static_ability!(Filter::ControlledByYou, Modifier::EquipCostsLess(1))"]
    );
    assert!(refused(
        "Name:BadSmith\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 1 | MinMana$ 1"
    ));
    assert!(refused(
        "Name:TargetSmith\nManaCost:R\nTypes:Creature\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 3"
    ));

    let grounds = read(
        "Name:TrainingRing\nManaCost:U\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.YouCtrl | Type$ Ability | Amount$ 2 | MinMana$ 1 | AffectedZone$ Battlefield",
    );
    assert_eq!(
        grounds.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::AbilitiesCostLess { amount: 2, minimum: 1 })"
        ]
    );
    assert!(refused(
        "Name:ZirdaLike\nManaCost:1 R W\nTypes:Creature\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Activator$ You | ValidSpell$ Activated.!ManaAbility | Amount$ 2 | MinMana$ 1"
    ));
    assert!(refused(
        "Name:AnyZoneAbil\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Activator$ You | Type$ Ability | Amount$ 1"
    ));

    let gy_reduce = read(
        "Name:GraveDiscount\nManaCost:3 U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ 2 | CheckSVar$ GY | SVarCompare$ GE3\n\
         SVar:GY:Count$ValidGraveyard Instant,Sorcery\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert!(
        gy_reduce.abilities[0].contains("Condition::AmountAtLeast(Amount::CountOf")
            && gy_reduce.abilities[0].contains("ZoneSel::GraveyardAll")
            && gy_reduce.abilities[0].contains(", 3)"),
        "{}",
        gy_reduce.abilities[0]
    );
    let opp_gy_reduce = read(
        "Name:StoryDiscount\nManaCost:4 U U U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ 3 | CheckSVar$ X | SVarCompare$ GE7\n\
         SVar:X:PlayerCountOpponents$HighestCardsInGraveyard\n\
         A:SP$ Draw | NumCards$ 4",
    );
    assert_eq!(
        opp_gy_reduce.abilities[0],
        "static_ability!(Filter::This, Modifier::SpellsCostLess(3), zone = ZoneRef::Stack, condition = Some(Condition::OpponentGraveyardCountAtLeast(7)))"
    );

    let spell_ward = read(
        "Name:SpellVeil\nManaCost:1 R W\nTypes:Creature\nPT:2/3\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Spell | ValidTarget$ You,Permanent.YouCtrl | Prevent$ True",
    );
    assert!(
        spell_ward
            .statics
            .contains("Filter::InZone(ZoneRef::Stack)")
            && spell_ward.abilities.len() == 2
            && spell_ward.abilities[0].contains("Modifier::PreventDamageToYouFrom(&SOURCE1)")
            && spell_ward.abilities[1].contains("Modifier::PreventDamageFrom(&SOURCE1)"),
        "{}\n{:?}",
        spell_ward.statics,
        spell_ward.abilities
    );
    assert!(refused(
        "Name:BadComb\nManaCost:1 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You,Permanent.YouCtrl | IsCombat$ True | Prevent$ True"
    ));
    assert!(refused(
        "Name:BadOpp\nManaCost:1 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You,Opponent | Prevent$ True"
    ));
}

#[test]
fn wave6_copy_permanent_become_copy_and_copy_spell_ability() {
    let token_copy = read(
        "Name:MirrorSpark\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ CopyPermanent | ValidTgts$ Creature.YouCtrl | NumCopies$ 2 | NonLegendary$ True | SetPower$ 3 | SetToughness$ 3 | SetColor$ Red | SetCreatureTypes$ Goblin | AddKeywords$ Haste | TokenTapped$ True | TokenAttacking$ True | PumpKeywords$ Haste | AtEOT$ Exile",
    );
    let a = &token_copy.abilities[0];
    assert!(a.contains("Effect::CreateTokenCopy"), "{a}");
    assert!(a.contains("amount: Amount::Fixed(2)"), "{a}");
    assert!(
        a.contains("CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY)"),
        "{a}"
    );
    assert!(a.contains("CopyMod::SetPT(3, 3)"), "{a}");
    assert!(
        a.contains("CopyMod::SetColor(ColorSet::from_slice(&[Color::Red]))"),
        "{a}"
    );
    assert!(
        a.contains("CopyMod::SetCreatureTypes(&[subtypes::creature::GOBLIN])"),
        "{a}"
    );
    assert!(
        a.contains("tapped: true, attacking: true, haste_until_eot: true"),
        "{a}"
    );
    assert!(
        a.contains("cleanup: CopyCleanup::ExileAtNextEndStep"),
        "{a}"
    );

    let pop =
        read("Name:SeedSong\nManaCost:W\nTypes:Sorcery\nA:SP$ CopyPermanent | Populate$ True");
    assert!(pop.abilities[0].contains("Effect::ChooseAndCreateTokenCopy"));

    let kicked_copy = read(
        "Name:ReplicateWave\nManaCost:2 U U\nTypes:Sorcery\nK:Kicker:5\n\
         A:SP$ CopyPermanent | ValidTgts$ Creature | NumCopies$ X\nSVar:X:Count$Kicked.5.1",
    );
    assert!(
        kicked_copy.abilities[0].contains("Effect::IfKicked")
            && kicked_copy.abilities[0].contains("Amount::Fixed(5)"),
        "{}",
        kicked_copy.abilities[0]
    );

    let clone_gy = read(
        "Name:ShiftLooter\nManaCost:U B\nTypes:Legendary Creature Shapeshifter\nPT:1/3\n\
         A:AB$ Clone | Cost$ 2 | ValidTgts$ Creature.YouOwn | TgtZone$ Graveyard | NewName$ ShiftLooter | AddTypes$ Legendary | GainThisAbility$ True | Duration$ UntilEndOfTurn",
    );
    let a = &clone_gy.abilities[0];
    assert!(a.contains("Effect::BecomeCopy"), "{a}");
    assert!(a.contains("CopyMod::SetName(\"ShiftLooter\")"), "{a}");
    assert!(a.contains("CopyMod::KeepResolvingAbility"), "{a}");
    assert!(a.contains("until_eot: true"), "{a}");

    let dual_clone = read(
        "Name:TwinMask\nManaCost:1 G U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 0 | SubAbility$ Make\n\
         SVar:Make:DB$ Clone | CloneTarget$ ParentTarget | ValidTgts$ Creature | TargetUnique$ True | Duration$ UntilEndOfTurn",
    );
    assert!(dual_clone.abilities[0].contains("original_is_second_target: true"));

    let mirror_all = read(
        "Name:WeaveAll\nManaCost:2 U U\nTypes:Instant\n\
         A:SP$ Clone | ValidTgts$ Creature.nonLegendary | CloneTarget$ Valid Creature | Duration$ UntilEndOfTurn",
    );
    assert!(mirror_all.abilities[0].contains("subject_filter: Some(&Filter::CREATURE)"));

    let copy_spell = read(
        "Name:DualFork\nManaCost:U U\nTypes:Instant\n\
         A:SP$ CopySpellAbility | ValidTgts$ Card | TargetType$ Spell,Activated,Triggered | NumCopies$ 2 | NonLegendary$ True",
    );
    let a = &copy_spell.abilities[0];
    assert!(a.contains("Effect::CopySpellOrAbility"), "{a}");
    assert!(a.contains("may_choose_new_targets: false"), "{a}");
    assert!(
        a.contains("CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY)"),
        "{a}"
    );

    assert!(refused(
        "Name:BadPow\nTypes:Sorcery\nA:SP$ CopyPermanent | ValidTgts$ Creature | SetPower$ 2"
    ));
}

#[test]
fn wave6_optional_copy_spell_uses_may_do_for_non_controller() {
    let chain_obj = read(
        "Name:ChainShatter\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Artifact | SubAbility$ Copy\n\
         SVar:Copy:DB$ CopySpellAbility | Defined$ Parent | Controller$ TargetedController | Optional$ True | MayChooseTarget$ True",
    );
    assert!(
        chain_obj.abilities[0].contains(
            "Effect::MayDoFor { player: PlayerRel::ControllerOfTarget, effects: &[Effect::CopySpellOrAbility"
        ),
        "{}",
        chain_obj.abilities[0]
    );
    let chain_player = read(
        "Name:ChainDiscard\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | NumCards$ 2 | Mode$ TgtChoose | SubAbility$ Copy\n\
         SVar:Copy:DB$ CopySpellAbility | Defined$ Parent | Controller$ TargetedPlayer | Optional$ True | MayChooseTarget$ True",
    );
    assert!(
        chain_player.abilities[0].contains(
            "Effect::MayDoFor { player: PlayerRel::Chosen, effects: &[Effect::CopySpellOrAbility"
        ),
        "{}",
        chain_player.abilities[0]
    );
    assert!(refused(
        "Name:ChainMulti\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ Copy\n\
         SVar:Copy:DB$ CopySpellAbility | Defined$ Parent | Controller$ Opponent | Optional$ True | MayChooseTarget$ True"
    ));
}

#[test]
fn wave6_flip_coin_roll_dice_and_triggers() {
    let flip = read(
        "Name:CoinBreaker\nManaCost:1 R\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ FlipCoin | Cost$ R T | ValidTgts$ Artifact | WinSubAbility$ Win | LoseSubAbility$ Lose\n\
         SVar:Win:DB$ Destroy | Defined$ Targeted\n\
         SVar:Lose:DB$ Sacrifice | Defined$ Self",
    );
    let a = &flip.abilities[0];
    assert!(
        a.contains("Effect::FlipCoin { won: &[Effect::destroy("),
        "{a}"
    );
    assert!(a.contains("lost: &[Effect::SacrificeSelf]"), "{a}");

    let until_lose = read(
        "Name:ChaosEye\nManaCost:4 R\nTypes:Creature Goblin\nPT:3/3\n\
         T:Mode$ FlippedCoin | ValidPlayer$ You | ValidResult$ Win | TriggerZones$ Battlefield | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         A:AB$ FlipCoin | Cost$ T | FlipUntilYouLose$ True | RememberNumber$ Wins | SubAbility$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ Wins",
    );
    assert!(
        until_lose.abilities[0]
            .contains("Trigger::CoinFlipped { who: PlayerRel::You, outcome: Some(true) }"),
        "{}",
        until_lose.abilities[0]
    );
    assert!(
        until_lose.abilities[1].contains("Effect::FlipUntilLose")
            && until_lose.abilities[1].contains("Amount::CoinsWonThisResolution"),
        "{}",
        until_lose.abilities[1]
    );

    let d20 = read(
        "Name:DungeonRogue\nManaCost:2 B\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ RollDice | Cost$ 2 | Sides$ 20 | ResultSubAbilities$ 1-9:Low,10-19:Mid,20:Crit\n\
         SVar:Low:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:Mid:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:Crit:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    let a = &d20.abilities[0];
    assert!(
        a.contains("Effect::RollDice { count: Amount::Fixed(1), sides: 20, modifier: 0 }"),
        "{a}"
    );
    assert!(
        a.contains("Effect::IfDieRollInRange { min: 1, max: 9"),
        "{a}"
    );
    assert!(a.contains("min: 20, max: 20"), "{a}");

    let roll_val = read(
        "Name:DiceSquirrel\nManaCost:1 G\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ RollDice | Cost$ T | ResultSVar$ Result | SubAbility$ Pump\n\
         SVar:Pump:DB$ Pump | Defined$ Self | NumAtt$ +X\n\
         SVar:X:SVar$Result/Minus.1\n\
         T:Mode$ RolledDie | ValidPlayer$ You | ValidResult$ GE4 | TriggerZones$ Battlefield | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ R\n\
         SVar:R:TriggerCount$Result",
    );
    assert!(
        roll_val.abilities[0]
            .contains("Amount::SaturatingSub { base: &Amount::LastDieRoll, subtract: 1 }"),
        "{}",
        roll_val.abilities[0]
    );
    assert!(
        roll_val.abilities[1]
            .contains("Trigger::DieRolled { who: PlayerRel::You, min_result: Some(4) }")
            && roll_val.abilities[1].contains("Amount::EventAmount"),
        "{}",
        roll_val.abilities[1]
    );

    assert!(refused(
        "Name:NoCallFlip\nTypes:Instant\nA:SP$ FlipCoin | NoCall$ True | HeadsSubAbility$ D\nSVar:D:DB$ Draw | NumCards$ 1"
    ));
    assert!(refused(
        "Name:ExactRoll\nTypes:Creature\nPT:1/1\nT:Mode$ RolledDie | ValidPlayer$ You | ValidResult$ EQ6 | TriggerZones$ Battlefield | Execute$ D\nSVar:D:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave6_phasing_dual_target_tap_slots_and_event_toughness() {
    let familiar = read(
        "Name:PhaseImp\nManaCost:1 U\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ PhaseOut | ValidCard$ Card.phasedOutSelf | Execute$ Mill\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | TriggerZones$ Battlefield | Secondary$ True | Execute$ Mill\n\
         SVar:Mill:DB$ Mill | NumCards$ 3\n\
         T:Mode$ PhaseIn | ValidCard$ Card.Self | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         A:AB$ Effect | Cost$ U | RememberObjects$ Self | ForgetOnMoved$ Battlefield | StaticAbilities$ Lock | Duration$ UntilYourNextUpkeep\n\
         SVar:Lock:Mode$ CantPhaseOut | ValidCard$ Card.IsRemembered",
    );
    assert!(
        familiar.abilities[0].contains("Trigger::PhasesOutOrLeavesBattlefield(&Filter::This)"),
        "{:?}",
        familiar.abilities
    );
    assert!(familiar.abilities[1].contains("Trigger::PHASES_IN"));
    assert!(
        familiar.abilities[2].contains("Modifier::CantPhaseOut")
            && familiar.abilities[2].contains("Duration::UntilYourNextUpkeep"),
        "{}",
        familiar.abilities[2]
    );

    let oubliette = read(
        "Name:PhasePrison\nManaCost:1 B B\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Out\n\
         SVar:Out:DB$ Phases | ValidTgts$ Creature | WontPhaseInNormal$ True | ConditionPresent$ Card.Self | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | Triggers$ Back | RememberObjects$ Targeted | ImprintCards$ Self | ConditionPresent$ Card.Self | Duration$ Permanent | ForgetOnPhasedIn$ True\n\
         SVar:Back:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.IsImprinted | Execute$ In | TriggerZones$ Command | TriggerController$ TriggeredCardController | Static$ True\n\
         SVar:In:DB$ Phases | Defined$ Remembered | Tapped$ True | PhaseInOrOut$ True | SubAbility$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Command | Destination$ Exile | Defined$ Self",
    );
    assert!(
        oubliette.abilities[0]
            .contains("Effect::PhaseOutUntilSourceLeaves { tap_on_phase_in: true }"),
        "{}",
        oubliette.abilities[0]
    );

    let conceal = read(
        "Name:HideAll\nManaCost:2 W W\nTypes:Instant\n\
         A:SP$ Phases | ValidTgts$ Permanent.nonLand+YouCtrl | TargetMin$ 0 | TargetMax$ MaxTgts\n\
         SVar:MaxTgts:Count$Valid Permanent.nonLand+YouCtrl",
    );
    assert!(
        conceal.abilities[0].contains("Effect::PhaseOut { target: Some(")
            && conceal.abilities[0].contains("TargetReq::up_to(")
            && conceal.abilities[0].contains(", 255)"),
        "{}",
        conceal.abilities[0]
    );

    let dual_tap = read(
        "Name:DualTap\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ UntapSecond\n\
         SVar:UntapSecond:DB$ Untap | ValidTgts$ Creature | TargetUnique$ True",
    );
    let a = &dual_tap.abilities[0];
    assert!(
        a.contains("Effect::TapTargetSlot(TargetSlot::First)"),
        "{a}"
    );
    assert!(
        a.contains("Effect::UntapTargetSlot(TargetSlot::Second)"),
        "{a}"
    );

    let deadshot_like = read(
        "Name:TapBite\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ Hit\n\
         SVar:Hit:DB$ DealDamage | ValidTgts$ Creature | DamageSource$ ParentTarget | NumDmg$ X | TargetUnique$ True\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        deadshot_like.abilities[0].contains("Effect::TapTargetSlot(TargetSlot::First)"),
        "{}",
        deadshot_like.abilities[0]
    );

    let chorus = read(
        "Name:ToughChorus\nManaCost:3 W W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:TriggeredCardLKICopy$Toughness",
    );
    assert!(
        chorus.abilities[0].contains("Amount::EventLastToughness"),
        "{}",
        chorus.abilities[0]
    );
}

#[test]
fn wave7_amass_and_prepare_rules() {
    let mut tok_cats = cats();
    for sub in ["Orc", "Zombie", "Army"] {
        tok_cats.creature.push(sub.to_string());
    }
    tok_cats.normalize();
    let army_tokens = TokenLookup::held(&[
        (
            "b_0_0_orc_army",
            "Name:Orc Army Token\nColors:black\nTypes:Creature Orc Army\nPT:0/0",
        ),
        (
            "b_0_0_zombie_army",
            "Name:Zombie Army Token\nColors:black\nTypes:Creature Zombie Army\nPT:0/0",
        ),
    ]);
    let amass_orc = transcode(
        &parse("Name:OrcRally\nManaCost:1 B\nTypes:Sorcery\nA:SP$ Amass | Type$ Orc | Num$ 2"),
        &tok_cats,
        Some(&army_tokens),
    )
    .expect("amass Orcs 2");
    assert!(
        amass_orc.abilities[0].contains(
            "Effect::Amass { token: &generated_tokens::ORC_ARMY_0_0_BLACK, \
             subtype: subtypes::creature::ORC, amount: Amount::Fixed(2) }"
        ),
        "{}",
        amass_orc.abilities[0]
    );
    assert!(
        transcode(
            &parse(
                "Name:RememberAmass\nManaCost:2 B\nTypes:Sorcery\n\
                 A:SP$ Amass | Type$ Zombie | Num$ 2 | RememberAmass$ True"
            ),
            &tok_cats,
            Some(&army_tokens),
        )
        .is_none()
    );
    assert!(
        transcode(
            &parse(
                "Name:OppAmass\nManaCost:1 B\nTypes:Sorcery\n\
                 A:SP$ Amass | Defined$ Opponent | Type$ Zombie | Num$ 1"
            ),
            &tok_cats,
            Some(&army_tokens),
        )
        .is_none()
    );

    let prep_self = read(
        "Name:ReadyMage\nTypes:Creature\nPT:2/2\n\
         A:AB$ AlterAttribute | Cost$ 1 T | Attributes$ Prepared",
    );
    assert!(prep_self.abilities[0].contains("Effect::BecomePrepared"));
    let prep_tgt = read(
        "Name:ReadyTutor\nTypes:Creature\nPT:2/2\n\
         A:AB$ AlterAttribute | Cost$ 2 T | ValidTgts$ Creature | Attributes$ Prepared",
    );
    assert!(prep_tgt.abilities[0].contains("Effect::BecomeTargetPrepared"));
    let saddle_self = read(
        "Name:Steed\nTypes:Creature\nPT:3/3\n\
         A:AB$ AlterAttribute | Cost$ 2 | Defined$ Self | Attributes$ Saddled",
    );
    assert!(saddle_self.abilities[0].contains("Effect::BecomeSaddled"));
    assert!(refused(
        "Name:Unprepare\nTypes:Creature\nPT:2/2\n\
         A:AB$ AlterAttribute | Cost$ 1 | Attributes$ Prepared | Activate$ False"
    ));
    assert!(refused(
        "Name:Suspector\nTypes:Creature\nPT:2/2\n\
         A:AB$ AlterAttribute | Cost$ 1 | ValidTgts$ Creature | Attributes$ Suspected"
    ));
}

#[test]
fn wave7_monarch_and_unless_rules() {
    let crown_you = read(
        "Name:CrownBearer\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ BecomeMonarch | Defined$ You",
    );
    assert!(crown_you.abilities[0].contains("Effect::BecomeMonarch(PlayerRel::You)"));
    let crown_tgt = read(
        "Name:CrownGift\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ BecomeMonarch | ValidTgts$ Player",
    );
    assert!(crown_tgt.abilities[0].contains("Effect::BecomeMonarch(PlayerRel::Chosen)"));
    assert!(refused(
        "Name:BadCrown\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ BecomeMonarch | Defined$ Player"
    ));

    let any_colored = read(
        "Name:TaxSpell\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Draw | Defined$ You | NumCards$ 2 | UnlessCost$ 1 U | UnlessPayer$ Player",
    );
    assert!(
        any_colored.abilities[0].contains(
            "Effect::AnyPlayerMayPayManaOr { cost: mana!(\"{1}{U}\"), effects: &[Effect::draw(2)] }"
        ),
        "{}",
        any_colored.abilities[0]
    );
    let draw_loot = read(
        "Name:DrawUnless\nManaCost:U\nTypes:Instant\n\
         A:SP$ Discard | Defined$ You | Mode$ TgtChoose | NumCards$ 1 | UnlessCost$ Draw<1/You> | UnlessPayer$ You | UnlessSwitched$ True",
    );
    assert!(
        draw_loot.abilities[0].contains(
            "Effect::MayDo { effects: &[Effect::draw(1), Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }] }"
        ),
        "{}",
        draw_loot.abilities[0]
    );
}

#[test]
fn wave7_alt_cost_conditions_and_self_spell_cast_triggers() {
    for (cond_word, expected) in [
        ("Surge", "Condition::Surge"),
        ("Prowl", "Condition::Prowl"),
        ("Blitz", "Condition::BlitzCostPaid"),
        ("AltCost", "Condition::AltCostPaid"),
    ] {
        let body = read(&format!(
            "Name:AltCheck\nManaCost:2 U\nTypes:Sorcery\n\
             A:SP$ Draw | Defined$ You | NumCards$ 1 | Condition$ {cond_word}"
        ));
        assert!(
            body.abilities[0].contains(expected),
            "{cond_word} -> {}",
            body.abilities[0]
        );
    }

    let branch = read(
        "Name:AltBranch\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ Branch | BranchConditionSVar$ Alt | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ Two | FalseSubAbility$ One\n\
         SVar:Alt:Count$AltCost.1.0\n\
         SVar:Two:DB$ Draw | Defined$ You | NumCards$ 2\n\
         SVar:One:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        branch.abilities[0].contains("Condition::AltCostPaid"),
        "{}",
        branch.abilities[0]
    );

    let opp_lost = read(
        "Name:SpectacleWatcher\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ Lost | SVarCompare$ GE1 | Execute$ Draw\n\
         SVar:Lost:PlayerCountOpponents$HasPropertyLostLifeThisTurn\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        opp_lost.abilities[0].contains("Condition::OpponentLostLifeThisTurn"),
        "{}",
        opp_lost.abilities[0]
    );

    let cast_self = read(
        "Name:TitanCast\nManaCost:8\nTypes:Creature\nPT:8/8\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ Draw2\n\
         SVar:Draw2:DB$ Draw | Defined$ You | NumCards$ 2",
    );
    assert!(
        cast_self.abilities[0].contains("Trigger::SpellCast(&Filter::This)"),
        "{}",
        cast_self.abilities[0]
    );
    let heroic = read(
        "Name:HeroicAllowed\nTypes:Creature\nPT:1/1\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | TargetsValid$ Card.Self | TriggerZones$ Battlefield | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        heroic.abilities[0].contains("Trigger::HEROIC"),
        "{}",
        heroic.abilities[0]
    );
    assert!(refused(
        "Name:HeroicNoZone\nTypes:Creature\nPT:1/1\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | TargetsValid$ Card.Self | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1"
    ));
}

#[test]
fn wave7_mill_take_one_and_remembered_sacrifice_rules() {
    let mill_take = read(
        "Name:MilledFind\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ You | NumCards$ 4 | RememberMilled$ True | SubAbility$ Take\n\
         SVar:Take:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | Hidden$ True | Optional$ True | ChangeType$ Permanent.IsRemembered | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mill_take.abilities[0].contains("Effect::MillMayTakeOne { amount: Amount::Fixed(4)"),
        "{}",
        mill_take.abilities[0]
    );
    assert_eq!(
        refusal_reason(
            &parse(
                "Name:MandatoryMillTake\nManaCost:1 G\nTypes:Sorcery\n\
                 A:SP$ Mill | Defined$ You | NumCards$ 4 | RememberMilled$ True | SubAbility$ Take\n\
                 SVar:Take:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | Hidden$ True | Mandatory$ True | ChangeType$ Creature.IsRemembered"
            ),
            &cats(),
            None,
        )
        .as_deref(),
        Some("unclaimed parameter `Mill.RememberMilled`")
    );

    let sac_then = read(
        "Name:SacForCards\nTypes:Artifact\n\
         A:AB$ Sacrifice | Cost$ T | Defined$ You | SacValid$ Creature | Optional$ True | RememberSacrificed$ True | SubAbility$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GE1 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        sac_then.abilities[0].contains(
            "Effect::PlayerMayPayCostThen { player: PlayerRel::You, cost: &CostPart::Sacrifice("
        ),
        "{}",
        sac_then.abilities[0]
    );

    let sac_or = read(
        "Name:SacOrHurt\nTypes:Creature\nPT:6/6\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Sac\n\
         SVar:Sac:DB$ Sacrifice | Defined$ You | SacValid$ Creature.Other | Optional$ True | RememberSacrificed$ True | SubAbility$ Hurt\n\
         SVar:Hurt:DB$ DealDamage | Defined$ You | NumDmg$ 5 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        sac_or.abilities[0].contains(
            "Effect::PlayerMayPayCostOr { player: PlayerRel::You, cost: &CostPart::Sacrifice("
        ),
        "{}",
        sac_or.abilities[0]
    );

    let can_sac = read(
        "Name:PitCheck\nTypes:Creature\nPT:7/7\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Sac\n\
         SVar:Sac:DB$ Sacrifice | Defined$ You | SacValid$ Creature.Other | RememberSacrificed$ True | SubAbility$ Hurt\n\
         SVar:Hurt:DB$ DealDamage | Defined$ You | NumDmg$ 7 | ConditionCheckSVar$ SacCnt | ConditionSVarCompare$ LT1 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacCnt:Remembered$Amount",
    );
    assert!(
        can_sac.abilities[0].contains("Condition::CanSacrifice("),
        "{}",
        can_sac.abilities[0]
    );

    let sac_and_if = read(
        "Name:SacAndCheck\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ Bonus\n\
         SVar:Bonus:DB$ GainLife | Defined$ You | LifeAmount$ 3 | ConditionDefined$ Remembered | ConditionPresent$ Artifact | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        sac_and_if.abilities[0].contains("Effect::SacrificeAndIf {"),
        "{}",
        sac_and_if.abilities[0]
    );

    let sac_opt_only = read(
        "Name:MaySacSelf\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Sac\n\
         SVar:Sac:DB$ Sacrifice | Optional$ True",
    );
    assert!(
        sac_opt_only.abilities[0].contains("Effect::MayDo { effects: &[Effect::SacrificeSelf] }"),
        "{}",
        sac_opt_only.abilities[0]
    );
}

#[test]
fn wave7_change_zone_dig_tap_untap_and_gain_control_rules() {
    let hand_mand = read(
        "Name:MustDrop\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.YouOwn | Mandatory$ True",
    );
    assert!(
        hand_mand.abilities[0].contains(
            "Effect::PutFromHandOntoBattlefield { tapped: false, filter: &Filter::CREATURE, mana_value: None, optional: false }"
        ),
        "{}",
        hand_mand.abilities[0]
    );

    let rand_gy = read(
        "Name:RandomRegrow\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Hidden$ True | AtRandom$ True | ChangeType$ Creature.YouOwn",
    );
    assert!(
        rand_gy.abilities[0].contains("Effect::RandomGraveyardToHand {"),
        "{}",
        rand_gy.abilities[0]
    );

    let blink_targets = read(
        "Name:TargetFlicker\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Exile | RememberTargets$ True | SubAbility$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ Remembered | Origin$ All | Destination$ Battlefield | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        blink_targets.abilities[0].contains("Effect::Blink { target:"),
        "{}",
        blink_targets.abilities[0]
    );

    let linked_hand = read(
        "Name:ReturnExiled\nTypes:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ 2 Sac<1/CARDNAME> | Origin$ Exile | Destination$ Hand | ChangeType$ Card.ExiledWithSource | Hidden$ True",
    );
    assert!(
        linked_hand.abilities[0].contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }"),
        "{}",
        linked_hand.abilities[0]
    );

    let dig_shuffle = read(
        "Name:DigShuffle\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Creature | FoundDestination$ Battlefield | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | Shuffle$ True",
    );
    assert!(
        dig_shuffle.abilities[0].contains("Effect::RevealUntilMatching {")
            && dig_shuffle.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{}",
        dig_shuffle.abilities[0]
    );

    let tap_self_and_untap_tgt = read(
        "Name:TapUntapSub\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 1 | SubAbility$ Unt\n\
         SVar:Unt:DB$ Untap | Defined$ Targeted",
    );
    assert!(
        tap_self_and_untap_tgt.abilities[0].contains("Effect::UntapTarget"),
        "{}",
        tap_self_and_untap_tgt.abilities[0]
    );
    let loot_one = read(
        "Name:LootOne\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ Loot\n\
         SVar:Loot:AB$ Discard | Defined$ You | Mode$ TgtChoose | Cost$ Draw<1/You>",
    );
    assert!(
        loot_one.abilities[0].contains(
            "Effect::MayDo { effects: &[Effect::draw(1), Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }] }"
        ),
        "{}",
        loot_one.abilities[0]
    );

    let gain_all = read(
        "Name:StealAll\nManaCost:5 U U\nTypes:Sorcery\n\
         A:SP$ GainControl | AllValid$ Artifact | NewController$ You",
    );
    assert!(
        gain_all.abilities[0].contains("Effect::GainControlAll {"),
        "{}",
        gain_all.abilities[0]
    );
    let gain_tapped = read(
        "Name:SirenSteal\nTypes:Creature\nPT:1/1\n\
         A:AB$ GainControl | Cost$ T | ValidTgts$ Creature | LoseControl$ Untap,LeavesPlay,LoseControl",
    );
    assert!(
        gain_tapped.abilities[0]
            .contains("Duration::WhileSourceTapped { control: true, power_limited: false }"),
        "{}",
        gain_tapped.abilities[0]
    );
    let populate =
        read("Name:PopSpell\nManaCost:2 G\nTypes:Instant\nA:SP$ CopyPermanent | Populate$ True");
    assert!(
        populate.statics.contains("Filter::IsToken") && !populate.statics.contains("Filter::TOKEN"),
        "{}",
        populate.statics
    );
}

#[test]
fn wave8b_command_zone_triggers_and_commander_conditions() {
    let eminence = read(
        "Name:EminenceCmdr\nTypes:Legendary Creature\nPT:3/3\n\
         T:Mode$ AttackersDeclared | ValidAttackers$ Creature.YouCtrl | TriggerZones$ Command | \
         PresentZone$ Command | IsPresent$ Card.StrictlySelf | Execute$ Draw1\n\
         SVar:Draw1:DB$ Draw | NumCards$ 1",
    );
    assert!(
        eminence.abilities[0].contains("zone = TriggerZone::Command")
            && eminence.abilities[0].contains("Condition::SourceInZone(ZoneRef::Command)"),
        "{}",
        eminence.abilities[0]
    );

    let both_zones = read(
        "Name:DualZoneCmdr\nTypes:Legendary Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Command,Battlefield | Execute$ Gain1\n\
         SVar:Gain1:DB$ GainLife | LifeAmount$ 1",
    );
    assert!(
        both_zones.abilities[0].contains("zone = TriggerZone::BattlefieldOrCommand"),
        "{}",
        both_zones.abilities[0]
    );

    let lieutenant = read(
        "Name:CmdrGuard\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | \
         IsPresent$ Card.IsCommander+YouOwn+YouCtrl | Execute$ Gain2\n\
         SVar:Gain2:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        lieutenant.abilities[0].contains("Condition::ControlYourCommander"),
        "{}",
        lieutenant.abilities[0]
    );

    let cond_spell = read(
        "Name:CmdrCheckSpell\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ GainLife | LifeAmount$ 3 | ConditionPresent$ Card.IsCommander+YouOwn+YouCtrl",
    );
    assert!(
        cond_spell.abilities[0].contains("Condition::ControlYourCommander"),
        "{}",
        cond_spell.abilities[0]
    );

    assert!(refused(
        "Name:BadZone\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Hand | Execute$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1"
    ));
    assert!(refused(
        "Name:SelfCmdEtb\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | TriggerZones$ Command | Execute$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1"
    ));
    assert!(refused(
        "Name:BadPresentZone\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Command | \
         PresentZone$ Library | IsPresent$ Card.StrictlySelf | Execute$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1"
    ));
}

#[test]
fn wave8b_goad_and_goad_all() {
    let single = read(
        "Name:GoadOne\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Goad | ValidTgts$ Creature",
    );
    assert!(
        single.abilities[0]
            .contains("Effect::Goad { target: TargetSpec::Object(&Filter::CREATURE) }"),
        "{}",
        single.abilities[0]
    );

    let multi = read(
        "Name:GoadTwo\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Goad | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2",
    );
    assert!(
        multi.abilities[0].contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)")
            && multi.abilities[0].contains("Effect::Goad {"),
        "{}",
        multi.abilities[0]
    );

    let goad_opp = read(
        "Name:GoadAllOpp\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Goad | Defined$ Valid Creature.OppCtrl",
    );
    assert!(
        goad_opp.abilities[0].contains("Effect::GoadAll {")
            && goad_opp.abilities[0].contains("controlled_by: None"),
        "{}",
        goad_opp.abilities[0]
    );

    let goad_player = read(
        "Name:GoadSeat\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Goad | ValidTgts$ Player | Defined$ Valid Creature.ControlledBy TargetedPlayer",
    );
    assert!(
        goad_player.abilities[0].contains("Effect::GoadAll {")
            && goad_player.abilities[0].contains("controlled_by: Some(PlayerRel::Chosen)"),
        "{}",
        goad_player.abilities[0]
    );

    assert!(refused(
        "Name:PermGoad\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Goad | ValidTgts$ Creature | Duration$ Permanent"
    ));
    assert!(refused(
        "Name:RemGoad\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Goad | ValidTgts$ Creature | RememberGoaded$ True"
    ));
}

#[test]
fn wave8b_chosen_color_protection_and_sweeps() {
    let prot_all = read(
        "Name:ColorShieldAll\nManaCost:W\nTypes:Instant\n\
         A:SP$ ChooseColor | Defined$ You | SubAbility$ Prot\n\
         SVar:Prot:DB$ ProtectionAll | ValidCards$ Creature.White+YouCtrl | Gains$ ChosenColor",
    );
    assert!(
        prot_all.abilities[0].contains("Effect::ChooseColor")
            && prot_all.abilities[0].contains("Effect::GrantProtectionFromChosenColorUntilEot {"),
        "{}",
        prot_all.abilities[0]
    );

    let standalone_choice = read(
        "Name:GraveShield\nTypes:Creature\nPT:3/3\n\
         A:AB$ ProtectionAll | Cost$ 2 W | ActivationZone$ Graveyard | ValidCards$ Creature.YouCtrl | Gains$ Choice | Choices$ AnyColor",
    );
    assert!(
        standalone_choice.abilities[0].contains("Effect::ChooseColor")
            && standalone_choice.abilities[0]
                .contains("Effect::GrantProtectionFromChosenColorUntilEot {")
            && standalone_choice.abilities[0].contains("ActivationZone::Graveyard"),
        "{}",
        standalone_choice.abilities[0]
    );

    let bounce_color = read(
        "Name:ColorBounce\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ ChooseColor | Defined$ You | SubAbility$ Sweep\n\
         SVar:Sweep:DB$ ChangeZoneAll | ChangeType$ Permanent.ChosenColor | Origin$ Battlefield | Destination$ Hand",
    );
    assert!(
        bounce_color.abilities[0].contains("Effect::ChooseColor")
            && bounce_color.abilities[0].contains("Effect::ReturnAllToHand {"),
        "{}",
        bounce_color.abilities[0]
    );

    let discard_color = read(
        "Name:ColorDiscard\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ ChooseColor | Defined$ You | SubAbility$ Disc\n\
         SVar:Disc:DB$ Discard | ValidTgts$ Player | Mode$ RevealDiscardAll | DiscardValid$ Card.ChosenColor",
    );
    assert!(
        discard_color.abilities[0].contains("Effect::ChooseColor")
            && discard_color.abilities[0].contains("Effect::RevealHand {")
            && discard_color.abilities[0].contains("Effect::DiscardMatching {"),
        "{}",
        discard_color.abilities[0]
    );

    assert!(refused(
        "Name:BadColorOr\nManaCost:W\nTypes:Instant\n\
         A:SP$ ChooseColor | Defined$ You | OrColors$ True | SubAbility$ Prot\n\
         SVar:Prot:DB$ ProtectionAll | ValidCards$ Creature.YouCtrl | Gains$ ChosenColor"
    ));
    assert!(refused(
        "Name:BadProtDur\nManaCost:W\nTypes:Instant\n\
         A:SP$ ProtectionAll | ValidCards$ Creature.YouCtrl | Gains$ Choice | Choices$ AnyColor | Duration$ Permanent"
    ));
}

#[test]
fn wave8b_repeat_each_domains_and_refusals() {
    let each_discard = read(
        "Name:EachDisc\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ Disc\n\
         SVar:Disc:DB$ Discard | Defined$ Player.IsRemembered | NumCards$ 1 | Mode$ TgtChoose",
    );
    assert!(
        each_discard.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachPlayer")
            && each_discard.abilities[0]
                .contains("Effect::DiscardForPlayers { who: PlayerRel::EventPlayer, count: 1 }"),
        "{}",
        each_discard.abilities[0]
    );

    let half_life = read(
        "Name:OppHalf\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Opponent | RepeatSubAbility$ Loss\n\
         SVar:Loss:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ Half\n\
         SVar:Half:PlayerCountRemembered$LifeTotal/HalfUp",
    );
    assert!(
        half_life.abilities[0].contains("RepeatDomain::EachOpponent")
            && half_life.abilities[0].contains(
                "Amount::Half { base: &Amount::LifeTotal(PlayerRel::EventPlayer), round_up: true }"
            ),
        "{}",
        half_life.abilities[0]
    );

    let damage_map = read(
        "Name:NonbasicBurn\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ Burn | DamageMap$ True\n\
         SVar:Burn:DB$ DealDamage | Defined$ Remembered | NumDmg$ X\n\
         SVar:X:Count$Valid Land.nonBasic+RememberedPlayerCtrl/Times.2",
    );
    assert!(
        damage_map.abilities[0].contains("Effect::DealDamage {")
            && damage_map.abilities[0].contains("Amount::Times {")
            && damage_map.abilities[0].contains("PlayerRel::Recipient"),
        "{}",
        damage_map.abilities[0]
    );

    let wheel = read(
        "Name:EachWheel\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ Shuf\n\
         SVar:Shuf:DB$ ChangeZoneAll | Defined$ Player.IsRemembered | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 | RememberChanged$ True | Shuffle$ True | SubAbility$ DrawX\n\
         SVar:DrawX:DB$ Draw | Defined$ Player.IsRemembered | NumCards$ X | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        wheel.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachPlayer")
            && wheel.abilities[0]
                .contains("Effect::ShuffleIntoLibrary { who: PlayerRel::EventPlayer")
            && wheel.abilities[0].contains("ThisWay::PutIntoLibraryFromHand"),
        "{}",
        wheel.abilities[0]
    );

    let perm_loop = read(
        "Name:CounterEach\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature.YouCtrl | Zone$ Battlefield | RepeatSubAbility$ Put\n\
         SVar:Put:DB$ PutCounter | Defined$ Remembered | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        perm_loop.abilities[0]
            .contains("RepeatDomain::EachMatchingPermanent(&Filter::YOUR_CREATURE)")
            && perm_loop.abilities[0].contains("Filter::Reference"),
        "{}",
        perm_loop.abilities[0]
    );

    assert!(refused(
        "Name:BadZoneRepeat\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | Zone$ Exile | RepeatSubAbility$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1"
    ));
    assert!(refused(
        "Name:BadTgtRepeat\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ D\n\
         SVar:D:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1"
    ));
}

#[test]
fn wave8b_choose_card_battlefield_and_graveyard() {
    let bf_then = read(
        "Name:PickPing\nTypes:Creature\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Pick\n\
         SVar:Pick:DB$ ChooseCard | Choices$ Creature.YouCtrl+Other | ChoiceZone$ Battlefield | SubAbility$ Dmg\n\
         SVar:Dmg:DB$ DealDamage | Defined$ ChosenCard | NumDmg$ 2 | SubAbility$ Draw1\n\
         SVar:Draw1:DB$ Draw | ConditionDefined$ ChosenCard | ConditionPresent$ Card | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        bf_then.abilities[0].contains("Effect::ChooseCard {")
            && bf_then.abilities[0].contains("zone: ChooseZone::Battlefield")
            && bf_then.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && bf_then.abilities[0].contains("movement: ChooseMove::None")
            && bf_then.abilities[0].contains("TargetSpec::EventObject"),
        "{}",
        bf_then.abilities[0]
    );

    let gy_reanimate = read(
        "Name:OppGyPick\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ValidTgts$ Opponent | Choices$ Creature.TargetedPlayerOwn | ChoiceZone$ Graveyard | Amount$ 1 | Mandatory$ True | SubAbility$ Move\n\
         SVar:Move:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True",
    );
    assert!(
        gy_reanimate.abilities[0].contains("chooser: PlayerRel::Chosen")
            && gy_reanimate.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::Chosen)")
            && gy_reanimate.abilities[0].contains("min: 1, max: Amount::Fixed(1)")
            && gy_reanimate.abilities[0]
                .contains("movement: ChooseMove::Battlefield { tapped: false }"),
        "{}",
        gy_reanimate.abilities[0]
    );

    assert!(refused(
        "Name:BadHandPick\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Hand | Choices$ Card.YouOwn | Mandatory$ True | SubAbility$ Move\n\
         SVar:Move:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Hand | Destination$ Graveyard"
    ));
    assert!(refused(
        "Name:BadRandPick\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Battlefield | Choices$ Creature | AtRandom$ True | SubAbility$ Kill\n\
         SVar:Kill:DB$ Destroy | Defined$ ChosenCard"
    ));
    assert!(refused(
        "Name:BadOppRean\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ValidTgts$ Opponent | Choices$ Creature.TargetedPlayerOwn | ChoiceZone$ Graveyard | Mandatory$ True | SubAbility$ Move\n\
         SVar:Move:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Graveyard | Destination$ Battlefield"
    ));
}

#[test]
fn wave8b_peek_reveal_and_change_zone_sweeps() {
    let pure_peek = read(
        "Name:PeekTop\nTypes:Artifact\n\
         A:AB$ PeekAndReveal | Cost$ 1 | NoReveal$ True",
    );
    assert!(
        pure_peek.abilities[0].contains("Effect::PeekAndReveal {")
            && pure_peek.abilities[0].contains("reveal_all: false, reveal_chosen: false")
            && pure_peek.abilities[0].contains("max_pick: 0"),
        "{}",
        pure_peek.abilities[0]
    );

    let kinship = read(
        "Name:KinshipUnit\nTypes:Creature Elf\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Kin\n\
         SVar:Kin:DB$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Card.sharesCreatureTypeWith | RevealOptional$ True | RememberRevealed$ True | SubAbility$ Gain\n\
         SVar:Gain:DB$ GainLife | LifeAmount$ 3 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ1 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        kinship.abilities[0].contains("Effect::PeekAndReveal {")
            && kinship.abilities[0].contains("reveal_chosen: true")
            && kinship.abilities[0].contains("if_picked: &[Effect::gain_life(3)]"),
        "{}",
        kinship.abilities[0]
    );

    let hand_or_bottom = read(
        "Name:TopPickOrBottom\nTypes:Creature\nPT:3/3\n\
         A:AB$ PeekAndReveal | Cost$ 3 G | PeekAmount$ 1 | RevealValid$ Creature | RevealOptional$ True | RememberRevealed$ True | SubAbility$ ToHand\n\
         SVar:ToHand:DB$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Hand | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1 | SubAbility$ ToBot\n\
         SVar:ToBot:DB$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ0 | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        hand_or_bottom.abilities[0].contains("picked: PeekPickDest::Hand")
            && hand_or_bottom.abilities[0]
                .contains("rest: PeekRestDest::BottomOfLibrary { random: false }"),
        "{}",
        hand_or_bottom.abilities[0]
    );

    let reveal_top = read(
        "Name:TopCheck\nTypes:Creature\nPT:1/2\n\
         A:AB$ PeekAndReveal | Cost$ 2 | NoPeek$ True | RememberRevealed$ True | SubAbility$ Unt\n\
         SVar:Unt:DB$ Untap | Defined$ Self | ConditionDefined$ Remembered | ConditionPresent$ Card.Land | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        reveal_top.abilities[0].contains("Effect::RevealTop {")
            && reveal_top.abilities[0].contains("if_matches: &[Effect::UntapSelf]"),
        "{}",
        reveal_top.abilities[0]
    );

    let parley = read(
        "Name:ParleyUnit\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Par\n\
         SVar:Par:DB$ PeekAndReveal | Defined$ Player | RememberRevealed$ True | SubAbility$ DrawAll\n\
         SVar:DrawAll:DB$ Draw | Defined$ Player | SubAbility$ Counter\n\
         SVar:Counter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ X | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.nonLand",
    );
    assert!(
        parley.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachPlayer")
            && parley.abilities[0].contains("Effect::RevealTop {")
            && parley.abilities[0].contains("ThisWay::Revealed("),
        "{}",
        parley.abilities[0]
    );

    let reveal_counted = read(
        "Name:HandCountLife\nManaCost:W\nTypes:Instant\n\
         A:SP$ Reveal | Defined$ You | RevealValid$ Card.White | AnyNumber$ True | RememberRevealed$ True | SubAbility$ Life\n\
         SVar:Life:DB$ GainLife | LifeAmount$ X | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount/Times.2",
    );
    assert!(
        reveal_counted.abilities[0].contains("Effect::RevealFromHand {")
            && reveal_counted.abilities[0].contains(
                "Amount::Times { base: &Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), factor: 2 }"
            ),
        "{}",
        reveal_counted.abilities[0]
    );
    assert!(refused(
        "Name:RandHandRev\nManaCost:U\nTypes:Instant\n\
         A:SP$ Reveal | Defined$ You | Random$ True"
    ));
    assert!(refused(
        "Name:MandFilteredRev\nManaCost:U\nTypes:Instant\n\
         A:SP$ Reveal | Defined$ You | RevealValid$ Card.Blue"
    ));
}

#[test]
fn wave9b_reveal_from_hand_counted_pump_target_and_self() {
    let reveal_pump_target = read(
        "Name:GreenScent\nManaCost:G\nTypes:Instant\n\
         A:SP$ Reveal | Defined$ You | RevealValid$ Card.Green | AnyNumber$ True | RememberRevealed$ True | SubAbility$ Pump\n\
         SVar:Pump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        reveal_pump_target.abilities[0].contains(
            "Effect::PumpTarget { power: Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), \
             toughness: Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        reveal_pump_target.abilities[0]
    );
    let reveal_pump_self = read(
        "Name:GreenScentBeast\nManaCost:2 G\nTypes:Creature Beast\nPT:2/2\n\
         A:AB$ Reveal | Cost$ G | Defined$ You | RevealValid$ Card.Green | AnyNumber$ True | RememberRevealed$ True | SubAbility$ Pump\n\
         SVar:Pump:DB$ Pump | Defined$ Self | NumAtt$ +X | NumDef$ +X | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        reveal_pump_self.abilities[0].contains(
            "Effect::PumpFilter { filter: &Filter::This, controlled_by: None, \
             power: Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), \
             toughness: Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        reveal_pump_self.abilities[0]
    );
}

#[test]
fn wave8b_change_zone_and_phase_check_svar_sweeps() {
    let top_tutor = read(
        "Name:TopTutor\nManaCost:G\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | LibraryPosition$ 0 | ChangeType$ Creature | ChangeNum$ 1",
    );
    assert!(
        top_tutor.abilities[0].contains("Find::TOP_OF_LIBRARY"),
        "{}",
        top_tutor.abilities[0]
    );

    let exile_enchanted = read(
        "Name:ExileHost\nManaCost:1 W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ ChangeZone | Cost$ 2 W | Defined$ Enchanted | Origin$ Battlefield | Destination$ Exile",
    );
    assert!(
        exile_enchanted
            .abilities
            .iter()
            .any(|a| a.contains("Effect::ExileAll { filter: &Filter::AttachedToBySource")),
        "{:?}",
        exile_enchanted.abilities
    );

    let phase_svar = read(
        "Name:MorbidPhase\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ X | SVarCompare$ GE1 | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature",
    );
    assert!(
        phase_svar.abilities[0].contains("Condition::CreaturesDiedThisTurnAtLeast(1)"),
        "{}",
        phase_svar.abilities[0]
    );

    assert!(refused(
        "Name:SacAuraExile\nManaCost:1 W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ ChangeZone | Cost$ 2 W Sac<1/CARDNAME> | Defined$ Enchanted | Origin$ Battlefield | Destination$ Exile"
    ));
    assert!(refused(
        "Name:BotTutor\nManaCost:G\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | ChangeType$ Creature"
    ));
}

#[test]
fn wave9b_exile_top_with_permission_windows_mana_and_variable_count() {
    let var_x = read(
        "Name:XImpulse\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | Imprint$ True | SubAbility$ Play\n\
         SVar:Play:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ UntilTheEndOfYourNextTurn | SubAbility$ Clean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreColor$ True | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        var_x.abilities[0].contains(
            "Effect::ExileTopWithPermission { who: PlayerRel::You, count: Amount::X, \
             permission: ExilePermission::cast(PlayWindow::ThroughYourNextTurn).spend_mana_as_any_color() }"
        ),
        "{}",
        var_x.abilities[0]
    );

    let while_exiled = read(
        "Name:VaultScout\nManaCost:2 R\nTypes:Creature Rogue\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Defined$ TopOfLibrary | DefinedPlayer$ Opponent | Origin$ Library | Destination$ Exile | RememberChanged$ True | ForgetOtherRemembered$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ Permanent | ForgetOnMoved$ Exile | SubAbility$ Clean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreType$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile | Description$ You may play that card for as long as it remains exiled.\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        while_exiled.abilities[0].contains(
            "Effect::ExileTopWithPermission { who: PlayerRel::Opponent, count: Amount::Fixed(1), \
             permission: ExilePermission::play(PlayWindow::WhileExiled).spend_mana_as_any_type() }"
        ),
        "{}",
        while_exiled.abilities[0]
    );

    let while_source = read(
        "Name:RelicKeeper\nManaCost:3 U\nTypes:Artifact\n\
         A:AB$ Dig | Cost$ T | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ Permanent | SubAbility$ Clean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered+Creature | AffectedZone$ Exile | Description$ You may cast that card for as long as CARDNAME remains on the battlefield.\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        while_source.abilities[0].contains(
            "ExilePermission::cast(PlayWindow::WhileSourceOnBattlefield).with_spell_types(TypeSet::CREATURE)"
        ),
        "{}",
        while_source.abilities[0]
    );

    assert!(refused(
        "Name:ControlTied\nManaCost:2 R\nTypes:Artifact\n\
         A:AB$ Dig | Cost$ T | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ Permanent\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile | Description$ You may play that card for as long as you control CARDNAME."
    ));
    assert!(refused(
        "Name:NextEndStep\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ UntilYourNextEndStep\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile"
    ));
    assert!(refused(
        "Name:FaceDownExile\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile"
    ));
    assert!(refused(
        "Name:LimitedPlay\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayLimit$ 1 | Affected$ Card.IsRemembered | AffectedZone$ Exile"
    ));
}

#[test]
fn wave9b_exile_with_permission_and_grant_linked_exile_permission() {
    let kidnap = read(
        "Name:CorsairRaider\nManaCost:2 U B\nTypes:Creature Pirate\nPT:2/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.Other | Duration$ UntilHostLeavesPlay | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | StaticAbilities$ ST | RememberObjects$ Remembered | Duration$ Permanent | ForgetOnMoved$ Exile | SubAbility$ Clean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreType$ True | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile | Description$ You may cast that card.\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        kidnap.abilities[0].contains("Effect::ExileWithPermission { target: TargetSpec::Object(&")
            && kidnap.abilities[0].contains(
                "until: Some(ExileUntil::SourceLeavesBattlefield), \
             permission: ExilePermission::cast(PlayWindow::WhileExiled).spend_mana_as_any_type() }"
            ),
        "{}",
        kidnap.abilities[0]
    );

    let gy_exile = read(
        "Name:GraveRaider\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Instant.OppOwn | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | ExileOnMoved$ Exile | SubAbility$ Clean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | Affected$ Card.nonLand+IsRemembered | AffectedZone$ Exile\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        gy_exile.abilities[0].contains(
            "Effect::ExileWithPermission { target: TargetSpec::CardInGraveyard(&"
        ) && gy_exile.abilities[0].contains(
            "until: None, permission: ExilePermission::cast(PlayWindow::ThisTurn).without_paying_mana_cost() }"
        ),
        "{}",
        gy_exile.abilities[0]
    );

    let linked_grant = read(
        "Name:VaultBeacon\nManaCost:3 B\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ 2 T | StaticAbilities$ ST | RememberObjects$ ValidExile Card.ExiledWithSource+nonLand | ForgetOnMoved$ Exile\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreColor$ True | Affected$ Card.ExiledWithEffectSource+nonLand | AffectedZone$ Exile",
    );
    assert!(
        linked_grant.abilities[0].contains(
            "Effect::GrantLinkedExilePermission { permission: ExilePermission::cast(PlayWindow::ThisTurn).spend_mana_as_any_color() }"
        ),
        "{}",
        linked_grant.abilities[0]
    );

    assert!(refused(
        "Name:BadHostLeaveSpell\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature | Duration$ UntilHostLeavesPlay | RememberChanged$ True | SubAbility$ Eff\n\
         SVar:Eff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile"
    ));
}

#[test]
fn wave9b_bargain_prototype_conditions_and_amount_splitting() {
    let etb_bargain = read(
        "Name:FaeEnvoy\nManaCost:3 B\nTypes:Creature Faerie\nPT:3/3\nK:Bargain\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+bargained | Execute$ Drain\n\
         SVar:Drain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 3",
    );
    assert!(
        etb_bargain.abilities[0]
            .contains("triggered!(Trigger::ETB, &[Effect::LoseLife { amount: Amount::Fixed(3), target: PlayerRel::Opponent }], condition = Some(Condition::Bargained))"),
        "{}",
        etb_bargain.abilities[0]
    );

    let etb_proto = read(
        "Name:ClockworkScout\nManaCost:6\nTypes:Artifact Creature Construct\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+prototyped | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        etb_proto.abilities[0].contains("condition = Some(Condition::Prototyped)"),
        "{}",
        etb_proto.abilities[0]
    );

    let split_pump = read(
        "Name:SweetBind\nManaCost:1 B\nTypes:Instant\nK:Bargain\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
         SVar:X:Count$Bargained.5.3",
    );
    assert!(
        split_pump.abilities[0].contains("Effect::IfCondition { condition: Condition::Bargained, then: &[Effect::PumpTarget { power: Amount::NegXFixed(5), toughness: Amount::NegXFixed(5)")
            && split_pump.abilities[0].contains("otherwise: &[Effect::PumpTarget { power: Amount::NegXFixed(3), toughness: Amount::NegXFixed(3)"),
        "{}",
        split_pump.abilities[0]
    );

    let svar_and_self_cond = read(
        "Name:LightStrike\nManaCost:W\nTypes:Instant\nK:Bargain\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ConditionCheckSVar$ B | ConditionSVarCompare$ EQ0 | SubAbility$ Des\n\
         SVar:Des:DB$ Destroy | Defined$ Targeted | ConditionDefined$ Self | ConditionPresent$ Card.bargained\n\
         SVar:B:Count$Bargained.1.0",
    );
    let a = &svar_and_self_cond.abilities[0];
    assert!(
        a.contains("condition: Condition::Not(&Condition::Bargained)")
            && a.contains("condition: Condition::Bargained"),
        "{a}"
    );
}

#[test]
fn wave9b_blink_dig_until_and_opponents_life_lost_sweeps() {
    let event_blink = read(
        "Name:PhaseWarden\nManaCost:2 W\nTypes:Creature Spirit\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Defined$ TriggeredAttackerLKICopy | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        event_blink.abilities[0]
            .contains("Effect::Blink { target: TargetSpec::EventObject, owner_control: true }"),
        "{}",
        event_blink.abilities[0]
    );

    let dig_shuffle = read(
        "Name:RelicReveal\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Defined$ You | Valid$ Artifact | FoundDestination$ Hand | RevealedDestination$ Library | Shuffle$ True",
    );
    assert!(
        dig_shuffle.abilities[0].contains("Effect::RevealUntilMatching")
            && dig_shuffle.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{}",
        dig_shuffle.abilities[0]
    );

    let dig_hand_all = read(
        "Name:VaultHunt\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Defined$ You | Valid$ Card.nonLand | RevealedDestination$ Hand",
    );
    assert!(
        dig_hand_all.abilities[0]
            .contains("matched: SearchDest::Hand, otherwise: SearchDest::Hand"),
        "{}",
        dig_hand_all.abilities[0]
    );

    let life_lost = read(
        "Name:BloodWatch\nManaCost:2 B\nTypes:Creature Vampire\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ L | SVarCompare$ GE4 | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:L:Count$LifeOppsLostThisTurn",
    );
    assert!(
        life_lost.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, 4)"),
        "{}",
        life_lost.abilities[0]
    );

    assert!(refused(
        "Name:SelfDelayedBlink\nManaCost:2 W\nTypes:Creature Spirit\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ 1 W | Defined$ Self | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ Del\n\
         SVar:Del:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | RememberObjects$ RememberedLKI | Execute$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Exile | Destination$ Battlefield"
    ));
}

#[test]
fn wave9b_effect_cost_handsize_permanent_pump_and_if_moved() {
    let hand_size = read(
        "Name:EndlessArchive\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ Hand | Duration$ Permanent\n\
         SVar:Hand:Mode$ Continuous | Affected$ You | SetMaxHandSize$ Unlimited",
    );
    assert!(
        hand_size.abilities[0].contains(
            "Effect::continuous(&Filter::Any, Modifier::NoMaxHandSize, Duration::Indefinitely)"
        ),
        "{}",
        hand_size.abilities[0]
    );

    let spell_tax = read(
        "Name:SpellDelay\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ Tax\n\
         SVar:Tax:Mode$ RaiseCost | ValidCard$ Card.nonCreature | Activator$ Opponent | Type$ Spell | Amount$ 2",
    );
    assert!(
        spell_tax.abilities[0].contains("Modifier::SpellsCostMore(2), Duration::UntilYourNextTurn"),
        "{}",
        spell_tax.abilities[0]
    );

    let spell_discount = read(
        "Name:ForgeDiscount\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ Disc\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Artifact | Activator$ You | Type$ Spell | Amount$ 1",
    );
    assert!(
        spell_discount.abilities[0]
            .contains("Modifier::SpellsCostLess(1), Duration::UntilEndOfTurn"),
        "{}",
        spell_discount.abilities[0]
    );

    let perm_pump = read(
        "Name:IronChimera\nManaCost:4\nTypes:Artifact Creature Construct\nPT:2/2\n\
         A:AB$ Pump | Cost$ Sac<1/CARDNAME> | ValidTgts$ Creature | KW$ First Strike | Duration$ Permanent",
    );
    assert!(
        perm_pump.abilities[0].contains("duration: Duration::Indefinitely"),
        "{}",
        perm_pump.abilities[0]
    );

    let if_bounced = read(
        "Name:RetreatAgent\nManaCost:1 B\nTypes:Creature Rogue\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ Bounce\n\
         SVar:Bounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | RememberChanged$ True | SubAbility$ Drain\n\
         SVar:Drain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ Clean\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    let a = &if_bounced.abilities[0];
    assert!(
        a.contains("Effect::bounce(TargetSpec::ThisObject)")
            && a.contains("Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ReturnedToHand(&Filter::Any)), 1)")
            && a.contains("Effect::LoseLife")
            && a.contains("Effect::gain_life(2)"),
        "{a}"
    );

    // One-shot "next spell" discount (`ForgetOnCast$`) must stay refused.
    assert!(refused(
        "Name:OneShotDiscount\nManaCost:R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ Disc | ForgetOnCast$ Card.YouCtrl\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Card | Activator$ You | Type$ Spell | Amount$ 1"
    ));
}

#[test]
fn wave10b_exerted_trigger_and_command_zone() {
    let watcher = read(
        "Name:DesertWatcher\nManaCost:3 W\nTypes:Creature Cleric\nPT:3/2\n\
         T:Mode$ Exerted | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Tap\n\
         SVar:Tap:DB$ Tap | ValidTgts$ Creature.OppCtrl",
    );
    assert!(
        watcher.abilities[0].contains("Trigger::Exerted(&")
            && watcher.abilities[0].contains("Effect::TapTarget"),
        "{}",
        watcher.abilities[0]
    );

    let self_exert = read(
        "Name:SunRunner\nManaCost:2 R\nTypes:Creature Warrior\nPT:2/2\n\
         T:Mode$ Exerted | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ Pump\n\
         SVar:Pump:DB$ Pump | Defined$ TriggeredCard | NumAtt$ +2",
    );
    assert!(
        self_exert.abilities[0].contains("Trigger::EXERTED_THIS")
            && self_exert.abilities[0].contains("Effect::PumpEventObject"),
        "{}",
        self_exert.abilities[0]
    );

    let cmd_blocks = read(
        "Name:VanguardShield\nTypes:Vanguard\n\
         T:Mode$ Blocks | ValidCard$ Creature.YouCtrl | TriggerZones$ Command | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        cmd_blocks.abilities[0].contains("zone = TriggerZone::Command"),
        "{}",
        cmd_blocks.abilities[0]
    );

    assert!(refused(
        "Name:BadExertNoValid\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Exerted | TriggerZones$ Battlefield | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ 1"
    ));
}

#[test]
fn wave10b_move_counter_and_move_counter_to_event() {
    let from_self = read(
        "Name:CounterRack\nManaCost:4\nTypes:Artifact\n\
         A:AB$ MoveCounter | Cost$ T | Source$ Self | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 1 | SorcerySpeed$ True",
    );
    assert!(
        from_self.abilities[0].contains(
            "Effect::MoveCounter { from: TargetSlot::This, to: TargetSlot::First, kind: Some(CounterKind::P1P1) }"
        ),
        "{}",
        from_self.abilities[0]
    );

    let to_self = read(
        "Name:CounterLeech\nManaCost:3\nTypes:Artifact Creature Construct\nPT:0/0\n\
         A:AB$ MoveCounter | Cost$ 2 | Defined$ Self | ValidTgts$ Creature.YouCtrl | CounterType$ Any | CounterNum$ 1",
    );
    assert!(
        to_self.abilities[0].contains(
            "Effect::MoveCounter { from: TargetSlot::First, to: TargetSlot::This, kind: None }"
        ),
        "{}",
        to_self.abilities[0]
    );

    let to_event = read(
        "Name:BeaconRelic\nManaCost:3 W\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Planeswalker.YouCtrl | TriggerZones$ Battlefield | Execute$ Move\n\
         SVar:Move:DB$ MoveCounter | Source$ Self | Defined$ TriggeredCardLKICopy | CounterType$ LOYALTY",
    );
    assert!(
        to_event.abilities[0].contains("Effect::MoveCounterToEvent { kind: CounterKind::Loyalty }"),
        "{}",
        to_event.abilities[0]
    );

    let two_target_move = read(
        "Name:ShiftGrounds\nTypes:Land\n\
         A:AB$ TargetOnly | Cost$ 1 T | ValidTgts$ Permanent.YouCtrl | SubAbility$ Move\n\
         SVar:Move:DB$ MoveCounter | Source$ ParentTarget | ValidTgts$ Permanent | CounterType$ Any | CounterNum$ 1",
    );
    assert!(
        two_target_move.abilities[0].contains(
            "Effect::MoveCounter { from: TargetSlot::First, to: TargetSlot::Second, kind: None }"
        ) && two_target_move.abilities[0].contains("second_targets = Some("),
        "{}",
        two_target_move.abilities[0]
    );

    assert!(refused(
        "Name:MoveAll\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ MoveCounter | Source$ Self | ValidTgts$ Creature | CounterType$ All | CounterNum$ All"
    ));
    assert!(refused(
        "Name:MoveUnique\nTypes:Land\n\
         A:AB$ TargetOnly | Cost$ 1 T | ValidTgts$ Permanent.YouCtrl | SubAbility$ Move\n\
         SVar:Move:DB$ MoveCounter | Source$ ParentTarget | ValidTgts$ Permanent | TargetUnique$ True | CounterType$ Any | CounterNum$ 1"
    ));
}

#[test]
fn wave10b_two_target_burn_target_only_and_destroy_slots() {
    let two_burn = read(
        "Name:TwinSpark\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ PickPlayer\n\
         SVar:PickPlayer:DB$ TargetOnly | ValidTgts$ Player | SubAbility$ HitBoth\n\
         SVar:HitBoth:DB$ DealDamage | Defined$ Targeted | NumDmg$ 2",
    );
    assert!(
        two_burn.abilities[0]
            .contains("Effect::DealDamageToBothTargetInstances { amount: Amount::Fixed(2) }")
            && two_burn.abilities[0]
                .contains("second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer))"),
        "{}",
        two_burn.abilities[0]
    );

    let boom_shape = read(
        "Name:LandCrack\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Land.YouCtrl | SubAbility$ Second\n\
         SVar:Second:DB$ TargetOnly | ValidTgts$ Land.YouDontCtrl | SubAbility$ Des\n\
         SVar:Des:DB$ Destroy | Defined$ Targeted",
    );
    assert!(
        boom_shape.abilities[0].contains(
            "Effect::DestroyTargetSlots { targets: &[TargetSlot::First, TargetSlot::Second], no_regen: false }"
        ) && boom_shape.abilities[0].contains("second_targets = Some("),
        "{}",
        boom_shape.abilities[0]
    );

    let tap_and_destroy = read(
        "Name:LockBreak\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ Des\n\
         SVar:Des:DB$ Destroy | ValidTgts$ Artifact",
    );
    assert!(
        tap_and_destroy.abilities[0].contains("Effect::TapTargetSlot(TargetSlot::First)")
            && tap_and_destroy.abilities[0].contains(
                "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: false }"
            ),
        "{}",
        tap_and_destroy.abilities[0]
    );

    // Direct two-target DealDamage -> DealDamage or TargetUnique$ on second TargetOnly stay refused.
    assert!(refused(
        "Name:UnequalSpark\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | SubAbility$ HitPlayer\n\
         SVar:HitPlayer:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 3"
    ));
    assert!(refused(
        "Name:UniqueCrack\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Land | SubAbility$ Second\n\
         SVar:Second:DB$ TargetOnly | ValidTgts$ Land | TargetUnique$ True | SubAbility$ Des\n\
         SVar:Des:DB$ Destroy | Defined$ Targeted"
    ));
}

#[test]
fn wave10b_targets_for_each_opponent_and_phase_svar_sweeps() {
    let grasp = read(
        "Name:FateGrasp\nManaCost:1 W W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Permanent.nonLand+OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | Duration$ UntilHostLeavesPlay\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        grasp.abilities[0].contains("Effect::exile_until(TargetSpec::ObjectOfEachOpponent(")
            && grasp.abilities[0].contains("TargetReq::up_to(TargetSpec::ObjectOfEachOpponent("),
        "{}",
        grasp.abilities[0]
    );

    let each_opp_tap = read(
        "Name:MoradinHammer\nManaCost:2 W\nTypes:Creature Dwarf\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ Tap\n\
         SVar:Tap:DB$ Tap | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        each_opp_tap.abilities[0].contains("Effect::TapTarget")
            && each_opp_tap.abilities[0].contains(
                "TargetReq::up_to(TargetSpec::ObjectOfEachOpponent(&Filter::CREATURE), 255)"
            ),
        "{}",
        each_opp_tap.abilities[0]
    );

    let low_hand_and_life = read(
        "Name:TempleFlip\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ H | SVarCompare$ GE1 | Execute$ D\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | CheckSVar$ G | SVarCompare$ EQ0 | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:H:PlayerCount$HasPropertyHasCardsInHand_Card_LE1\n\
         SVar:G:Count$LifeYouGainedThisTurn",
    );
    assert!(
        low_hand_and_life.abilities[0].contains("Condition::APlayerHandSizeAtMost(1)")
            && low_hand_and_life.abilities[1].contains("Condition::Not(&")
            && low_hand_and_life
                .statics
                .contains("Amount::LifeGainedThisTurn(PlayerRel::You)"),
        "{:?} / {:?}",
        low_hand_and_life.abilities,
        low_hand_and_life.statics
    );

    // `PlayerCountPlayers$Amount` includes controller, so must NOT use ObjectOfEachOpponent.
    assert!(refused(
        "Name:AllPlayersExile\nManaCost:3 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.Other | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | Duration$ UntilHostLeavesPlay\n\
         SVar:OneEach:PlayerCountPlayers$Amount"
    ));
}

#[test]
fn wave10b_pump_kwchoice_prevent_valid_and_self_lki_counters() {
    let kw_choice_self = read(
        "Name:AdaptiveBlade\nManaCost:2 G\nTypes:Creature Beast\nPT:3/3\n\
         A:AB$ Pump | Cost$ 1 | KWChoice$ Reach,Trample,Haste",
    );
    assert!(
        kw_choice_self.abilities[0].contains("Effect::ChooseOne { choices: &[")
            && kw_choice_self.abilities[0].contains("keywords: KeywordSet::REACH")
            && kw_choice_self.abilities[0].contains("keywords: KeywordSet::TRAMPLE")
            && kw_choice_self.abilities[0].contains("keywords: KeywordSet::HASTE"),
        "{}",
        kw_choice_self.abilities[0]
    );

    let kw_choice_team = read(
        "Name:SkyMarshal\nManaCost:4 W\nTypes:Creature Angel\nPT:4/4\n\
         T:Mode$ Phase | Phase$ BeginCombat | TriggerZones$ Battlefield | Execute$ Pick\n\
         SVar:Pick:DB$ Pump | Defined$ Valid Creature.YouCtrl | KWChoice$ First Strike,Lifelink",
    );
    assert!(
        kw_choice_team.abilities[0].contains("Effect::ChooseOne { choices: &[")
            && kw_choice_team.abilities[0].contains("filter: &Filter::YOUR_CREATURE")
            && kw_choice_team.abilities[0].contains("KeywordSet::FIRST_STRIKE")
            && kw_choice_team.abilities[0].contains("KeywordSet::LIFELINK"),
        "{}",
        kw_choice_team.abilities[0]
    );

    let prevent_both = read(
        "Name:AegisWard\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ +1 | KW$ Prevent all damage that would be dealt to CARDNAME.",
    );
    assert!(
        prevent_both.abilities[0].contains("Effect::PumpTarget {")
            && prevent_both.abilities[0].contains(
                "Effect::ModifyTargets { modifier: Modifier::PreventDamageFrom(&Filter::Any), duration: Duration::UntilEndOfTurn }"
            ),
        "{}",
        prevent_both.abilities[0]
    );

    let prevent_self_combat = read(
        "Name:MistShade\nManaCost:1 U\nTypes:Creature Illusion\nPT:2/2\n\
         A:AB$ Pump | Cost$ U | Defined$ Self | KW$ Prevent all combat damage that would be dealt to and dealt by CARDNAME.",
    );
    assert!(
        prevent_self_combat.abilities[0]
            .contains("Effect::ModifySource { modifier: Modifier::PreventDamageToIt")
            && prevent_self_combat.abilities[0]
                .contains("Effect::ModifySource { modifier: Modifier::PreventDamageFromIt")
            && !prevent_self_combat.abilities[0].contains("PumpFilter"),
        "{}",
        prevent_self_combat.abilities[0]
    );

    let self_lki_counters = read(
        "Name:AshTracker\nManaCost:3 B\nTypes:Creature Vampire\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ DrawN\n\
         SVar:DrawN:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:TriggeredCard$CardCounters.P1P1/Times.2",
    );
    assert!(
        self_lki_counters.abilities[0].contains(
            "Amount::Times { base: &Amount::CountersOnSource(CounterKind::P1P1), factor: 2 }"
        ),
        "{}",
        self_lki_counters.abilities[0]
    );

    // Non-self TriggeredCard$CardCounters.P1P1 or CardCounters.ALL must stay refused.
    assert!(refused(
        "Name:OtherTracker\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ DrawN\n\
         SVar:DrawN:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:TriggeredCard$CardCounters.P1P1"
    ));
    assert!(refused(
        "Name:AllCounterTracker\nManaCost:3 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ DrawN\n\
         SVar:DrawN:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:TriggeredCard$CardCounters.ALL"
    ));
}

#[test]
fn wave10b_hidden_graveyard_and_battlefield_choose_card() {
    let gy_two = read(
        "Name:CryptFeeder\nManaCost:1 B\nTypes:Creature Horror\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ ExTwo\n\
         SVar:ExTwo:DB$ ChangeZone | Hidden$ True | Mandatory$ True | ChangeType$ Card.YouCtrl | ChangeNum$ 2 | DefinedPlayer$ You | Origin$ Graveyard | Destination$ Exile",
    );
    assert!(
        gy_two.abilities[0].contains(
            "Effect::ChooseCard { chooser: PlayerRel::You, zone: ChooseZone::Graveyard(PlayerRel::You), filter: &Filter::Any, min: 2, max: Amount::Fixed(2), movement: ChooseMove::Exile, then: &[] }"
        ),
        "{}",
        gy_two.abilities[0]
    );

    let gy_one_defined_you = read(
        "Name:RotField\nManaCost:1 B B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ ExOne\n\
         SVar:ExOne:DB$ ChangeZone | Hidden$ True | Mandatory$ True | ChangeType$ Card | ChangeNum$ 1 | DefinedPlayer$ You | Origin$ Graveyard | Destination$ Exile",
    );
    assert!(
        gy_one_defined_you.abilities[0].contains(
            "Effect::ChooseFromYourGraveyard { filter: &Filter::Any, destination: SearchDest::Exile, tapped: false, optional: false }"
        ),
        "{}",
        gy_one_defined_you.abilities[0]
    );

    let def_exile_two = read(
        "Name:VoidCrusher\nManaCost:7\nTypes:Creature Eldrazi\nPT:7/5\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ ExTwo\n\
         SVar:ExTwo:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | DefinedPlayer$ TriggeredDefendingPlayer | Mandatory$ True | ChangeType$ Permanent | ChangeNum$ 2 | Chooser$ DefendingPlayer | Hidden$ True",
    );
    assert!(
        def_exile_two.abilities[0].contains(
            "Effect::ChooseCard { chooser: PlayerRel::DefendingPlayer, zone: ChooseZone::Battlefield, filter: &Filter::ControlledByDefendingPlayer, min: 2, max: Amount::Fixed(2), movement: ChooseMove::Exile, then: &[] }"
        ),
        "{}",
        def_exile_two.abilities[0]
    );

    // Opponent graveyard or multi-player chooser must stay refused.
    assert!(refused(
        "Name:EachReturn\nManaCost:3 B\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Creature | DefinedPlayer$ Player.Other | ChangeNum$ 1 | Hidden$ True | Optional$ True"
    ));
}

#[test]
fn wave11b_changes_zone_all_sacrificed_and_player_attacked_triggers() {
    let token_batch = read(
        "Name:GroveScout\nManaCost:1 G\nTypes:Creature Elf\nPT:2/2\n\
         T:Mode$ ChangesZoneAll | ValidCards$ Card.token+YouCtrl | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ Grow\n\
         SVar:Grow:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:TriggerCount$Amount",
    );
    assert!(
        token_batch.abilities[0].contains(
            "Trigger::ChangesZoneAll { origin: None, destination: Some(ZoneRef::Battlefield),"
        ) && token_batch.abilities[0].contains("exclude_self: false")
            && token_batch.abilities[0].contains("amount: Amount::EventAmount"),
        "{}",
        token_batch.abilities[0]
    );

    let die_batch = read(
        "Name:GraveWatcher\nManaCost:2 B\nTypes:Creature Rogue\nPT:1/3\n\
         T:Mode$ ChangesZoneAll | ValidCards$ Creature.Other | Origin$ Battlefield | Destination$ Graveyard | TriggerZones$ Battlefield | ActivationLimit$ 1 | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1",
    );
    assert!(
        die_batch.abilities[0].contains(
            "Trigger::ChangesZoneAll { origin: Some(ZoneRef::Battlefield), destination: Some(ZoneRef::Graveyard), filter: &Filter::CREATURE, exclude_self: true }"
        ) && die_batch.abilities[0].contains("once_per_turn = true"),
        "{}",
        die_batch.abilities[0]
    );

    let sac_trig = read(
        "Name:VenomWatcher\nManaCost:2 B\nTypes:Creature Horror\nPT:2/3\n\
         T:Mode$ Sacrificed | ValidCard$ Creature.OppCtrl | TriggerZones$ Battlefield | Execute$ Sting\n\
         SVar:Sting:DB$ Poison | Defined$ TriggeredCardController | Num$ X\n\
         SVar:X:TriggeredCard$CardPower",
    );
    assert!(
        sac_trig.abilities[0].contains("Trigger::Sacrificed { who: PlayerRel::EachOpponent,")
            && sac_trig.abilities[0].contains(
                "Effect::Poison { who: PlayerRel::ControllerOfEvent, amount: Amount::EventPower }"
            ),
        "{}",
        sac_trig.abilities[0]
    );

    let sac_once = read(
        "Name:AshSmelter\nManaCost:2 R\nTypes:Creature Giant\nPT:3/3\n\
         T:Mode$ Sacrificed | ValidCard$ Creature.Other+YouCtrl | TriggerZones$ Battlefield | ActivationLimit$ 1 | Execute$ Ping | TriggerDescription$ Whenever you sacrifice one or more other creatures, CARDNAME deals 1 damage to each opponent. This ability triggers only once each turn.\n\
         SVar:Ping:DB$ DealDamage | Defined$ Opponent | NumDmg$ 1",
    );
    assert!(
        sac_once.abilities[0].contains("Trigger::Sacrificed { who: PlayerRel::You,")
            && sac_once.abilities[0].contains("once_per_turn = true"),
        "{}",
        sac_once.abilities[0]
    );

    // "one or more" Sacrificed without ActivationLimit$ 1 must stay refused.
    assert!(refused(
        "Name:BadBatchSac\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Sacrificed | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Ping | TriggerDescription$ Whenever you sacrifice one or more creatures, CARDNAME deals 1 damage to each opponent.\n\
         SVar:Ping:DB$ DealDamage | Defined$ Opponent | NumDmg$ 1"
    ));

    let curse_attack = read(
        "Name:BaneMark\nManaCost:2 B\nTypes:Enchantment Aura Curse\n\
         T:Mode$ AttackersDeclared | AttackedTarget$ Player.EnchantedBy | TriggerZones$ Battlefield | Execute$ Drain\n\
         SVar:Drain:DB$ LoseLife | Defined$ TriggeredAttackingPlayer | LifeAmount$ X\n\
         SVar:X:TriggerCount$Amount",
    );
    assert!(
        curse_attack.abilities[0].contains("Trigger::PlayerAttacked(PlayerRel::EnchantedPlayer)")
            && curse_attack.abilities[0].contains(
                "Effect::LoseLife { amount: Amount::EventAmount, target: PlayerRel::EventPlayer }"
            ),
        "{}",
        curse_attack.abilities[0]
    );

    let mode_attacked = read(
        "Name:DirectMark\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ Attacked | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Heal\n\
         SVar:Heal:DB$ GainLife | LifeAmount$ X\n\
         SVar:X:TriggerCount$Amount",
    );
    assert!(
        mode_attacked.abilities[0].contains("Trigger::PlayerAttacked(PlayerRel::You)")
            && mode_attacked.abilities[0]
                .contains("Effect::GainLife { amount: Amount::EventAmount }"),
        "{}",
        mode_attacked.abilities[0]
    );
}

#[test]
fn wave11b_manifest_turn_face_up_spell_locks_and_hooks() {
    let manifest_exile = read(
        "Name:VoidShift\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature | SubAbility$ Man\n\
         SVar:Man:DB$ Manifest | DefinedPlayer$ TargetedController",
    );
    assert!(
        manifest_exile.abilities[0].contains(
            "Effect::ManifestTop { who: PlayerRel::ControllerOfTarget, count: Amount::Fixed(1) }"
        ),
        "{}",
        manifest_exile.abilities[0]
    );

    let dread = read(
        "Name:DreadWhisper\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ManifestDread",
    );
    assert!(
        dread.abilities[0].contains("Effect::ManifestDread"),
        "{}",
        dread.abilities[0]
    );

    let flip_up = read(
        "Name:UnveilBeacon\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ SetState | Cost$ T | ValidTgts$ Creature.faceDown | Mode$ TurnFaceUp",
    );
    assert!(
        flip_up.abilities[0].contains("Effect::TurnFaceUp { target: TargetSpec::Object("),
        "{}",
        flip_up.abilities[0]
    );
    assert!(refused(
        "Name:FlipDown\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ SetState | Cost$ T | ValidTgts$ Creature | Mode$ TurnFaceDown"
    ));

    let lock_target = read(
        "Name:HushChant\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Player | RememberObjects$ Targeted | StaticAbilities$ NoCast\n\
         SVar:NoCast:Mode$ CantBeCast | ValidCard$ Card | Caster$ Player.IsRemembered",
    );
    assert!(
        lock_target.abilities[0].contains(
            "Effect::continuous(&Filter::Any, Modifier::PlayersCantCast { who: PlayerRel::Chosen, filter: &Filter::Any }, Duration::UntilEndOfTurn)"
        ),
        "{}",
        lock_target.abilities[0]
    );

    let lock_counter = read(
        "Name:SilentCounter\nManaCost:W U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ Lock\n\
         SVar:Lock:DB$ Effect | RememberObjects$ TargetedController | StaticAbilities$ NoCast\n\
         SVar:NoCast:Mode$ CantBeCast | ValidCard$ Card | Caster$ Player.IsRemembered",
    );
    assert!(
        lock_counter.abilities[0].contains(
            "Modifier::PlayersCantCast { who: PlayerRel::ControllerOfTarget, filter: &Filter::Any }"
        ),
        "{}",
        lock_counter.abilities[0]
    );

    // #326: CopySpellAbility omitting MayChooseTarget$ infers true when Oracle says "you may choose new targets".
    let copy_infer = read(
        "Name:RingEcho\nManaCost:2\nTypes:Artifact\n\
         A:AB$ CopySpellAbility | Cost$ 1 T Sac<1/CARDNAME> | TargetType$ Spell | ValidTgts$ Card.YouCtrl\n\
         Oracle:{1}, {T}, Sacrifice RingEcho: Copy target spell you control. You may choose new targets for the copy.",
    );
    assert!(
        copy_infer.abilities[0].contains("Effect::CopyTargetSpell { mods: &[] }"),
        "{}",
        copy_infer.abilities[0]
    );
    assert!(refused(
        "Name:MandatoryRetarget\nManaCost:2\nTypes:Artifact\n\
         A:AB$ CopySpellAbility | Cost$ T | TargetType$ Spell | ValidTgts$ Card\n\
         Oracle:{T}: Copy target spell. Choose new targets for the copy."
    ));

    // Dune Mover shape: ETB search saying "you may search your library" without Optional$ True sets optional: true.
    let dune = read(
        "Name:SandWalker\nManaCost:2\nTypes:Artifact Creature Golem\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Find | TriggerDescription$ When CARDNAME enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on top.\n\
         SVar:Find:DB$ ChangeZone | Origin$ Library | Destination$ Library | LibraryPosition$ 0 | ChangeType$ Land.Basic\n\
         Oracle:When SandWalker enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on top.",
    );
    assert!(
        dune.abilities[0].contains("optional: true"),
        "{}",
        dune.abilities[0]
    );
}

#[test]
fn wave11b_object_matches_and_amount_compare_conditions() {
    let snare_shape = read(
        "Name:FlameSnare\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.attacking | KW$ Prevent all damage that would be dealt by CARDNAME. | SubAbility$ Des\n\
         SVar:Des:DB$ Destroy | Defined$ Targeted | ConditionDefined$ Targeted | ConditionPresent$ Creature.Black,Creature.Red",
    );
    assert!(
        snare_shape.abilities[0].contains(
            "Condition::ObjectMatches { what: &TargetSpec::Object(&Filter::Any), filter: &"
        ),
        "{}",
        snare_shape.abilities[0]
    );

    let sac_nontoken = read(
        "Name:SoulCatcher\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Sacrificed | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Grow\n\
         SVar:Grow:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | ConditionDefined$ TriggeredCardLKICopy | ConditionNotPresent$ Creature.token",
    );
    assert!(
        sac_nontoken.abilities[0].contains(
            "Condition::Not(&Condition::ObjectMatches { what: &TargetSpec::EventObject, filter: &"
        ),
        "{}",
        sac_nontoken.abilities[0]
    );

    let cmp_trig = read(
        "Name:BalanceRelic\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ MyCreatures | SVarCompare$ NEMyLands | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:MyCreatures:Count$Valid Creature.YouCtrl\n\
         SVar:MyLands:Count$Valid Land.YouCtrl",
    );
    assert!(
        cmp_trig.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::CountOf { filter: &Filter::YOUR_CREATURE, zone: ZoneSel::Battlefield }, op: AmountCmp::NotEqual, right: &Amount::CountOf { filter: &Filter::YOUR_LAND, zone: ZoneSel::Battlefield } }"
        ),
        "{}",
        cmp_trig.abilities[0]
    );

    let cmp_res = read(
        "Name:PowerCheck\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ Bonus\n\
         SVar:Bonus:DB$ Draw | NumCards$ 1 | ConditionCheckSVar$ P | ConditionSVarCompare$ NE2\n\
         SVar:P:Targeted$CardPower",
    );
    assert!(
        cmp_res.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::TargetPower, op: AmountCmp::NotEqual, right: &Amount::Fixed(2) }"
        ),
        "{}",
        cmp_res.abilities[0]
    );

    // Resolution-only TargetPower must stay refused in trigger CheckSVar$.
    assert!(refused(
        "Name:BadTriggerCmp\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ P | SVarCompare$ GE4 | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:P:Targeted$CardPower"
    ));
}

#[test]
fn wave11b_pass2_unlocks_and_honesty_guards() {
    // ChangesZoneAll with omitted Destination$ (leaving battlefield) + Sacrificed with inert Origin$/Destination$.
    let ltb_batch = read(
        "Name:ForgeAnvil\nManaCost:B R\nTypes:Artifact\n\
         T:Mode$ ChangesZoneAll | ValidCards$ Artifact.YouCtrl | Origin$ Battlefield | TriggerZones$ Battlefield | PlayerTurn$ True | ActivationLimit$ 1 | Execute$ D\n\
         SVar:D:DB$ Scry | ScryNum$ 1",
    );
    assert!(
        ltb_batch.abilities[0].contains(
            "Trigger::ChangesZoneAll { origin: Some(ZoneRef::Battlefield), destination: None,"
        ),
        "{}",
        ltb_batch.abilities[0]
    );

    let sac_inert = read(
        "Name:TokenWatcher\nManaCost:1 U\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Sacrificed | ValidPlayer$ You | ValidCard$ Permanent.token+YouCtrl | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ S\n\
         SVar:S:DB$ Surveil | Amount$ 1",
    );
    assert!(
        sac_inert.abilities[0].contains("Trigger::Sacrificed { who: PlayerRel::You,"),
        "{}",
        sac_inert.abilities[0]
    );

    // Drawn TriggeredPlayer and BecomesTarget TriggeredSourceController bind EventPlayer.
    let draw_burn = read(
        "Name:DrawPunisher\nManaCost:3 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Drawn | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ Ping\n\
         SVar:Ping:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1",
    );
    assert!(
        draw_burn.abilities[0].contains("TargetSpec::Player(PlayerRel::EventPlayer)"),
        "{}",
        draw_burn.abilities[0]
    );

    let bolt_back = read(
        "Name:ScaleGuard\nManaCost:2 R R\nTypes:Creature\nPT:4/4\n\
         T:Mode$ BecomesTarget | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ Zap\n\
         SVar:Zap:DB$ DealDamage | Defined$ TriggeredSourceController | NumDmg$ 3",
    );
    assert!(
        bolt_back.abilities[0].contains("TargetSpec::Player(PlayerRel::EventPlayer)"),
        "{}",
        bolt_back.abilities[0]
    );

    // Implicit-Self and optional Transform / TurnFaceUp.
    let opt_transform = read(
        "Name:SunCompass\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Flip\n\
         SVar:Flip:DB$ SetState | Mode$ Transform | Optional$ True",
    );
    assert!(
        opt_transform.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::TransformSource] }"),
        "{}",
        opt_transform.abilities[0]
    );

    // Targeted spell followed by non-targeting ManifestDread / Proliferate / ChangeZoneAll(Graveyard->Exile).
    let bounce_dread = read(
        "Name:EerieGrasp\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent.nonLand | Origin$ Battlefield | Destination$ Hand | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ Dread\n\
         SVar:Dread:DB$ ManifestDread",
    );
    assert!(
        bounce_dread.abilities[0].contains("Effect::ManifestDread"),
        "{}",
        bounce_dread.abilities[0]
    );

    let reclaim_exile_all = read(
        "Name:MemoryVault\nManaCost:3\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ 2 T | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Card.YouOwn | SubAbility$ Wipe\n\
         SVar:Wipe:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card",
    );
    assert!(
        reclaim_exile_all.abilities[0]
            .contains("Effect::ExileGraveyard { player: PlayerRel::EachPlayer }"),
        "{}",
        reclaim_exile_all.abilities[0]
    );

    // Target deals damage equal to its power to source.
    let counter_punch = read(
        "Name:ArenaBrawler\nManaCost:3 R\nTypes:Creature\nPT:4/4\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Creature | NumDmg$ 2 | SubAbility$ Retaliate\n\
         SVar:Retaliate:DB$ DealDamage | DamageSource$ Targeted | Defined$ Self | NumDmg$ TheirP\n\
         SVar:TheirP:Targeted$CardPower",
    );
    assert!(
        counter_punch.abilities[0].contains(
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::This }"
        ),
        "{}",
        counter_punch.abilities[0]
    );

    // Effect APIs: CantActivate, CantBlock(&Filter::This), CantAttackYou, AttackTaxMana,
    // UntilHostLeavesPlayOrEOT on EffectSource, and Grace win/loss/life-floor replacements.
}

#[test]
fn wave11b_pass2_effect_apis_graveyard_and_turn_history() {
    let lock_all_act = read(
        "Name:CoilBind\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Permanent | RememberObjects$ Targeted | StaticAbilities$ Lock\n\
         SVar:Lock:Mode$ CantBeActivated | ValidCard$ Card.IsRemembered",
    );
    assert!(
        lock_all_act.abilities[0]
            .contains("Effect::ModifyTargets { modifier: Modifier::CantActivate, duration: Duration::UntilEndOfTurn }"),
        "{}",
        lock_all_act.abilities[0]
    );

    let cant_block_me = read(
        "Name:TunnelCrawler\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
         A:AB$ Effect | Cost$ 1 R | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ NoBlk\n\
         SVar:NoBlk:Mode$ CantBlockBy | ValidAttacker$ Creature.EffectSource | ValidBlocker$ Creature.IsRemembered",
    );
    assert!(
        cant_block_me.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::CantBlock(&Filter::This), duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        cant_block_me.abilities[0]
    );

    let tax_and_safe = read(
        "Name:HallowedVow\nManaCost:1 W W\nTypes:Instant\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ Tax,NoAttack\n\
         SVar:Tax:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You,Planeswalker.YouCtrl | Cost$ 2\n\
         SVar:NoAttack:Mode$ CantAttack | ValidCard$ Creature | Target$ You",
    );
    assert!(
        tax_and_safe.abilities[0].contains("Modifier::AttackTaxMana { mana: 2 }")
            && tax_and_safe.abilities[0].contains("Modifier::CantAttackYou"),
        "{}",
        tax_and_safe.abilities[0]
    );

    let grace = read(
        "Name:RadiantGrace\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ Keep | ReplacementEffects$ NoLose,NoWin,Floor\n\
         SVar:Keep:Mode$ CantLoseLife | ValidPlayer$ You\n\
         SVar:NoLose:Event$ GameLoss | ValidPlayer$ You | Layer$ CantHappen\n\
         SVar:NoWin:Event$ GameWin | ValidPlayer$ Opponent | Layer$ CantHappen | Secondary$ True\n\
         SVar:Floor:Event$ LifeReduced | ValidPlayer$ You.lifeGE1 | Result$ LT1 | IsDamage$ True | ReplaceWith$ Rep\n\
         SVar:Rep:DB$ ReplaceEffect | VarName$ Amount | VarValue$ X\n\
         SVar:X:Count$YourLifeTotal/Minus.1",
    );
    assert!(
        grace.abilities[0].contains("Modifier::CantLoseLife { who: PlayerRel::You }")
            && grace.abilities[0].contains("Modifier::NamedPlayersCantLose(PlayerRel::You)")
            && grace.abilities[0].contains("Modifier::PlayersCantWin(PlayerRel::EachOpponent)")
            && grace.abilities[0]
                .contains("Modifier::DamageLifeLossFloor { players: PlayerRel::You, minimum: 1 }"),
        "{}",
        grace.abilities[0]
    );

    let nimble = read(
        "Name:PastryRunner\nManaCost:1\nTypes:Artifact Creature\nPT:1/1\n\
         A:AB$ Effect | Cost$ 1 | StaticAbilities$ Evade | Duration$ UntilHostLeavesPlayOrEOT\n\
         SVar:Evade:Mode$ CantBlockBy | ValidAttacker$ Creature.EffectSource | ValidBlocker$ Creature.withoutHaste",
    );
    assert!(
        nimble.abilities[0].contains("Effect::ModifySource { modifier: Modifier::CantBeBlockedBy("),
        "{}",
        nimble.abilities[0]
    );

    // Non-combat CantPreventDamage must stay refused.
    assert!(refused(
        "Name:AllUnpreventable\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ NoPrev\n\
         SVar:NoPrev:Mode$ CantPreventDamage"
    ));

    // Untargeted exile from any graveyard via ChooseCard; opponent reanimate without GainControl$ refused.
    let exile_any_gy = read(
        "Name:GraveCleanser\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card | ChangeNum$ 1 | Mandatory$ True | Hidden$ True",
    );
    assert!(
        exile_any_gy.abilities[0].contains(
            "Effect::ChooseCard { chooser: PlayerRel::You, zone: ChooseZone::Graveyard(PlayerRel::EachPlayer),"
        ),
        "{}",
        exile_any_gy.abilities[0]
    );
    assert!(refused(
        "Name:MissingGainCtrl\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Creature.OppOwn | ChangeNum$ 1 | Mandatory$ True | Hidden$ True"
    ));

    // Turn-history CheckSVar condition: EnteredSubtypeUnderYourControlThisTurn.
    let goblin_check = read(
        "Name:WarrenBell\nManaCost:1 R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ G | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:G:Count$ThisTurnEntered_Battlefield_Goblin.YouCtrl",
    );
    assert!(
        goblin_check.abilities[0].contains(
            "Condition::EnteredSubtypeUnderYourControlThisTurn(&[subtypes::creature::GOBLIN])"
        ),
        "{}",
        goblin_check.abilities[0]
    );
}

#[test]
fn wave12c_wave5b_heroic_enlist_and_bolster() {
    let heroic = read(
        "Name:HeroicScout\nManaCost:1 W\nTypes:Creature\nPT:2/1\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | TargetsValid$ Card.Self | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        heroic.abilities[0].contains("Trigger::HEROIC"),
        "{}",
        heroic.abilities[0]
    );

    let single_tgt = read(
        "Name:WardWatcher\nManaCost:2 U\nTypes:Creature\nPT:2/3\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ Opponent | TargetsValid$ You,Creature.YouCtrl+inZoneBattlefield | IsSingleTarget$ True | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ Draw | NumCards$ 1",
    );
    assert!(
        single_tgt.abilities[0].contains("Trigger::SpellCastTargeting {")
            && single_tgt.abilities[0].contains("who: PlayerRel::EachOpponent")
            && single_tgt.abilities[0].contains("targets_you: true")
            && single_tgt.abilities[0].contains("only: true"),
        "{}",
        single_tgt.abilities[0]
    );
    assert!(refused(
        "Name:GraveTargetSpell\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ SpellCast | ValidActivatingPlayer$ You | TargetsValid$ Card.inZoneGraveyard | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ Draw | NumCards$ 1"
    ));

    let enlisted = read(
        "Name:EnlistVanguard\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Enlisted | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ Scry | ScryNum$ 2",
    );
    assert!(
        enlisted.abilities[0].contains("Trigger::ENLISTED_THIS"),
        "{}",
        enlisted.abilities[0]
    );
    assert!(refused(
        "Name:EnlistFilterRefused\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Enlisted | ValidCard$ Card.Self | ValidEnlisted$ Creature.Goblin | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ Draw | NumCards$ 1"
    ));

    let bolster_chain = read(
        "Name:TacticalBolster\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | SubAbility$ DBBolster\n\
         SVar:DBBolster:DB$ PutCounter | Bolster$ 2",
    );
    assert!(
        bolster_chain.abilities[0].contains("Effect::Bolster(Amount::Fixed(2))"),
        "{}",
        bolster_chain.abilities[0]
    );
    assert!(refused(
        "Name:BolsterRemember\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ PutCounter | Bolster$ 1 | RememberCards$ True"
    ));
}

#[test]
fn wave12c_damage_prevention_effect_and_exchange_control() {
    let fog_pair = read(
        "Name:TruceBlade\nManaCost:W\nTypes:Creature\nPT:1/1\n\
         A:AB$ Effect | Cost$ 1 W | ValidTgts$ Creature | RememberObjects$ Targeted | ExileOnMoved$ Battlefield | ReplacementEffects$ R1,R2\n\
         SVar:R1:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidSource$ Card.IsRemembered\n\
         SVar:R2:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidTarget$ Card.IsRemembered",
    );
    assert!(
        fog_pair.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::PreventDamageFromIt, duration: Duration::UntilEndOfTurn }"
        ) && fog_pair.abilities[0].contains(
            "Effect::ModifyTargets { modifier: Modifier::PreventDamageToIt, duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        fog_pair.abilities[0]
    );

    let self_shield = read(
        "Name:SafeGuard\nManaCost:1 W\nTypes:Creature\nPT:1/2\n\
         A:AB$ Effect | Cost$ T | Duration$ UntilHostLeavesPlayOrEOT | ReplacementEffects$ R1\n\
         SVar:R1:Event$ DamageDone | Prevent$ True | ValidTarget$ Card.EffectSource",
    );
    assert!(
        self_shield.abilities[0].contains(
            "Effect::ModifySource { modifier: Modifier::PreventDamageFrom(&Filter::Any), duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        self_shield.abilities[0]
    );
    assert!(refused(
        "Name:CombatOnlyYouNextTurn\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | ReplacementEffects$ R1\n\
         SVar:R1:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidTarget$ You"
    ));

    let swap = read(
        "Name:ControlSwap\nManaCost:4 U\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ DBEx\n\
         SVar:DBEx:DB$ ExchangeControl | Defined$ ParentTarget | ValidTgts$ Creature",
    );
    assert!(
        swap.abilities[0].contains("Effect::ExchangeControl")
            && swap.abilities[0].contains("second_targets"),
        "{}",
        swap.abilities[0]
    );
}

#[test]
fn wave12c_remembered_zone_changes_and_exiled_with_guard() {
    let fetch_untap = read(
        "Name:PassageLand\nManaCost:no cost\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land | Tapped$ True | RememberChanged$ True | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Remembered | ConditionPresent$ Land.YouCtrl | ConditionCompare$ GE4 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        fetch_untap.abilities[0].contains("Effect::SearchLibrary")
            && fetch_untap.abilities[0].contains("Effect::UntapFound"),
        "{}",
        fetch_untap.abilities[0]
    );

    let gy_exile_cond = read(
        "Name:GraveScavenger\nManaCost:2 G\nTypes:Creature\nPT:3/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ T\n\
         SVar:T:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 1 | RememberChanged$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Self | NumAtt$ +1 | NumDef$ +1 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ0 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        gy_exile_cond.abilities[0].contains("Condition::ObjectMatches")
            && gy_exile_cond
                .statics
                .contains("Filter::InZone(ZoneRef::Exile)")
            && gy_exile_cond.statics.contains("Filter::Not("),
        "{}\n{}",
        gy_exile_cond.statics,
        gy_exile_cond.abilities[0]
    );

    let reanimate_mod = read(
        "Name:HastyReturn\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature | RememberChanged$ True | SubAbility$ DBPC\n\
         SVar:DBPC:DB$ PutCounter | Defined$ Remembered | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBAnim\n\
         SVar:DBAnim:DB$ Animate | Defined$ Remembered | Duration$ Permanent | Keywords$ Haste | LeaveBattlefield$ Exile | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        reanimate_mod.abilities[0].contains("Effect::GraveyardToBattlefieldModified")
            && reanimate_mod.abilities[0].contains("Some((CounterKind::P1P1, 1))")
            && reanimate_mod.abilities[0].contains("Modifier::AddKeyword(KeywordSet::HASTE)")
            && reanimate_mod.abilities[0].contains("Modifier::ExileInsteadOfLeavingBattlefield"),
        "{}",
        reanimate_mod.abilities[0]
    );

    let archive = read(
        "Name:HiddenArchive\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TEx\n\
         SVar:TEx:DB$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Exile | ExileFaceDown$ True\n\
         A:AB$ ChangeZoneAll | Cost$ 5 Sac<1/CARDNAME> | ChangeType$ Card.ExiledWithSource | Origin$ Exile | Destination$ Hand",
    );
    assert!(
        archive.abilities[0].contains("Effect::ExileTopFaceDownLinked")
            && archive.abilities[1].contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }"),
        "{}\n{}",
        archive.abilities[0],
        archive.abilities[1]
    );

    let thresh_search = read(
        "Name:DeepWander\nManaCost:2 G\nTypes:Sorcery\n\
         Oracle:Search your library for a Plains card. Threshold — If there are seven or more cards in your graveyard, instead search your library for up to three Plains cards.\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | Tapped$ True | ChangeNum$ X\n\
         SVar:X:Count$Threshold.3.1",
    );
    assert!(
        thresh_search.abilities[0].contains("Condition::GraveyardCountAtLeast(7)")
            && thresh_search.abilities[0].contains(
                "then: &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED], optional: true }]"
            )
            && thresh_search.abilities[0].contains(
                "otherwise: &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED], optional: false }]"
            ),
        "{}",
        thresh_search.abilities[0]
    );

    // #349/#350 guard: ChooseMove::Exile must refuse cards reading ExiledWith/ExiledWithSource.
    assert!(refused(
        "Name:UnlinkedExileNewt\nManaCost:1 U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEx\n\
         SVar:TrigEx:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ChangeType$ Land.YouCtrl | Hidden$ True | Mandatory$ True\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ChangeZoneAll | ChangeType$ Card.ExiledWithSource | Origin$ Exile | Destination$ Battlefield"
    ));
}

#[test]
fn wave12c_pass2_effect_statics_defender_land_plays_and_colors() {
    let wall_march = read(
        "Name:MarchingWall\nManaCost:1 W\nTypes:Creature Wall\nPT:0/4\nK:Defender\n\
         A:AB$ Effect | Cost$ 1 W | StaticAbilities$ CanAtk\n\
         SVar:CanAtk:Mode$ CanAttackDefender | ValidCard$ Card.EffectSource",
    );
    assert!(
        wall_march.abilities[0].contains("Modifier::AttacksDespiteDefender"),
        "{}",
        wall_march.abilities[0]
    );

    let extra_land = read(
        "Name:ExplorePath\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ LandDrop | SubAbility$ DBDraw\n\
         SVar:LandDrop:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 1\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        extra_land.abilities[0].contains("Modifier::ExtraLandDrops(1)"),
        "{}",
        extra_land.abilities[0]
    );

    let color_and_type = read(
        "Name:GoblinTouch\nManaCost:G\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ ST\n\
         SVar:ST:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Goblin | SetColor$ Green",
    );
    assert!(
        color_and_type.abilities[0].contains("Modifier::AddSubtype(subtypes::creature::GOBLIN)")
            && color_and_type.abilities[0]
                .contains("Modifier::SetColor(ColorSet::from_slice(&[Color::Green]))"),
        "{}",
        color_and_type.abilities[0]
    );

    let multi_fog = read(
        "Name:TwinTruce\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2 | RememberObjects$ Targeted | ReplacementEffects$ R1\n\
         SVar:R1:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidSource$ Card.IsRemembered",
    );
    assert!(
        multi_fog.abilities[0]
            .contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)")
            && multi_fog.abilities[0].contains("Modifier::PreventDamageFromIt"),
        "{}",
        multi_fog.abilities[0]
    );
}

#[test]
fn wave13c_fix_374_permanent_spell_cast_and_token_copy_guard() {
    // #374 CR 110.4b: `Permanent.<Subtype>` on SpellCast keeps the permanent
    // type union so Kindred Instant/Sorcery spells cannot falsely match.
    let echo_mage = read(
        "Name:EchoWizard\nManaCost:3 U U\nTypes:Creature Wizard\nPT:4/4\n\
         T:Mode$ SpellCast | ValidCard$ Permanent.Wizard,Permanent.Goblin | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility",
    );
    assert!(
        echo_mage
            .statics
            .contains("TypeSet::ARTIFACT.union(TypeSet::BATTLE)")
            && echo_mage.statics.contains("subtypes::creature::WIZARD")
            && echo_mage.statics.contains("subtypes::creature::GOBLIN"),
        "{}\n{}",
        echo_mage.statics,
        echo_mage.abilities[0]
    );

    // #374 CR 111.10/111.13: CopySpellAbility must refuse cast triggers whose
    // Oracle creates a token copy directly on the battlefield.
    assert!(refused(
        "Name:TokenCastDouble\nManaCost:3 U\nTypes:Legendary Creature\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | NonLegendary$ True\n\
         Oracle:When you cast this spell, create a token that's a copy of it, except it isn't legendary."
    ));
    // A real spell-copying card that also has another token-copying sentence stays allowed.
    let real_copy = read(
        "Name:DualMirror\nManaCost:2 U R\nTypes:Instant\n\
         A:SP$ CopySpellAbility | TargetType$ Spell | ValidTgts$ Instant,Sorcery | MayChooseTarget$ True\n\
         Oracle:Copy target instant or sorcery spell. You may choose new targets for the copy. (Other cards may create a token that's a copy.)",
    );
    assert!(
        real_copy.abilities[0].contains("Effect::CopyTargetSpell"),
        "{}",
        real_copy.abilities[0]
    );
}

#[test]
fn wave13c_wave6b_plotted_trigger_and_plot_effects() {
    let exile_plot_trig = read(
        "Name:PlottedBrawler\nManaCost:1 G\nTypes:Creature\nPT:3/2\n\
         T:Mode$ BecomesPlotted | ValidCard$ Card.Self | TriggerZones$ Exile | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2",
    );
    assert!(
        exile_plot_trig.abilities[0]
            .contains("Trigger::Plotted { who: PlayerRel::EachPlayer, filter: &Filter::This }")
            && exile_plot_trig.abilities[0].contains("zone = TriggerZone::Exile"),
        "{}",
        exile_plot_trig.abilities[0]
    );

    let plot_spell = read(
        "Name:PlotBird\nManaCost:1 W W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEx\n\
         SVar:TrigEx:DB$ ChangeZone | TargetType$ Spell | ValidTgts$ Card | TgtZone$ Stack | Origin$ Stack | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlot\n\
         SVar:DBPlot:DB$ AlterAttribute | Defined$ Remembered | Attributes$ Plotted | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        plot_spell.abilities[0]
            .contains("Effect::Plot { target: TargetSpec::Spell(&Filter::Any) }"),
        "{}",
        plot_spell.abilities[0]
    );

    let plot_top = read(
        "Name:TopPlotter\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlot\n\
         SVar:DBPlot:DB$ AlterAttribute | Defined$ Remembered | Attributes$ Plotted | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        plot_top.abilities[0]
            .contains("Effect::PlotTop { who: PlayerRel::You, count: Amount::Fixed(1) }"),
        "{}",
        plot_top.abilities[0]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave13c_stack_zone_change_counter_abilities_and_deal_damage() {
    let bounce_spell = read(
        "Name:DelayBounce\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Card | TgtZone$ Stack | Origin$ Stack | Destination$ Hand | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        bounce_spell.abilities[0].contains("Effect::bounce(TargetSpec::Spell(&Filter::Any))"),
        "{}",
        bounce_spell.abilities[0]
    );

    let stifle = read(
        "Name:HushAbility\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Activated,Triggered | ValidTgts$ Card,Emblem",
    );
    assert!(
        stifle.abilities[0].contains("Effect::CounterTargetAbility")
            && stifle.abilities[0].contains("TargetSpec::AbilityOnStack(&Filter::Any)"),
        "{}",
        stifle.abilities[0]
    );

    let disallow = read(
        "Name:BlanketNo\nManaCost:1 U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell,Activated,Triggered | ValidTgts$ Card,Emblem",
    );
    assert!(
        disallow.abilities[0].contains("Effect::CounterTargetSpellOrAbility")
            && disallow.abilities[0].contains("TargetSpec::SpellOrAbility(&Filter::Any)"),
        "{}",
        disallow.abilities[0]
    );

    let stack_or_perm_bounce = read(
        "Name:SpellOrCreatureBounce\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature,Card.inZoneStack | TgtZone$ Stack,Battlefield | Origin$ Battlefield,Stack | Destination$ Hand",
    );
    assert!(
        stack_or_perm_bounce.abilities[0]
            .contains("Effect::bounce(TargetSpec::StackOrBattlefield("),
        "{}",
        stack_or_perm_bounce.abilities[0]
    );
    assert!(refused(
        "Name:RestrictedStackBounce\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent,Instant.inZoneStack | TgtZone$ Stack,Battlefield | Origin$ Battlefield,Stack | Destination$ Hand"
    ));

    let bite_pw = read(
        "Name:BiteDownTest\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature,Planeswalker | DamageSource$ ParentTarget | NumDmg$ X\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        bite_pw.abilities[0].contains(
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::Second }"
        ) && bite_pw.abilities[0].contains("Filter::CREATURE_OR_PLANESWALKER"),
        "{}",
        bite_pw.abilities[0]
    );

    let self_punch = read(
        "Name:TauntWall\nManaCost:2 R\nTypes:Creature\nPT:1/5\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Creature | Defined$ Self | DamageSource$ Targeted | NumDmg$ X\n\
         SVar:X:Targeted$CardPower",
    );
    assert!(
        self_punch.abilities[0].contains(
            "Effect::DamageEqualToPower { dealer: TargetSlot::First, to: TargetSlot::This }"
        ),
        "{}",
        self_punch.abilities[0]
    );

    let burn_then_destroy = read(
        "Name:BurnAndBreak\nManaCost:2 R W\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | SubAbility$ DBKill\n\
         SVar:DBKill:DB$ Destroy | ValidTgts$ Artifact",
    );
    assert!(
        burn_then_destroy.abilities[0].contains(
            "Effect::DestroyTargetSlots { targets: &[TargetSlot::Second], no_regen: false }"
        ),
        "{}",
        burn_then_destroy.abilities[0]
    );

    let brainstorm_like = read(
        "Name:MindSurge\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 3 | SubAbility$ DBPutBack\n\
         SVar:DBPutBack:DB$ ChangeZone | Origin$ Hand | Destination$ Library | ChangeNum$ 2 | Mandatory$ True | Reorder$ True",
    );
    assert!(
        brainstorm_like.abilities[0].contains("Effect::PutFromHandOnTop { count: 2 }"),
        "{}",
        brainstorm_like.abilities[0]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave14c_connive_explore_discover_set_life_and_magecraft() {
    let self_connive = read(
        "Name:VaultScout\nManaCost:1 U\nTypes:Creature\nPT:1/3\n\
         A:AB$ Connive | Cost$ 2",
    );
    assert!(
        self_connive.abilities[0].contains("Effect::Connive { amount: Amount::Fixed(1) }"),
        "{}",
        self_connive.abilities[0]
    );

    let tgt_connive = read(
        "Name:SchemeTactician\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Connive | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 1 | ConniveNum$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        tgt_connive.abilities[0].contains("Effect::Connive { amount: Amount::CountOf {")
            && tgt_connive.abilities[0].contains("TargetReq::up_to_one("),
        "{}",
        tgt_connive.abilities[0]
    );

    // Connive cannot aim at Self when the chain already targets another object,
    // nor at arbitrary non-target subjects.
    assert!(refused(
        "Name:BadConniveOther\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Tap | Cost$ T | ValidTgts$ Creature | SubAbility$ DBC\n\
         SVar:DBC:DB$ Connive | Defined$ Self"
    ));
    assert!(refused(
        "Name:BadConniveConvoked\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Connive | Defined$ Convoked"
    ));

    let double_explore = read(
        "Name:DeepRanger\nManaCost:1 G G\nTypes:Creature\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ Explore | Num$ 2",
    );
    assert_eq!(
        double_explore.abilities[0]
            .matches("Effect::Explore { target: TargetSpec::ThisObject }")
            .count(),
        2,
        "{}",
        double_explore.abilities[0]
    );

    let etb_explore = read(
        "Name:DiscoveryPath\nManaCost:3 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Ex\n\
         SVar:Ex:DB$ Explore | Defined$ TriggeredCardLKICopy",
    );
    assert!(
        etb_explore.abilities[0].contains("Effect::Explore { target: TargetSpec::EventObject }"),
        "{}",
        etb_explore.abilities[0]
    );
    assert!(refused(
        "Name:BadExploreAll\nManaCost:2 G U\nTypes:Creature\nPT:3/3\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Ex\n\
         SVar:Ex:DB$ Explore | Defined$ Valid Creature.YouCtrl"
    ));

    let disc_fixed = read(
        "Name:CaveRelic\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ Discover | Num$ 4",
    );
    assert!(
        disc_fixed.abilities[0].contains("Effect::Discover { mana_value: 4 }"),
        "{}",
        disc_fixed.abilities[0]
    );
    let disc_dyn = read(
        "Name:ScaledDiscovery\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ Discover | Num$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        disc_dyn.abilities[0].contains("Effect::DiscoverAmount { mana_value: Amount::X }"),
        "{}",
        disc_dyn.abilities[0]
    );
    assert!(refused(
        "Name:BadOtherDiscover\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Discover | Defined$ TargetedOwner | Num$ 3"
    ));

    let set_life = read(
        "Name:LifeSetter\nManaCost:3 W B\nTypes:Sorcery\n\
         A:SP$ SetLife | ValidTgts$ Opponent | TargetMin$ 0 | TargetMax$ 1 | LifeAmount$ 10",
    );
    assert!(
        set_life.abilities[0]
            .contains("Effect::SetLifeTotal { who: PlayerRel::Chosen, amount: Amount::Fixed(10) }")
            && set_life.abilities[0].contains("TargetReq::up_to_one(TargetSpec::AnyOpponent)"),
        "{}",
        set_life.abilities[0]
    );
    assert!(refused(
        "Name:BadRedistribute\nManaCost:4 W W\nTypes:Sorcery\n\
         A:SP$ SetLife | PlayerChoices$ Player | ChoiceAmount$ Any | Redistribute$ True"
    ));

    let magecraft = read(
        "Name:SpellScholar\nManaCost:2 U U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ SpellCastOrCopy | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        magecraft.abilities[0].contains("Trigger::SpellCastOrCopied(&TRIGGER1)")
            && magecraft.statics.contains("TypeSet::INSTANT")
            && magecraft.statics.contains("TypeSet::SORCERY"),
        "{}\n{}",
        magecraft.statics,
        magecraft.abilities[0]
    );
    assert!(refused(
        "Name:BadZonelessMagecraft\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ SpellCastOrCopy | ValidCard$ Instant,Sorcery | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave14c_pass_e7_self_return_modal_second_targets_and_flashback() {
    // Pass E7 fix: omitted GainControl$ respects Oracle ("under its owner's control" vs not).
    let front_owner = read(
        "Name:KnightDominant\nManaCost:3 W\nTypes:Creature\nPT:3/3\n\
         A:AB$ ChangeZone | Cost$ 4 W T | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | Transformed$ True | SubAbility$ DBCln\n\
         SVar:DBCln:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:{4}{W}, {T}: Exile KnightDominant, then return it to the battlefield transformed under its owner's control.",
    );
    assert!(
        front_owner.abilities[0]
            .contains("Effect::ExileSelfReturnAsFace { face: 1, owner_control: true }"),
        "{}",
        front_owner.abilities[0]
    );
    let back_front_up = read(
        "Name:WardenSaga\nManaCost:no cost\nTypes:Enchantment Creature Saga\nPT:5/5\n\
         K:Chapter:1:DBEx\n\
         SVar:DBEx:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | SubAbility$ DBCln\n\
         SVar:DBCln:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:I — Exile WardenSaga, then return it to the battlefield (front face up).",
    );
    assert!(
        back_front_up.abilities[0]
            .contains("Effect::ExileSelfReturnAsFace { face: 0, owner_control: false }"),
        "{}",
        back_front_up.abilities[0]
    );

    // Wave 6a: ModalTriggered and ModalActivated carrying SpellMode::second_targets.
    let modal_trig = read(
        "Name:ModalBiter\nManaCost:3 G\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigC\n\
         SVar:TrigC:DB$ Charm | Choices$ M1,M2\n\
         SVar:M1:DB$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ M1Fight\n\
         SVar:M1Fight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.OppCtrl\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 3",
    );
    assert!(
        modal_trig.abilities[0].contains("modal_triggered!(")
            && modal_trig.abilities[0].contains("second_targets = Some("),
        "{}",
        modal_trig.abilities[0]
    );
    let modal_act = read(
        "Name:ModalTool\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ T | Choices$ M1,M2\n\
         SVar:M1:DB$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ M1Fight\n\
         SVar:M1Fight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.OppCtrl\n\
         SVar:M2:DB$ Draw | NumCards$ 1",
    );
    assert!(
        modal_act.abilities[0].contains("modal_activated!(")
            && modal_act.abilities[0].contains("second_targets = Some("),
        "{}",
        modal_act.abilities[0]
    );

    // Targeted & mass graveyard Flashback + follow-up PumpAll + Protection pump.
    let snap = read(
        "Name:SnapMage\nManaCost:1 U\nTypes:Creature\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigFB\n\
         SVar:TrigFB:DB$ Pump | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TgtZone$ Graveyard | PumpZone$ Graveyard | KW$ Flashback",
    );
    assert!(
        snap.abilities[0].contains("Effect::GrantFlashback")
            && snap.abilities[0].contains("TargetSpec::CardInGraveyard("),
        "{}",
        snap.abilities[0]
    );
    let past_flames = read(
        "Name:FlameRecall\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ PumpAll | ValidCards$ Instant.YouCtrl,Sorcery.YouCtrl | PumpZone$ Graveyard | KW$ Flashback",
    );
    assert!(
        past_flames.abilities[0].contains("Modifier::GrantsFlashback")
            && past_flames.statics.contains("Filter::OwnedByYou")
            && past_flames
                .statics
                .contains("Filter::InZone(ZoneRef::Graveyard)"),
        "{}\n{}",
        past_flames.statics,
        past_flames.abilities[0]
    );
    let barge_like = read(
        "Name:ChargePush\nManaCost:R\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.attacking | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBAll\n\
         SVar:DBAll:DB$ PumpAll | ValidCards$ Creature.attacking | KW$ Trample",
    );
    assert!(
        barge_like.abilities[0].contains("Effect::PumpFilter {"),
        "{}",
        barge_like.abilities[0]
    );
    let prot_pump = read(
        "Name:VeilStride\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | KW$ Protection:Creature.OppCtrl:creatures your opponents control | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ ParentTarget",
    );
    assert!(
        prot_pump.abilities[0].contains("Modifier::ProtectionFrom(&Filter::OPPONENT_CREATURE)")
            && prot_pump.abilities[0].contains("Effect::UntapTarget"),
        "{}",
        prot_pump.abilities[0]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave14c_counter_types_and_wave6c_effect_modifiers() {
    let renew = read(
        "Name:RenewChamp\nManaCost:2 G\nTypes:Creature\nPT:3/3\n\
         A:AB$ PutCounter | Cost$ 1 G ExileFromGrave<1/CARDNAME> | ActivationZone$ Graveyard | ValidTgts$ Creature | CounterTypes$ P1P1,P1P1,Lifelink | CounterNum$ 1 | SorcerySpeed$ True",
    );
    assert!(
        renew.abilities[0]
            .contains("Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }")
            && renew.abilities[0].contains(
                "Effect::AddCounter { kind: CounterKind::Lifelink, amount: Amount::Fixed(1) }"
            ),
        "{}",
        renew.abilities[0]
    );
    assert!(refused(
        "Name:BadListCounter\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature | CounterTypes$ P1P1,ChosenFromList | TypeList$ Flying,Lifelink"
    ));
    assert!(refused(
        "Name:UnmodeledKindCounter\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature | CounterTypes$ P1P1,Trample | CounterNum$ 1"
    ));

    let max_block = read(
        "Name:SingleBlockerSpell\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ STMax\n\
         SVar:STMax:Mode$ MinMaxBlocker | ValidCard$ Card.IsRemembered | Max$ 1",
    );
    assert!(
        max_block.abilities[0].contains("Modifier::MaxBlockers(1)"),
        "{}",
        max_block.abilities[0]
    );

    let noncombat_shield = read(
        "Name:NoncombatShield\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ RPrev\n\
         SVar:RPrev:Event$ DamageDone | ValidTarget$ You,Creature.YouCtrl | IsCombat$ False | Prevent$ True",
    );
    assert!(
        noncombat_shield.abilities[0].contains(
            "Modifier::PreventNoncombatDamageToPlayers { who: PlayerRel::You, from: &Filter::Any }"
        ) && noncombat_shield.abilities[0]
            .contains("Modifier::PreventNoncombatDamageFrom(&Filter::Any)"),
        "{}",
        noncombat_shield.abilities[0]
    );

    let doubler = read(
        "Name:FlameQuest\nManaCost:R\nTypes:Enchantment\n\
         A:AB$ Effect | Cost$ Sac<1/CARDNAME> | ReplacementEffects$ DmgEv\n\
         SVar:DmgEv:Event$ DamageDone | ValidSource$ Card.YouCtrl,Emblem.YouCtrl | ValidTarget$ Permanent,Player | ReplaceWith$ Dmg2\n\
         SVar:Dmg2:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Times.2",
    );
    assert!(
        doubler.abilities[0].contains(
            "Modifier::MultiplyDamageFrom { sources: &Filter::ControlledByYou, factor: 2, combat_only: false, noncombat_only: false, recipients: DamageRecipients::Any }"
        ),
        "{}",
        doubler.abilities[0]
    );

    let adder = read(
        "Name:HeatBoost\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ DmgEv\n\
         SVar:DmgEv:Event$ DamageDone | ValidSource$ Card.YouCtrl | ValidTarget$ Player.Opponent,Permanent.OppCtrl | IsCombat$ False | ReplaceWith$ DmgPlus\n\
         SVar:DmgPlus:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Plus.2",
    );
    assert!(
        adder.abilities[0].contains(
            "Modifier::AddDamageFrom { sources: &Filter::ControlledByYou, extra: 2, combat_only: false, noncombat_only: true, recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent) }"
        ),
        "{}",
        adder.abilities[0]
    );

    // MultiplyDamageFrom does not bind ValidSource$ Creature.IsRemembered via ObjectIs.
    assert!(refused(
        "Name:BadSourceRememberedMul\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | ReplacementEffects$ DmgEv\n\
         SVar:DmgEv:Event$ DamageDone | ValidSource$ Creature.IsRemembered | ValidTarget$ Player.Opponent | IsCombat$ True | ReplaceWith$ Dmg3\n\
         SVar:Dmg3:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ ReplaceCount$DamageAmount/Times.3"
    ));
}
