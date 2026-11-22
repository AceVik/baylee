use super::*;

fn assert_alt_conditions(cases: &[(&str, &str)]) {
    for (script, expected_cond) in cases {
        let body = read(script);
        assert_eq!(body.alternative_costs.len(), 1, "{script}");
        assert!(
            body.alternative_costs[0].ends_with(&format!("condition: {expected_cond} }}")),
            "got `{}` for:\n{script}",
            body.alternative_costs[0]
        );
    }
}

fn assert_refusals(cases: &[(&str, &str)]) {
    for (script, why) in cases {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(*why),
            "{script}"
        );
    }
}

#[test]
fn self_spell_alternative_costs_read_mana_free_life_and_pitch() {
    let free = read(
        "Name:X\nTypes:Creature\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0",
    );
    assert!(!free.is_empty());
    assert_eq!(
        free.alternative_costs,
        ["AlternativeCost { cost: Cost::FREE, condition: AltCondition::Always }"]
    );

    let explicit_zero = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell | ValidCard$ Card.Self | \
         ValidPlayer$ You | EffectZone$ All | Cost$ 0 | \
         Description$ You may pay {0} rather than pay this spell's mana cost.\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        explicit_zero.alternative_costs,
        ["AlternativeCost { cost: cost!(\"{0}\"), condition: AltCondition::Always }"]
    );

    let mana = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 1 U\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        mana.alternative_costs,
        ["AlternativeCost { cost: cost!(\"{1}{U}\"), condition: AltCondition::Always }"]
    );

    let pitch = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
         Cost$ PayLife<1> ExileFromHand<1/Card.Blue+Other>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        pitch.alternative_costs,
        [
            "AlternativeCost { cost: cost!(PayLife(1), ExileFromHand(&COST1)), condition: AltCondition::Always }"
        ]
    );
    assert_eq!(
        pitch.statics,
        "static COST1: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));\n\n"
    );

    let double_pitch = read(
        "Name:X\nTypes:Sorcery\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
         Cost$ ExileFromHand<2/Card.Red+StrictlyOther>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        double_pitch.alternative_costs,
        [
            "AlternativeCost { cost: cost!(ExileFromHand(&COST1), ExileFromHand(&COST1)), condition: AltCondition::Always }"
        ]
    );
}

#[test]
fn self_spell_alternative_costs_read_sacrifice_return_and_tap_parts() {
    let sac_two = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Sac<2/Mountain>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        sac_two.alternative_costs,
        [
            "AlternativeCost { cost: cost!(Sacrifice(&COST1), Sacrifice(&COST1)), condition: AltCondition::Always }"
        ]
    );
    assert_eq!(
        sac_two.statics,
        concat!(
            "static COST1: Filter = Filter::And(&[",
            "Filter::HasSubtype(subtypes::land::MOUNTAIN), ",
            "Filter::ControlledByYou]);\n\n"
        )
    );

    let bounce_two = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Return<2/Island>\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert_eq!(
        bounce_two.alternative_costs,
        [
            "AlternativeCost { cost: cost!(ReturnToHand(&COST1), ReturnToHand(&COST1)), condition: AltCondition::Always }"
        ]
    );

    let tap_group = read(
        "Name:X\nTypes:Creature\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
         Cost$ W tapXType<4/Creature.withFlying>",
    );
    assert_eq!(
        tap_group.alternative_costs,
        [
            "AlternativeCost { cost: cost!(\"{W}\", TapN { filter: &COST1, count: 4 }), condition: AltCondition::Always }"
        ]
    );
    assert_eq!(
        tap_group.statics,
        concat!(
            "static COST1: Filter = Filter::And(&[",
            "Filter::CREATURE, ",
            "Filter::HasKeyword(KeywordSet::FLYING), ",
            "Filter::ControlledByYou]);\n\n"
        )
    );
}

