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

    // 3) Immediate single-target Blink (multi-target blink refused!).
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
    assert!(refused(
        "Name:MultiFlicker\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 2 | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield"
    ));

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

    let wither_pump = read(
        "Name:BlightBlade\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Wither | Duration$ UntilEndOfCombat",
    );
    assert!(
        wither_pump.abilities[0].contains("KeywordSet::WITHER")
            && wither_pump.abilities[0].contains("Duration::UntilEndOfCombat"),
        "{}",
        wither_pump.abilities[0]
    );

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
    assert!(refused(
        "Name:YawgWill\nManaCost:2 B\nTypes:Sorcery\n\
         S:Mode$ Continuous | Affected$ Card.YouCtrl | AffectedZone$ Graveyard | MayPlay$ True"
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
