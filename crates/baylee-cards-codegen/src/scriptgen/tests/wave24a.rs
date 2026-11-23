//! Wave 24a: `Branch`, Wave 23b deferred follow-ups (triggered divided
//! counters, excess-damage chains, same-controller refusal), and
//! `Mill.RememberMilled` / `ChangeZone.RememberChanged` /
//! `ChangeZoneAll.RememberChanged` chains.

use super::{
    SubtypeCatalogs, TokenLookup, parse, read, read_with_tokens, refusal_reason, refused, transcode,
};

fn wave24a_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Elf".into(),
            "Goblin".into(),
            "Human".into(),
            "Phyrexian".into(),
            "Spirit".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into()],
        enchantment: vec!["Aura".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn wave24a_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "w_1_1_spirit_flying",
            "Name:Spirit\nTypes:Creature Spirit\nColors:white\nPT:1/1\nK:Flying",
        ),
        (
            "w_4_4_angel_flying",
            "Name:Angel\nTypes:Creature Angel\nColors:white\nPT:4/4\nK:Flying",
        ),
        (
            "g_1_1_elf_warrior",
            "Name:Elf Warrior\nTypes:Creature Elf Warrior\nColors:green\nPT:1/1",
        ),
        (
            "c_a_clue_sac",
            "Name:Clue Token\nTypes:Artifact Clue\n\
             A:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1",
        ),
    ])
}

fn read_w24(text: &str) -> super::CardBody {
    let parsed = parse(text);
    let cats = wave24a_cats();
    let toks = wave24a_tokens();
    transcode(&parsed, &cats, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, Some(&toks))
        )
    })
}

fn refused_w24(text: &str) -> bool {
    let cats = wave24a_cats();
    let toks = wave24a_tokens();
    transcode(&parse(text), &cats, Some(&toks)).is_none()
}

// -----------------------------------------------------------------------------
// Scope 1: `Branch` (`DB$ Branch`, `SP$ Branch`, `AB$ Branch`)
// -----------------------------------------------------------------------------

#[test]
fn branch_monarch_upkeep_lowers_to_if_condition() {
    let body = read_w24(
        "Name:Court of Grace\nManaCost:2 W W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMonarch\n\
         SVar:TrigMonarch:DB$ BecomeMonarch\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | BranchConditionSVar$ X | TrueSubAbility$ DBAngel | FalseSubAbility$ DBSpirit\n\
         SVar:DBSpirit:DB$ Token | TokenScript$ w_1_1_spirit_flying | TokenOwner$ You\n\
         SVar:DBAngel:DB$ Token | TokenScript$ w_4_4_angel_flying | TokenOwner$ You\n\
         SVar:X:Count$Monarch.1.0",
    );
    let trig = &body.abilities[1];
    assert!(
        trig.contains("Effect::IfCondition { condition: Condition::IsMonarch(PlayerRel::You), then: &[Effect::CreateToken"),
        "{trig}"
    );
    assert!(trig.contains("otherwise: &[Effect::CreateToken"), "{trig}");
}

#[test]
fn branch_stack_spell_target_claims_target_type() {
    let body = read(
        "Name:Anticognition\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X | TargetType$ Spell | TgtZone$ Stack | ValidTgts$ Creature,Planeswalker | BranchConditionSVarCompare$ GE8 | TrueSubAbility$ CounterScry | FalseSubAbility$ CounterUnless\n\
         SVar:CounterUnless:DB$ Counter | Defined$ Targeted | UnlessCost$ 2\n\
         SVar:CounterScry:DB$ Counter | Defined$ Targeted | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 2\n\
         SVar:X:PlayerCountOpponents$HighestCardsInGraveyard",
    );
    let spell = &body.abilities[0];
    assert!(spell.contains("Effect::IfCondition"), "{spell}");
    assert!(spell.contains("Effect::CounterTargetSpell"), "{spell}");
    assert!(spell.contains("Effect::scry(2)"), "{spell}");
    assert!(spell.contains("TargetSpec::Spell("), "{spell}");
}