#[test]
fn alternative_cost_turn_life_commander_and_attack_conditions_map_one_to_one() {
    assert_alt_conditions(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ NotPlayerTurn\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::NotYourTurn",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell | ValidCard$ Card.Self | \
             EffectZone$ All | Cost$ 0 | IsPresent$ Card.IsCommander+YouCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::CommanderControlled",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ PlayerTurn\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::YourTurn)",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ Threshold\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::GraveyardCountAtLeast(7))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | LifeTotal$ You | LifeAmount$ LE5\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::LifeAtMost(5))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | LifeTotal$ You | LifeAmount$ GE30\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), 30))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 1 R | CheckSVar$ RaidTest\n\
             SVar:RaidTest:Count$AttackersDeclared\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::AttackedWithAtLeast(1))",
        ),
    ]);
}

#[test]
fn alternative_cost_board_and_graveyard_presence_conditions_map_one_to_one() {
    assert_alt_conditions(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ PayLife<4> | IsPresent$ Forest.YouCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::ControlCount(&Filter::HasSubtype(subtypes::land::FOREST), 1))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Creature.YouCtrl | PresentCompare$ EQ0\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::ControlCountAtMost(&Filter::YOUR_CREATURE, 0))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Island.OppCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::OpponentControlCount(&Filter::And(&[Filter::HasSubtype(subtypes::land::ISLAND), Filter::ControlledByOpponent]), 1))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Creature.OppCtrl | PresentCompare$ EQ0\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::BattlefieldCountAtMost(&Filter::OPPONENT_CREATURE, 0))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Creature | PresentCompare$ GE3\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::BattlefieldCount(&Filter::CREATURE, 3))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Instant.YouOwn,Sorcery.YouOwn | \
             PresentZone$ Graveyard | PresentPlayer$ You | PresentCompare$ GE3\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::INSTANT_OR_SORCERY, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 3))",
        ),
        (
            "Name:X\nTypes:Creature\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell | ValidCard$ Card.Self | \
             EffectZone$ All | Cost$ 0 | CheckSVar$ X | CheckSecondSVar$ Y\n\
             SVar:X:Count$Valid Island.OppCtrl\n\
             SVar:Y:Count$Valid Forest.YouCtrl",
            "AltCondition::If(&Condition::All(&[Condition::OpponentControlCount(&Filter::And(&[Filter::HasSubtype(subtypes::land::ISLAND), Filter::ControlledByOpponent]), 1), Condition::ControlCount(&Filter::HasSubtype(subtypes::land::FOREST), 1)]))",
        ),
    ]);
}

#[test]
fn alternative_cost_refuses_unpayable_parts_and_stays_scoped_off_activated_lines() {
    assert_refusals(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Discard<1/Forest>\n\
             A:SP$ Draw | NumCards$ 1",
            "an `AlternativeCost` cost `Discard<1/Forest>` the cast cannot pay",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 1 W ExileFromGrave<1/Creature>\n\
             A:SP$ Draw | NumCards$ 1",
            "an `AlternativeCost` cost `1 W ExileFromGrave<1/Creature>` the cast cannot pay",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ T\n\
             A:SP$ Draw | NumCards$ 1",
            "an `AlternativeCost` cost `T` the cast cannot pay",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Sac<1/CARDNAME>\n\
             A:SP$ Draw | NumCards$ 1",
            "an `AlternativeCost` cost `Sac<1/CARDNAME>` the cast cannot pay",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Return<1/CARDNAME>\n\
             A:SP$ Draw | NumCards$ 1",
            "an `AlternativeCost` cost `Return<1/CARDNAME>` the cast cannot pay",
        ),
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Draw | Cost$ ExileFromHand<1/Card.Blue> | NumCards$ 1",
            "cost `ExileFromHand`",
        ),
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Draw | Cost$ Return<4/Forest> | NumCards$ 1",
            "a cost naming `4` objects",
        ),
    ]);
}

