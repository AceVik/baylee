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
    // Read Ahead sagas stay refused via the unimplemented keyword.
    assert!(refused(
        "Name:Story2\nManaCost:2 W\nTypes:Enchantment Saga\n\
         K:Read ahead\nK:Chapter:2:Ch1,Ch2\n\
         SVar:Ch1:DB$ Draw | NumCards$ 1\nSVar:Ch2:DB$ GainLife | LifeAmount$ 2"
    ));
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