#[test]
fn branch_true_only_with_inline_card_counters() {
    let body = read(
        "Name:Cursed Recording\nManaCost:2 R R\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ TIME | CounterNum$ 1 | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ Count$CardCounters.TIME | BranchConditionSVarCompare$ GE7 | TrueSubAbility$ DBRemoveCounter\n\
         SVar:DBRemoveCounter:DB$ RemoveCounter | CounterType$ TIME | CounterNum$ All | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | NumDmg$ 20 | Defined$ You",
    );
    let trig = &body.abilities[0];
    assert!(
        trig.contains("Condition::AmountAtLeast(Amount::CountersOnSource(CounterKind::Time), 7)"),
        "{trig}"
    );
    assert!(trig.contains("otherwise: &[]"), "{trig}");
}

#[test]
fn branch_up_to_one_targeted_valid_and_zero_target_safety_refusal() {
    let body = read(
        "Name:Agrus Kos, Spirit of Justice\nManaCost:2 R W\nTypes:Legendary Creature\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | BranchConditionSVar$ X | BranchConditionSVarCompare$ EQ1 | TrueSubAbility$ ExileIt | FalseSubAbility$ SuspectIt\n\
         SVar:ExileIt:DB$ ChangeZone | Defined$ Targeted | Origin$ Battlefield | Destination$ Exile\n\
         SVar:SuspectIt:DB$ AlterAttribute | Defined$ Targeted | Attributes$ Suspected\n\
         SVar:X:Targeted$Valid Card.IsSuspected",
    );
    let trig = &body.abilities[0];
    assert!(
        trig.contains("TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))"),
        "{trig}"
    );
    assert!(trig.contains("Condition::ObjectMatches"), "{trig}");
    assert!(trig.contains("Effect::exile("), "{trig}");
    assert!(trig.contains("Effect::Suspect {"), "{trig}");

    // Refusal: `TargetMin$ 0` with a non-target side effect in `FalseSubAbility$`
    // would wrongly execute when 0 targets are chosen.
    assert!(refused(
        "Name:Bad Zero Target Branch\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | BranchConditionSVar$ X | BranchConditionSVarCompare$ EQ1 | TrueSubAbility$ ExileIt | FalseSubAbility$ DrawOne\n\
         SVar:ExileIt:DB$ ChangeZone | Defined$ Targeted | Origin$ Battlefield | Destination$ Exile\n\
         SVar:DrawOne:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Targeted$Valid Card.IsSuspected",
    ));
}

#[test]
fn branch_triggered_card_valid_and_targeted_counters_and_die_roll() {
    let attacking = read(
        "Name:Garna Sample\nManaCost:1 B R\nTypes:Creature\nPT:2/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl+Other | TriggerZones$ Battlefield | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ EQ1 | TrueSubAbility$ DBDraw | FalseSubAbility$ DBDmg\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ Player.Opponent | NumDmg$ 1\n\
         SVar:X:TriggeredCard$Valid Creature.attacking",
    );
    assert!(
        attacking.abilities[0].contains("what: &TargetSpec::EventObject"),
        "{}",
        attacking.abilities[0]
    );

    let counters = read(
        "Name:Target Counter Branch\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ DBExile | FalseSubAbility$ DBTap\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Targeted | Origin$ Battlefield | Destination$ Exile\n\
         SVar:DBTap:DB$ Tap | Defined$ Targeted\n\
         SVar:X:Targeted$CardCounters.P1P1",
    );
    assert!(
        counters.abilities[0].contains("Condition::ObjectMatches"),
        "{}",
        counters.abilities[0]
    );
    assert!(
        counters
            .statics
            .contains("Filter::CountersAtLeast(CounterKind::P1P1, 1)"),
        "{}",
        counters.statics
    );

    let die = read(
        "Name:Die Branch\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ RolledDie | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE4 | TrueSubAbility$ DBDraw | FalseSubAbility$ DBGain\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:X:TriggerCount$Result",
    );
    assert!(
        die.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::EventAmount, op: AmountCmp::AtLeast, right: &Amount::Fixed(4) }"
        ),
        "{}",
        die.abilities[0]
    );
}