#[test]
fn alternative_cost_refuses_non_self_and_wrong_zone_statics_by_name() {
    assert_refusals(&[
        (
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ AlternativeCost | EffectZone$ All | Cost$ 0",
            "alternative cost with no `ValidSA$`",
        ),
        (
            "Name:X\nTypes:Creature\n\
             S:Mode$ AlternativeCost | ValidSA$ Activated.Equip | EffectZone$ All | Cost$ 0",
            "alternative cost for `ValidSA$ Activated.Equip`",
        ),
        (
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell | EffectZone$ All | Cost$ 0",
            "alternative cost for other spells",
        ),
        (
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell | ValidCard$ Creature.Goblin | \
             EffectZone$ All | Cost$ 0",
            "alternative cost for `ValidCard$ Creature.Goblin`",
        ),
        (
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | ValidPlayer$ Opponent | \
             EffectZone$ All | Cost$ 0",
            "alternative cost for `ValidPlayer$ Opponent`",
        ),
        (
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | Cost$ 0",
            "alternative cost from the battlefield",
        ),
        (
            "Name:X\nTypes:Creature\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ Graveyard | Cost$ 1 B",
            "alternative cost from `EffectZone$ Graveyard`",
        ),
    ]);
}

#[test]
fn alternative_cost_refuses_unread_conditions_and_unclaimed_keys_by_definition() {
    assert_refusals(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Island.OppCtrl | PresentCompare$ GE2\n\
             A:SP$ Draw | NumCards$ 1",
            "`IsPresent$ Island.OppCtrl` compared `GE2`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Creature.Other+YouCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "`IsPresent$ Creature.Other+YouCtrl`, a count relative to this card",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Creature.YouCtrl | PresentCompare$ EQ1\n\
             A:SP$ Draw | NumCards$ 1",
            "`PresentCompare$ EQ1`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | LifeTotal$ Opponent | LifeAmount$ LE10\n\
             A:SP$ Draw | NumCards$ 1",
            "`LifeTotal$ Opponent`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | LifeTotal$ You | LifeAmount$ EQ1\n\
             A:SP$ Draw | NumCards$ 1",
            "`LifeAmount$ EQ1`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | CheckSVar$ X\n\
             SVar:X:Count$SearchedLibrary.Opponent\n\
             A:SP$ Draw | NumCards$ 1",
            "alternative-cost condition `Count$SearchedLibrary.Opponent`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | CheckSVar$ X | SVarCompare$ EQ0\n\
             SVar:X:Count$YouCastThisGame\n\
             A:SP$ Draw | NumCards$ 1",
            "alternative-cost condition `Count$YouCastThisGame` compared `EQ0`",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | CheckSVar$ Missing\n\
             A:SP$ Draw | NumCards$ 1",
            "`CheckSVar$ Missing` naming no SVar",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 1 R | ManaRestriction$ Artifact.Treasure\n\
             A:SP$ Draw | NumCards$ 1",
            "unclaimed parameter `AlternativeCost.ManaRestriction`",
        ),
    ]);
}

#[test]
fn wave18a_alternative_cost_parts_and_conditions() {
    let dmg_alt = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ DamageYou<3>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        dmg_alt.alternative_costs,
        ["AlternativeCost { cost: cost!(DealDamageToYou(3)), condition: AltCondition::Always }"]
    );

    let forage_alt = read(
        "Name:X\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ Forage\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        forage_alt.alternative_costs,
        ["AlternativeCost { cost: cost!(Forage), condition: AltCondition::Always }"]
    );

    assert_alt_conditions(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ Delirium\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::GraveyardTypesAtLeast(4))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ Metalcraft\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::ControlCount(&Filter::ARTIFACT, 3))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | Condition$ Morbid\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::CreaturesDiedThisTurnAtLeast(1))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | IsPresent$ Card.YouCtrl+IsRingbearer\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::HasRingBearer)",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | CheckSVar$ Count$CommittedCrimeThisTurn.1.0\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::CommittedCrimeThisTurn(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Sorcery\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ 0 | CheckSVar$ X | SVarCompare$ EQ0\n\
             SVar:X:Count$ValidHand Land.YouOwn\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::Not(&Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::LAND, zone: ZoneSel::HandYou }, 1)))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ B | CheckSVar$ X\n\
             SVar:X:Count$LifeOppsLostThisTurn\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::OpponentLostLifeThisTurn)",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | \
             Cost$ U | CheckSVar$ X | SVarCompare$ GE2\n\
             SVar:X:Count$ThisTurnEntered_Battlefield_Creature.YouCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, 2))",
        ),
    ]);
}

#[test]
fn wave19a_alternative_cost_phyrexian_and_two_generic_mana() {
    let body = read(
        "Name:PhyrAlt\nManaCost:3 U\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 2U WP URP\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        body.alternative_costs,
        ["AlternativeCost { cost: cost!(\"{2/U}{W/P}{U/R/P}\"), condition: AltCondition::Always }"]
    );

    for bad_cost in ["2C", "CP", "WWP"] {
        let script = format!(
            "Name:BadAlt\nManaCost:3 U\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ {bad_cost}\n\
             A:SP$ Draw | NumCards$ 1"
        );
        assert!(refused(&script), "must refuse: {bad_cost}");
    }
}

#[test]
fn wave20a_alternative_cost_extra_named_conditions() {
    assert_alt_conditions(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ MaxSpeed\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::MaxSpeed)",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ EnduringStory\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::EnduringStory)",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ FatefulHour\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::LifeAtMost(5))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ Revolt\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::Revolt)",
        ),
    ]);
}

#[test]
fn wave21a_alternative_cost_remove_any_counter_and_wave11c_conditions() {
    let body = read(
        "Name:CounterAlt\nManaCost:3 B\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ RemoveAnyCounter<2/Any/Permanent>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        body.alternative_costs,
        [
            "AlternativeCost { cost: cost!(RemoveAnyCounter { count: 2, kind: None, filter: &Filter::Any }), condition: AltCondition::Always }"
        ]
    );

    assert_alt_conditions(&[
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ Monarch\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::IsMonarch(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ Initiative\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::HasInitiative(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | Condition$ CompletedDungeon\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::CompletedADungeon(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | CheckDefinedPlayer$ You.hasInitiative\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::HasInitiative(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | CheckSVar$ NoM | SVarCompare$ EQ0\n\
             SVar:NoM:PlayerCountPlayers$HasPropertyisMonarch\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::Not(&Condition::IsMonarch(PlayerRel::EachPlayer)))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | CheckSVar$ D | SVarCompare$ GE1\n\
             SVar:D:DungeonsCompleted$Amount\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::CompletedADungeon(PlayerRel::You))",
        ),
        (
            "Name:X\nTypes:Instant\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | IsPresent$ Card.IsCommander+YouOwn+YouCtrl\n\
             A:SP$ Draw | NumCards$ 1",
            "AltCondition::If(&Condition::ControlYourCommander)",
        ),
    ]);

    for bad in [
        // RemoveAnyCounterX is not paid by `paid_as_an_alternative_cost`.
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ RemoveAnyCounter<X/P1P1/Creature>\nSVar:X:Count$xPaid\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ RemoveAnyCounter<1/Any/CARDNAME>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ RemoveAnyCounter<1/Any/Creature.Other>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0 | CheckSVar$ D | SVarCompare$ EQ1\nSVar:D:DungeonsCompleted$Amount\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22a_alternative_cost_sub_counter_non_any() {
    let body = read(
        "Name:SubCounterAlt\nManaCost:3 B\nTypes:Instant\n\
         S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/a creature you control>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        body.alternative_costs,
        [
            "AlternativeCost { cost: cost!(RemoveAnyCounter { count: 1, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }), condition: AltCondition::Always }"
        ]
    );

    for bad in [
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<1/P1P1>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<1/P1P1/CARDNAME>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<1/P1P1/Creature.OppCtrl>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/desc/Exile>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:3 B\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ SubCounter<0/P1P1/Creature.YouCtrl>\nA:SP$ Draw | NumCards$ 1",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason: {bad}"
        );
    }
}