#[test]
fn branch_refuses_unsupported_svars_alchemy_and_malformed_arms() {
    // Unsupported Branch SVars
    for svar in [
        "TargetedController$Counters.Poison",
        "Count$TotalManaSpent Treasure",
        "Count$ManaColorsPaid",
        "Count$Foretold.1.0",
        "TargetedPlayer$LifeLostThisTurn",
    ] {
        let script = format!(
            "Name:Bad Branch\nManaCost:1 U\nTypes:Instant\n\
             A:SP$ Branch | ValidTgts$ Creature | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ DBDraw\n\
             SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
             SVar:X:{svar}"
        );
        let parsed = parse(&script);
        assert!(refused(&script), "must refuse Branch SVar `{svar}`");
        assert!(
            refusal_reason(&parsed, &super::cats(), None).is_some(),
            "must record a refusal reason for `{svar}`"
        );
    }

    // Alchemy Branch (Seek / MakeCard / Perpetual) must stay refused.
    assert!(refused(
        "Name:Alchemy Branch\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X | TrueSubAbility$ DBSeek | FalseSubAbility$ DBDraw\n\
         SVar:DBSeek:DB$ Seek | Type$ Card\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$Monarch.1.0",
    ));
    assert!(refused(
        "Name:Empty Branch\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Branch | BranchConditionSVar$ X\n\
         SVar:X:Count$Monarch.1.0",
    ));
}

// -----------------------------------------------------------------------------
// Scope 2: Wave 23b deferred follow-ups (Divided counters, ExcessSVar, SameCtrl)
// -----------------------------------------------------------------------------

#[test]
fn triggered_divided_counters_fixed_continuation_and_counters_on_source() {
    let gearhulk = read(
        "Name:Verdurous Gearhulk\nManaCost:3 G G\nTypes:Artifact Creature\nPT:4/4\nK:Trample\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 4 | TargetMin$ 0 | TargetMax$ 4 | DividedAsYouChoose$ 4",
    );
    let trig = &gearhulk.abilities[0];
    assert!(
        trig.contains(
            "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(4) }"
        ),
        "{trig}"
    );
    assert!(
        trig.contains("TargetReq::up_to(TargetSpec::Object(&Filter::YOUR_CREATURE), 4)"),
        "{trig}"
    );

    let garenbrig = read(
        "Name:Court of Garenbrig\nManaCost:1 G G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMonarch\n\
         SVar:TrigMonarch:DB$ BecomeMonarch\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 2 | TargetMin$ 0 | TargetMax$ 2 | DividedAsYouChoose$ 2 | SubAbility$ TrigDoubleCounters\n\
         SVar:TrigDoubleCounters:DB$ MultiplyCounter | Defined$ Valid Creature.YouCtrl | CounterType$ P1P1 | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1\n\
         SVar:X:Count$Monarch.1.0",
    );
    let upkeep = &garenbrig.abilities[1];
    assert!(
        upkeep.contains(
            "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"
        ),
        "{upkeep}"
    );
    assert!(
        upkeep.contains("Condition::IsMonarch(PlayerRel::You)"),
        "{upkeep}"
    );
    assert!(upkeep.contains("Effect::DoubleCountersFilter"), "{upkeep}");

    let beastcaller = read(
        "Name:Quirion Beastcaller\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigCounterDist\n\
         SVar:TrigCounterDist:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X | TargetMin$ 0 | TargetMax$ X\n\
         SVar:X:TriggeredCard$CardCounters.P1P1",
    );
    let dies = &beastcaller.abilities[0];
    assert!(
        dies.contains(
            "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::CountersOnSource(CounterKind::P1P1) }"
        ),
        "{dies}"
    );
    assert!(
        dies.contains("TargetReq::up_to(TargetSpec::Object(&Filter::YOUR_CREATURE), 255)"),
        "{dies}"
    );
}

#[test]
fn triggered_divided_counters_refusals() {
    // Optional triggered divided counters (Jugan, the Rising Star):
    // `MayDo` around `PutCountersDivided` is forbidden by engine division shapes.
    assert!(refused(
        "Name:Jugan, the Rising Star\nManaCost:3 G G G\nTypes:Legendary Creature\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigPutCounter | OptionalDecider$ TriggeredCardController\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 5 | TargetMin$ 0 | TargetMax$ 5 | DividedAsYouChoose$ 5\n\
         Oracle:When Jugan, the Rising Star dies, you may distribute five +1/+1 counters among any number of target creatures.",
    ));
    // Mismatched total, player counters, unsupported bounds, zero total.
    assert!(refused(
        "Name:Mismatched\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 4 | TargetMin$ 0 | TargetMax$ 4 | DividedAsYouChoose$ 3",
    ));
    assert!(refused(
        "Name:Player Counter\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ PutCounter | ValidTgts$ Creature | CounterType$ POISON | CounterNum$ 2 | TargetMin$ 0 | TargetMax$ 2 | DividedAsYouChoose$ 2",
    ));
    assert!(refused(
        "Name:Bad Bounds\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 2 | TargetMin$ 1 | TargetMax$ 2 | DividedAsYouChoose$ 2",
    ));
}

#[test]
fn excess_damage_bite_fight_and_single_chains_plus_refusals() {
    let claws = read(
        "Name:Contest of Claws\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Creature | TargetUnique$ True | NumDmg$ X | DamageSource$ ParentTarget | ExcessSVar$ Excess | SubAbility$ DBDiscover\n\
         SVar:DBDiscover:DB$ Discover | Num$ Excess | ConditionCheckSVar$ Excess\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        claws.abilities[0].contains("Effect::DamageEqualToPower"),
        "{}",
        claws.abilities[0]
    );
    assert!(
        claws.abilities[0].contains(
            "Effect::DiscoverAmount { mana_value: Amount::ThisWay(ThisWay::ExcessDamage) }"
        ),
        "{}",
        claws.abilities[0]
    );

    let slice = read_w24(
        "Name:Windswift Slice\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ SoulsDamage\n\
         SVar:SoulsDamage:DB$ DealDamage | ValidTgts$ Creature.YouDontCtrl | ExcessSVar$ Excess | NumDmg$ X | DamageSource$ ParentTarget | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ g_1_1_elf_warrior | TokenAmount$ Excess\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        slice.abilities[0].contains("Amount::ThisWay(ThisWay::ExcessDamage)"),
        "{}",
        slice.abilities[0]
    );

    let fight_excess = read(
        "Name:Fight Excess Sample\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | ValidTgts$ Creature.OppCtrl | Defined$ ParentTarget | ExcessSVar$ Excess | ExcessSVarCondition$ Creature.targetedBy+OppCtrl | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ Excess",
    );
    assert!(
        fight_excess.abilities[0].contains("Effect::Fight"),
        "{}",
        fight_excess.abilities[0]
    );
    assert!(
        fight_excess.abilities[0].contains(
            "Effect::DrawCards { amount: Amount::ThisWayBy { what: ThisWay::ExcessDamage, who: PlayerRel::SecondTargetPlayerOrController } }"
        ),
        "{}",
        fight_excess.abilities[0]
    );

    // Refusals: reflexive Fight ExcessSVar (Rhino's Rampage), Alchemy Perpetual
    // / Conjure, and Archaic's Agony (must record a refusal reason).
    assert!(refused(
        "Name:Rhino's Rampage\nManaCost:RG\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | ValidTgts$ Creature.OppCtrl | Defined$ ParentTarget | ExcessSVar$ Excess | ExcessSVarCondition$ Creature.targetedBy+OppCtrl | SubAbility$ DBTrig\n\
         SVar:DBTrig:DB$ ImmediateTrigger | ConditionCheckSVar$ Excess | Execute$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Artifact.cmcLE3",
    ));
    assert!(refused(
        "Name:Ravenous Pursuit\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | ValidTgts$ Creature.OppCtrl | Defined$ ParentTarget | ExcessSVar$ Excess | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ParentTarget | NumAtt$ +Excess | Duration$ Perpetual",
    ));
    let agony = "Name:Archaic's Agony\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | ExcessSVar$ Excess | ExcessSVarCondition$ Card.targetedBy | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | Defined$ You | DigNum$ Excess | ChangeNum$ All | DestinationZone$ Exile\n\
         SVar:X:Count$Converge";
    assert!(refused(agony));
    let trig_excess_refusal = "Name:Unsupported Trigger Excess\nManaCost:3 R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | ExcessSVar$ Excess | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | Defined$ You | DigNum$ Excess | ChangeNum$ All | DestinationZone$ Exile";
    assert!(refused(trig_excess_refusal));
    assert!(
        refusal_reason(&parse(trig_excess_refusal), &super::cats(), None).is_some(),
        "triggered DealDamage ExcessSVar refusal in chains.rs must record a reason"
    );
}

#[test]
fn triggered_targets_with_same_controller_emits_constraint() {
    // Triggered `TargetsWithSameController$ True` lowers through
    // `Chain.target_constraint` (`mod.rs`/`chains.rs`/`activated.rs`) into
    // `TargetConstraint::SameController`.
    let zombie = read(
        "Name:Soul-Shackled Zombie\nManaCost:2 B\nTypes:Creature\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 2 | TargetsWithSameController$ True",
    );
    assert!(
        zombie.abilities[0].contains(
            "TargetReq::up_to(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer), 2)\
             .with_constraint(TargetConstraint::SameController)"
        ),
        "{:?}",
        zombie.abilities
    );
}

// -----------------------------------------------------------------------------
// Scope 3: `Mill.RememberMilled`, `ChangeZone.RememberChanged`, `ChangeZoneAll`
// -----------------------------------------------------------------------------

#[test]
fn mill_remembered_branch_and_predict_named_card() {
    let lorehold = read(
        "Name:Lorehold Excavation\nManaCost:R W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 1 | RememberMilled$ True | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE1 | TrueSubAbility$ GainLife | FalseSubAbility$ DealDamage | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Land\n\
         SVar:GainLife:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:DealDamage:DB$ DamageAll | ValidPlayers$ Player.Opponent | NumDmg$ 1",
    );
    let trig = &lorehold.abilities[0];
    assert!(
        trig.contains("Amount::ThisWay(ThisWay::Milled(&Filter::LAND))"),
        "{trig}"
    );
    assert!(trig.contains("Effect::gain_life(1)"), "{trig}");
    assert!(
        trig.contains("Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::Opponent) }"),
        "{trig}"
    );

    let predict = read(
        "Name:Predict\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ NameCard | Defined$ You | SubAbility$ DBMill\n\
         SVar:DBMill:DB$ Mill | ValidTgts$ Player | NumCards$ 1 | RememberMilled$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X\n\
         SVar:X:Count$Compare Y GE1.2.1\n\
         SVar:Y:Remembered$Valid Card.NamedCard",
    );
    let spell = &predict.abilities[0];
    assert!(spell.contains("Effect::ChooseCardName"), "{spell}");
    assert!(
        spell.contains("Amount::ThisWay(ThisWay::Milled(&Filter::NamedBySource))"),
        "{spell}"
    );
    assert!(spell.contains("then: &[Effect::draw(2)]"), "{spell}");
    assert!(spell.contains("otherwise: &[Effect::draw(1)]"), "{spell}");
}

#[test]
fn hand_to_bf_attach_if_equipment_on_activated_ability() {
    let body = read_w24(
        "Name:Armor Deployer\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ 2 T | Origin$ Hand | Destination$ Battlefield | ChangeType$ Artifact.YouCtrl | Optional$ You | Hidden$ True | RememberChanged$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Remembered | Defined$ Self | ConditionDefined$ Remembered | ConditionPresent$ Equipment | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        body.abilities[0].contains("Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn exile_gy_cast_copy_and_copy_permanent_plus_refusals() {
    let shiko = read(
        "Name:Shiko, Paragon of the Way\nManaCost:2 U R W\nTypes:Legendary Creature\nPT:4/5\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Destination$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.YouOwn+nonLand+cmcLE3 | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidZone$ Exile | Controller$ You | CopyCard$ True | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        shiko.abilities[0].contains("Effect::ExileAndCastCopies { graveyard: None }"),
        "{}",
        shiko.abilities[0]
    );

    // Mizzix's Mastery has Overload and must be refused cleanly with a reason.
    let mizzix = "Name:Mizzix's Mastery\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Instant.YouOwn,Sorcery.YouOwn | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidZone$ Exile | Controller$ You | CopyCard$ True | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | Amount$ All | SubAbility$ ExileMe\n\
         SVar:ExileMe:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         K:Overload:5 R R R";
    assert!(refused(mizzix));
    assert!(refusal_reason(&parse(mizzix), &super::cats(), None).is_some());

    let anikthea = read_w24(
        "Name:Anikthea, Hand of Erebos\nManaCost:2 W B G\nTypes:Legendary Enchantment Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Enchantment.nonAura+YouCtrl | Origin$ Graveyard | TargetMin$ 0 | TargetMax$ 1 | Destination$ Exile | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 3 | SetToughness$ 3 | AddTypes$ Creature & Zombie | SetColor$ Black | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let trig = &anikthea.abilities[0];
    assert!(trig.contains("Effect::exile("), "{trig}");
    assert!(
        trig.contains("ThisWay::ExiledFromGraveyard(&Filter::Any)"),
        "{trig}"
    );
    assert!(trig.contains("Effect::CreateTokenCopy"), "{trig}");
    assert!(trig.contains("CopyMod::SetPT(3, 3)"), "{trig}");

    // Battlefield -> Exile -> CopyPermanent must stay refused (would copy
    // printed values in exile rather than battlefield LKI).
    assert!(refused(
        "Name:Dedicated Dollmaker\nManaCost:1 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ T | ValidTgts$ Creature.Other+YouCtrl | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 1 | SetToughness$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ));
}

#[test]
fn reanimate_attach_pump_copy_explore_and_mass_reanimate() {
    let formalwear = read_w24(
        "Name:Pre-War Formalwear\nManaCost:2 W\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.cmcLE3+YouOwn | RememberChanged$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        formalwear.abilities[0].contains("Effect::GraveyardToBattlefield"),
        "{}",
        formalwear.abilities[0]
    );
    assert!(
        formalwear.abilities[0].contains("Effect::AttachSelf"),
        "{}",
        formalwear.abilities[0]
    );

    let coiling = read(
        "Name:Coiling Rebirth\nManaCost:3 B B\nTypes:Sorcery\n\
         K:Gift\n\
         SVar:GiftAbility:DB$ Draw | Defined$ Promised | GiftDescription$ a card\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl | SubAbility$ DBCopy | RememberChanged$ True\n\
         SVar:DBCopy:DB$ CopyPermanent | ConditionCheckSVar$ X | ConditionDefined$ Remembered | ConditionPresent$ Card.nonLegendary | Defined$ Remembered | SetPower$ 1 | SetToughness$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$PromisedGift.1.0",
    );
    let spell = &coiling.abilities[0];
    assert!(spell.contains("Effect::GraveyardToBattlefield"), "{spell}");
    assert!(spell.contains("Condition::GiftWasPromised"), "{spell}");
    assert!(spell.contains("Effect::CreateTokenCopy"), "{spell}");
    assert!(
        coiling
            .statics
            .contains("Filter::InZone(ZoneRef::Battlefield)"),
        "{}",
        coiling.statics
    );

    let defossilize = read(
        "Name:Defossilize\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | RememberChanged$ True | Destination$ Battlefield | ValidTgts$ Creature.YouOwn | SubAbility$ DBExplore\n\
         SVar:DBExplore:DB$ Explore | Num$ 2 | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let def_spell = &defossilize.abilities[0];
    assert!(
        def_spell.contains("Effect::GraveyardToBattlefield"),
        "{def_spell}"
    );
    assert_eq!(
        def_spell.matches("Effect::Explore").count(),
        2,
        "{def_spell}"
    );
    assert!(
        defossilize
            .statics
            .contains("Filter::InZone(ZoneRef::Battlefield)"),
        "{}",
        defossilize.statics
    );

    let wake = read(
        "Name:Wake the Past\nManaCost:5 R W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Artifact.YouOwn | RememberChanged$ True | Origin$ Graveyard | Destination$ Battlefield | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Card.IsRemembered | KW$ Haste | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        wake.abilities[0].contains("Effect::MoveAndModify { subject: MoveSubject::AllInZone"),
        "{}",
        wake.abilities[0]
    );
    assert!(
        wake.abilities[0].contains("Modifier::AddKeyword(KeywordSet::HASTE)"),
        "{}",
        wake.abilities[0]
    );
}

#[test]
fn sunfall_disorder_path_to_festival_and_remembered_followups() {
    let sunfall = read(
        "Name:Sunfall\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBIncubate\n\
         SVar:DBIncubate:DB$ Incubate | Amount$ X\n\
         SVar:X:Remembered$Amount",
    );
    let sun = &sunfall.abilities[0];
    assert!(sun.contains("Effect::ExileAll"), "{sun}");
    assert!(
        sun.contains(
            "Effect::Incubate { who: PlayerRel::You, count: Amount::ThisWay(ThisWay::ExiledFromBattlefield(&Filter::Any)) }"
        ),
        "{sun}"
    );

    let disorder = read_with_tokens(
        "Name:Disorder in the Court\nManaCost:X W U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | TargetMin$ X | TargetMax$ X | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBInvestigate\n\
         SVar:DBInvestigate:DB$ Investigate | Num$ X | SubAbility$ DBDelTrig\n\
         SVar:DBDelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ TrigReturn | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Exile | Destination$ Battlefield | Defined$ DelayTriggerRememberedLKI | Tapped$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        disorder.abilities[0].contains("Effect::ExileAndReturnTappedAtEndStep"),
        "{}",
        disorder.abilities[0]
    );
    assert!(
        disorder.abilities[0]
            .contains("Effect::CreateTokenN { token: &generated_tokens::CLUE, amount: Amount::X }"),
        "{}",
        disorder.abilities[0]
    );

    let festival = read(
        "Name:Path to the Festival\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | RememberChanged$ True | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ConditionCheckSVar$ X | ConditionSVarCompare$ GE3 | ScryNum$ 1\n\
         SVar:X:PlayerCountPropertyYou$DomainPlayer\n\
         K:Flashback:4 G",
    );
    assert!(
        festival.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::BasicLandTypesAmong(&Filter::YOUR_LAND), 3)"
        ),
        "{}",
        festival.abilities[0]
    );

    let appraiser = read(
        "Name:Corpse Appraiser\nManaCost:U B R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBDig | RememberChanged$ True\n\
         SVar:DBDig:DB$ Dig | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionZone$ Exile | DigNum$ 3 | DestinationZone2$ Graveyard | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        appraiser.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)"),
        "{}",
        appraiser.abilities[0]
    );

    let locust = read(
        "Name:Carrion Locust\nManaCost:2 B\nTypes:Creature\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.OppOwn | RememberChanged$ True | SubAbility$ DBDrain\n\
         SVar:DBDrain:DB$ LoseLife | ConditionDefined$ Remembered | ConditionPresent$ Creature | Defined$ TargetedOwner | LifeAmount$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        locust.abilities[0].contains("target: PlayerRel::ControllerOfTarget"),
        "{}",
        locust.abilities[0]
    );

    let elk = read_w24(
        "Name:Mirkwood Elk\nManaCost:5 G\nTypes:Creature\nPT:6/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Elf.YouOwn | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardPower",
    );
    assert!(
        elk.abilities[0].contains("Effect::GainLife { amount: Amount::TargetPower }"),
        "{}",
        elk.abilities[0]
    );

    let vision = read(
        "Name:Volcanic Vision\nManaCost:5 R R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | RememberChanged$ True | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DamageAll | NumDmg$ X | ValidCards$ Creature.OppCtrl | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Stack | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        vision.abilities[0].contains("amount: Amount::TargetCmc"),
        "{}",
        vision.abilities[0]
    );
}

#[test]
fn non_paper_and_unsupported_remember_changed_chains_stay_refused() {
    // Planar Guide: ChangeZoneAll -> end-step return has no simultaneous sweep
    // return effect in DSL (EachExiledThisWay would fire N separate triggers).
    assert!(refused(
        "Name:Planar Guide\nManaCost:W\nTypes:Creature\nPT:1/1\n\
         A:AB$ ChangeZoneAll | Cost$ 3 W Exile<1/CARDNAME> | ChangeType$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DelTrig\n\
         SVar:DelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ TrigReturn | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Exile | Destination$ Battlefield\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ));

    // Act of Authority: Wave 27c lowers this via CaptureAffected + GainControlEx(ThisObject, RememberedController).
    let act = read(
        "Name:Act of Authority\nManaCost:1 W W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigExile | TriggerZones$ Battlefield | OptionalDecider$ You\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Artifact,Enchantment | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBGainControl\n\
         SVar:DBGainControl:DB$ GainControl | Defined$ Self | NewController$ RememberedController | ConditionCheckSVar$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        act.abilities[0].contains("Effect::CaptureAffected")
            && act.abilities[0].contains("target: TargetSpec::ThisObject")
            && act.abilities[0].contains("to: PlayerRel::RememberedController"),
        "{}",
        act.abilities[0]
    );

    // Riders of the Mark: Wave 27c lowers this via CaptureAffected + Amount::RememberedToughness.
    let riders = read_w24(
        "Name:Riders of the Mark\nManaCost:6 R\nTypes:Creature\nPT:7/4\n\
         T:Mode$ Phase | TriggerZones$ Battlefield | Phase$ End of Turn | ValidPlayer$ You | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Defined$ Self | Origin$ Battlefield | Destination$ Hand | RememberChanged$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ w_1_1_spirit_flying | TokenOwner$ You | TokenAmount$ Y | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:RememberedLKI$CardToughness",
    );
    assert!(
        riders.abilities[0].contains("Effect::CaptureAffected")
            && riders.abilities[0].contains("Amount::RememberedToughness"),
        "{}",
        riders.abilities[0]
    );

    // Alchemy Perpetual / Planechase must stay refused.
    assert!(refused_w24(
        "Name:Divine Purge\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | RememberChanged$ True | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | Duration$ Perpetual | Defined$ Remembered | StaticAbilities$ RaiseCost\n\
         SVar:RaiseCost:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2",
    ));
    assert!(refused(
        "Name:Planechase Sample\nTypes:Plane\n\
         T:Mode$ PlaneswalkedTo | TriggerZones$ Command | Execute$ TrigBranch\n\
         SVar:TrigBranch:DB$ Branch | BranchConditionSVar$ X | TrueSubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$Monarch.1.0",
    ));
}
