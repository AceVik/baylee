//! Unit tests for reading Kicker (`K:Kicker:`), kicked branches, kicked
//! targets, kicked filters, kicked ETB counters, and multikicker multiples.

use super::*;

#[test]
fn single_mana_kicker_reads_onto_face_and_conditional_subability() {
    let body = read(
        "Name:X\nManaCost:2 W\nTypes:Instant\n\
         K:Kicker:1 B\n\
         A:SP$ Destroy | ValidTgts$ Artifact | SubAbility$ Extra\n\
         SVar:Extra:DB$ Draw | NumCards$ 2 | Condition$ Kicked | ConditionDescription$ If kicked,",
    );
    assert_eq!(body.additional_costs, ["cost!(\"{1}{B}\")"]);
    assert!(body.kicked_targets.is_none());
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT)), \
             Effect::IfKicked { then: &[Effect::draw(2)], otherwise: &[] }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))))"
        ]
    );
}

#[test]
fn kicker_supports_non_mana_and_variable_x_while_refusing_invalid_forms() {
    let dual_nonmana =
        read("Name:X\nTypes:Instant\nK:Kicker:W:PayLife<3>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(
        dual_nonmana.additional_costs,
        ["cost!(\"{W}\")", "cost!(PayLife(3))"]
    );

    let sac_land = read("Name:X\nTypes:Instant\nK:Kicker:Sac<1/Land>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(
        sac_land.additional_costs,
        ["cost!(Sacrifice(&Filter::YOUR_LAND))"]
    );

    let pay_life = read("Name:X\nTypes:Instant\nK:Kicker:PayLife<3>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(pay_life.additional_costs, ["cost!(PayLife(3))"]);

    let mixed = read("Name:X\nTypes:Instant\nK:Kicker:2 R Sac<1/Land>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(
        mixed.additional_costs,
        ["cost!(\"{2}{R}\", Sacrifice(&Filter::YOUR_LAND))"]
    );

    let var_x = read("Name:X\nTypes:Creature\nK:Kicker:X\nSVar:X:Count$xPaid");
    assert_eq!(var_x.additional_costs, ["cost!(\"{X}\")"]);

    for (script, expected) in [
        (
            "Name:X\nTypes:Instant\nK:Kicker:W:U:B\nA:SP$ Draw | NumCards$ 1",
            "a `Kicker` with more than two costs",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:1 G:2 U\nA:SP$ Draw | NumCards$ 1\nOracle:Kicker {W} and/or {B}",
            "a dual `Kicker` cost that disagrees with its Oracle text",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:Forage\nA:SP$ Draw | NumCards$ 1",
            "cost `Forage`",
        ),
        (
            "Name:X\nTypes:Creature\nK:Kicker:XMin1 X\nSVar:X:Count$xPaid",
            "a variable-X `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:\nA:SP$ Draw | NumCards$ 1",
            "an empty `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:1\nK:Kicker:2\nA:SP$ Draw | NumCards$ 1",
            "a second `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:1\nK:Multikicker:2\nA:SP$ Draw | NumCards$ 1",
            "a second `Kicker` cost",
        ),
    ] {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "should refuse: {script}"
        );
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(expected),
            "wrong refusal reason for: {script}"
        );
    }
}

#[test]
fn kicked_etb_trigger_reads_entering_object_and_refuses_dual_and_static() {
    let body = read_with_tokens(
        "Name:X\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         K:Kicker:1 R\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked | Execute$ Trig\n\
         SVar:Trig:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ TriggeredCardController",
    );
    assert_eq!(body.additional_costs, ["cost!(\"{1}{R}\")"]);
    assert!(
        body.statics
            .contains("static TRIGGER1: Filter = Filter::And(&[Filter::This, Filter::Kicked]);"),
        "{}",
        body.statics
    );
    assert_eq!(
        body.abilities,
        ["triggered!(Trigger::EntersBattlefield(&TRIGGER1), \
             &[Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }])"]
    );

    let dual = parse(
        "Name:X\nTypes:Creature\nK:Kicker:1 R\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked 1 | Execute$ Trig\n\
         SVar:Trig:DB$ Draw | NumCards$ 1",
    );
    assert!(transcode(&dual, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&dual, &cats(), None).as_deref(),
        Some("filter atom `kicked 1`")
    );

    let static_replacement = parse(
        "Name:X\nTypes:Creature\nK:Kicker:1 R\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked | Static$ True | Execute$ Trig\n\
         SVar:Trig:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2",
    );
    assert!(transcode(&static_replacement, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&static_replacement, &cats(), None).as_deref(),
        Some("unclaimed parameter `ChangesZone.Static`")
    );
}

#[test]
fn etb_self_and_spell_kicked_conditions_emit_if_kicked_and_merge() {
    let etb = read(
        "Name:X\nManaCost:3 B\nTypes:Creature\nPT:3/3\n\
         K:Kicker:W W\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Unkicked\n\
         SVar:Unkicked:DB$ DestroyAll | ValidCards$ Land.YouCtrl | ConditionDefined$ Self | ConditionPresent$ Card.!kicked | SubAbility$ Kicked\n\
         SVar:Kicked:DB$ DestroyAll | ValidCards$ Land | ConditionDefined$ Self | ConditionPresent$ Card.kicked",
    );
    assert_eq!(etb.additional_costs, ["cost!(\"{W}{W}\")"]);
    assert!(
        !etb.statics.contains("CHECK"),
        "ETB kicked branch must use IfKicked rather than SourceMatches: {}",
        etb.statics
    );
    assert_eq!(
        etb.abilities,
        ["triggered!(Trigger::ETB, &[Effect::IfKicked { \
             then: &[Effect::destroy_all(&Filter::LAND)], \
             otherwise: &[Effect::destroy_all(&Filter::YOUR_LAND)] }])"]
    );

    let spell = read(
        "Name:X\nManaCost:3 B\nTypes:Sorcery\n\
         K:Kicker:2 W\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouOwn | SubAbility$ Kicked\n\
         SVar:Kicked:DB$ GainLife | LifeAmount$ X | Condition$ Kicked | SubAbility$ Unkicked\n\
         SVar:Unkicked:DB$ LoseLife | LifeAmount$ X | ConditionCheckSVar$ K | ConditionSVarCompare$ EQ0\n\
         SVar:X:Targeted$CardManaCost\n\
         SVar:K:Count$TimesKicked",
    );
    assert!(
        !spell.statics.contains("CmcAtMostX"),
        "non-X spell reading Targeted$CardManaCost must not restrict target mana value: {}",
        spell.statics
    );
    assert_eq!(
        spell.abilities,
        [
            "spell!(&[Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), owner_control: false, counters: None }, \
             Effect::IfKicked { then: &[Effect::GainLife { amount: Amount::TargetCmc }], otherwise: &[Effect::LoseLife { amount: Amount::TargetCmc, target: PlayerRel::You }] }], \
             targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))))"
        ]
    );
}

#[test]
fn kicked_conditions_are_refused_where_stack_object_carries_no_kicked_flag() {
    for (script, expected) in [
        (
            "Name:X\nTypes:Artifact\nK:Kicker:1\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | Condition$ Kicked",
            "`Condition$ Kicked` outside a spell or self-ETB trigger",
        ),
        (
            "Name:X\nTypes:Creature\nK:Kicker:1\n\
             T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ Trig\n\
             SVar:Trig:DB$ Draw | NumCards$ 1 | Condition$ Kicked",
            "`Condition$ Kicked` outside a spell or self-ETB trigger",
        ),
        (
            "Name:X\nTypes:Creature\nK:Kicker:1\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked | Execute$ Trig\n\
             SVar:Trig:DB$ Draw | NumCards$ 1 | Condition$ Kicked",
            "`Condition$ Kicked` outside a spell or self-ETB trigger",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:1\n\
             A:SP$ Draw | NumCards$ 1 | Condition$ Kicked 1",
            "a dual-kicker `Condition$`",
        ),
        (
            "Name:X\nTypes:Instant\nK:Multikicker:1\n\
             A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ K | ConditionSVarCompare$ EQ1\n\
             SVar:K:Count$TimesKicked",
            "kicked comparison `Count$TimesKicked` `EQ1`",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:1\n\
             A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ K | ConditionSVarCompare$ EQ2\n\
             SVar:K:Count$TimesKicked",
            "kicked comparison `Count$TimesKicked` `EQ2`",
        ),
    ] {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "should refuse: {script}"
        );
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(expected),
            "wrong refusal reason for: {script}"
        );
    }
}

#[test]
fn count_kicked_amounts_split_into_if_kicked_and_refuse_non_pump_zero() {
    let burn = read(
        "Name:X\nManaCost:R\nTypes:Instant\n\
         K:Kicker:4\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ N\n\
         SVar:N:Count$Kicked.4.2",
    );
    assert_eq!(
        burn.abilities,
        ["spell!(&[Effect::IfKicked { \
             then: &[Effect::DealDamage { amount: Amount::Fixed(4), target: TargetSpec::AnyTarget }], \
             otherwise: &[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::AnyTarget }] }], \
             targets = Some(TargetReq::one(TargetSpec::AnyTarget)))"]
    );

    let pump_with_kw = read(
        "Name:X\nManaCost:1 R\nTypes:Sorcery\n\
         K:Kicker:G\n\
         A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | KW$ First Strike | NumAtt$ +N | NumDef$ +N\n\
         SVar:N:Count$Kicked.1.0",
    );
    assert_eq!(
        pump_with_kw.abilities,
        ["spell!(&[Effect::IfKicked { \
             then: &[Effect::PumpFilter { filter: &Filter::YOUR_CREATURE, controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(1), keywords: KeywordSet::FIRST_STRIKE, duration: Duration::UntilEndOfTurn }], \
             otherwise: &[Effect::PumpFilter { filter: &Filter::YOUR_CREATURE, controlled_by: None, power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::FIRST_STRIKE, duration: Duration::UntilEndOfTurn }] }])"]
    );

    let neg_pump = read(
        "Name:X\nManaCost:1 B\nTypes:Instant\n\
         K:Kicker:2 B\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -N | NumDef$ -N | IsCurse$ True\n\
         SVar:N:Count$Kicked.5.2",
    );
    assert_eq!(
        neg_pump.abilities,
        ["spell!(&[Effect::IfKicked { \
             then: &[Effect::PumpTarget { power: Amount::NegXFixed(5), toughness: Amount::NegXFixed(5), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }], \
             otherwise: &[Effect::PumpTarget { power: Amount::NegXFixed(2), toughness: Amount::NegXFixed(2), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }] }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"]
    );

    assert!(refused(
        "Name:X\nManaCost:2 R\nTypes:Instant\n\
         K:Kicker:8 R\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ N\n\
         SVar:N:Count$Kicked.0.3",
    ));
}

#[test]
fn kicked_targets_replacement_count_change_and_kicked_only_target() {
    let replace = read(
        "Name:X\nManaCost:1 G\nTypes:Instant\n\
         K:Kicker:1 B\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Artifact,Enchantment | TargetMin$ A | TargetMax$ A | SubAbility$ B\n\
         SVar:B:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Permanent.nonLand | TargetMin$ C | TargetMax$ C | Condition$ Kicked\n\
         SVar:A:Count$Kicked.0.1\n\
         SVar:C:Count$Kicked.1.0",
    );
    assert_eq!(replace.additional_costs, ["cost!(\"{1}{B}\")"]);
    assert_eq!(
        replace.kicked_targets.as_deref(),
        Some("Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND)))")
    );
    assert_eq!(
        replace.abilities,
        [
            "spell!(&[Effect::exile(TargetSpec::Object(&Filter::ARTIFACT_OR_ENCHANTMENT))], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT_OR_ENCHANTMENT))))"
        ]
    );

    let exact_two = read(
        "Name:X\nManaCost:B\nTypes:Sorcery\n\
         K:Kicker:3\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouOwn | TargetMin$ N | TargetMax$ N\n\
         SVar:N:Count$Kicked.2.1",
    );
    assert_eq!(
        exact_two.kicked_targets.as_deref(),
        Some(
            "Some(TargetReq::exactly(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), 2))"
        )
    );
    assert_eq!(
        exact_two.abilities,
        [
            "spell!(&[Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You) }], \
             targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))))"
        ]
    );

    let any_number = read(
        "Name:X\nManaCost:W\nTypes:Instant\n\
         K:Kicker:2 W\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | TargetMin$ A | TargetMax$ B | KW$ Indestructible\n\
         SVar:A:Count$Kicked.0.1\n\
         SVar:B:Count$Kicked.C.1\n\
         SVar:C:Count$Valid Creature.YouCtrl",
    );
    assert_eq!(
        any_number.kicked_targets.as_deref(),
        Some("Some(TargetReq::up_to(TargetSpec::Object(&Filter::YOUR_CREATURE), u8::MAX))")
    );
    assert_eq!(
        any_number.abilities,
        [
            "spell!(&[Effect::PumpTarget { power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::INDESTRUCTIBLE, duration: Duration::UntilEndOfTurn }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))))"
        ]
    );

    let kicked_only = read(
        "Name:X\nManaCost:2 U\nTypes:Sorcery\n\
         K:Kicker:1 B\n\
         A:SP$ Draw | NumCards$ 3 | SubAbility$ DiscYou\n\
         SVar:DiscYou:DB$ Discard | Defined$ You | NumCards$ 2 | Mode$ TgtChoose | SubAbility$ DiscTgt\n\
         SVar:DiscTgt:DB$ Discard | Condition$ Kicked | ValidTgts$ Player | NumCards$ 2 | Mode$ TgtChoose",
    );
    assert_eq!(
        kicked_only.kicked_targets.as_deref(),
        Some("Some(TargetReq::one(TargetSpec::AnyPlayer))")
    );
    assert_eq!(
        kicked_only.abilities,
        ["spell!(&[Effect::draw(3), \
             Effect::DiscardForPlayers { who: PlayerRel::You, count: 2 }, \
             Effect::IfKicked { then: &[Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 2 }], otherwise: &[] }])"]
    );
}

#[test]
fn kicked_second_target_and_growing_target_counts_are_refused() {
    let second_target = parse(
        "Name:X\nManaCost:2 W\nTypes:Instant\n\
         K:Kicker:R\n\
         A:SP$ Destroy | ValidTgts$ Artifact | SubAbility$ K\n\
         SVar:K:DB$ DealDamage | Condition$ Kicked | ValidTgts$ Creature | NumDmg$ 2 | TargetMin$ Y | TargetMax$ Y\n\
         SVar:Y:Count$Kicked.1.0",
    );
    assert!(transcode(&second_target, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&second_target, &cats(), None).as_deref(),
        Some("a kicked-only second target")
    );

    let comet = parse(
        "Name:X\nManaCost:X R R\nTypes:Instant\n\
         K:Multikicker:1\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | TargetMin$ T | TargetMax$ T\n\
         SVar:T:Count$TimesKicked/Plus.1\n\
         SVar:X:Count$xPaid",
    );
    assert!(transcode(&comet, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&comet, &cats(), None).as_deref(),
        Some("a target count of `Count$TimesKicked/Plus.1`")
    );

    let etb_growing = parse(
        "Name:X\nManaCost:3 U\nTypes:Creature\nPT:3/3\n\
         K:Multikicker:U\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ Pump | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ K | KW$ Flying\n\
         SVar:K:Count$TimesKicked",
    );
    assert!(transcode(&etb_growing, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&etb_growing, &cats(), None).as_deref(),
        Some("a target count of `Count$TimesKicked`")
    );

    assert!(refused(
        "Name:X\nManaCost:W\nTypes:Instant\n\
         K:Kicker:2 W\n\
         A:SP$ Destroy | ValidTgts$ Creature | TargetMin$ A | TargetMax$ A | SubAbility$ B\n\
         SVar:B:DB$ Destroy | ValidTgts$ Artifact | NoRegen$ True | TargetMin$ C | TargetMax$ C | Condition$ Kicked\n\
         SVar:A:Count$Kicked.0.1\n\
         SVar:C:Count$Kicked.1.0",
    ));
}

#[test]
fn kicked_etb_counters_require_single_kicker_and_count_kicked_svar() {
    let one = read(
        "Name:X\nManaCost:1 W\nTypes:Creature\nPT:1/2\n\
         K:Kicker:2\n\
         K:etbCounter:P1P1:1:CheckSVar$ K:If kicked, enters with a counter.\n\
         SVar:K:Count$Kicked.1.0",
    );
    assert_eq!(
        one.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::TimesKicked }"]
    );

    // Order of K: lines must not matter.
    let three_before_kicker = read(
        "Name:X\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         K:etbCounter:P1P1:3:CheckSVar$ K:If kicked, enters with three counters.\n\
         K:Kicker:4\n\
         SVar:K:Count$Kicked.1.0",
    );
    assert_eq!(
        three_before_kicker.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 3 } }"]
    );

    // Same SVar name with a different definition must stay refused.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         K:Kicker:2\n\
         K:etbCounter:P1P1:2:CheckSVar$ WasKicked:desc\n\
         SVar:WasKicked:Count$CardCounters.P1P1",
    ));
    // Multikicker with a fixed CheckSVar counter must stay refused.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         K:Multikicker:2\n\
         K:etbCounter:P1P1:2:CheckSVar$ K:desc\n\
         SVar:K:Count$Kicked.1.0",
    ));
    // Variable counter count on kicked etbCounter must stay refused.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         K:Kicker:2\n\
         K:etbCounter:P1P1:X:CheckSVar$ K:desc\n\
         SVar:K:Count$Kicked.1.0\n\
         SVar:X:Count$xPaid",
    ));
}

#[test]
fn kicked_filters_spend_restriction_end_step_and_multikicker_multiples() {
    let mana = read(
        "Name:X\nTypes:Creature\n\
         A:AB$ Mana | Cost$ T | Produced$ G | Amount$ 2 | RestrictValid$ Spell.Kicked",
    );
    assert_eq!(
        mana.abilities,
        [
            "mana_ability!(&[Effect::mana(ManaColor::Green, 2).restricted(&Filter::Kicked, SpendRider::None)])"
        ]
    );

    let cast_trig = read(
        "Name:X\nTypes:Creature\n\
         T:Mode$ SpellCast | ValidCard$ Card.kicked | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ Trig\n\
         SVar:Trig:DB$ Scry | ScryNum$ 2",
    );
    assert!(
        cast_trig.statics.contains(
            "static TRIGGER1: Filter = Filter::And(&[Filter::Kicked, Filter::ControlledByYou]);"
        ),
        "{}",
        cast_trig.statics
    );

    let narrowed_player_kicker = parse(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card.YouCtrl+kicked | TriggerZones$ Battlefield | Execute$ Trig\n\
         SVar:Trig:DB$ Draw | NumCards$ 1\n\
         Oracle:Whenever a player kicks a spell, draw a card.",
    );
    assert!(transcode(&narrowed_player_kicker, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&narrowed_player_kicker, &cats(), None).as_deref(),
        Some("a `SpellCast` trigger restricting a player-wide kicker trigger to `YouCtrl`")
    );

    let not_kicked = read(
        "Name:X\nTypes:Creature\nK:Kicker:R\n\
         T:Mode$ Phase | Phase$ End of Turn | IsPresent$ Card.Self+!kicked | TriggerZones$ Battlefield | Execute$ Sac\n\
         SVar:Sac:DB$ Sacrifice",
    );
    assert!(
        not_kicked.statics.contains(
            "static CHECK1: Filter = Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::This, Filter::Not(&Filter::Kicked)]);"
        ),
        "{}",
        not_kicked.statics
    );
    assert_eq!(
        not_kicked.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::End, whose: PlayerRel::EachPlayer }, \
             &[Effect::SacrificeSelf], condition = Some(Condition::SourceMatches(&CHECK1)))"
        ]
    );

    let counter_if_kicked = read(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | ConditionDefined$ Targeted | ConditionPresent$ Spell.kicked",
    );
    assert_eq!(
        counter_if_kicked.abilities,
        [
            "spell!(&[Effect::IfTargetMatches { filter: &Filter::Kicked, then: &[Effect::CounterTargetSpell] }], \
             targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))"
        ]
    );

    let doubled = read(
        "Name:X\nManaCost:3 W\nTypes:Creature\n\
         K:Multikicker:W\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ GainLife | LifeAmount$ N\n\
         SVar:N:Count$TimesKicked/Times.2",
    );
    assert_eq!(
        doubled.abilities,
        ["triggered!(Trigger::ETB, &[Effect::GainLife { \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 2 } }])"]
    );

    let tripled = read(
        "Name:X\nManaCost:3 W\nTypes:Creature\n\
         K:Multikicker:W\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ GainLife | LifeAmount$ N\n\
         SVar:N:Count$TimesKicked/Thrice",
    );
    assert_eq!(
        tripled.abilities,
        ["triggered!(Trigger::ETB, &[Effect::GainLife { \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 3 } }])"]
    );
}

/// CR 702.33g: a target of a part that happens only if the spell was kicked
/// is chosen only then. Gated on kicking through `ConditionCheckSVar$` or
/// `ConditionPresent$`, such a target line has no `kicked_targets` reading and
/// would be demanded on every cast, so it is refused by name.
#[test]
fn a_target_gated_on_kicking_by_svar_or_presence_is_refused() {
    for gate in [
        "ConditionCheckSVar$ K | ConditionSVarCompare$ GE1\nSVar:K:Count$Kicked.1.0",
        "ConditionCheckSVar$ K | ConditionSVarCompare$ GE1\nSVar:K:Count$TimesKicked",
        "ConditionDefined$ Self | ConditionPresent$ Card.kicked",
    ] {
        let script = parse(&format!(
            "Name:X\nManaCost:1 R\nTypes:Instant\nK:Kicker:1 R\n\
             A:SP$ Draw | NumCards$ 1 | SubAbility$ DBDmg\n\
             SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | {gate}"
        ));
        assert!(transcode(&script, &cats(), None).is_none(), "{gate}");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(
                "a target behind a kicked condition read through \
                 `ConditionCheckSVar$` or `ConditionPresent$`"
            ),
            "{gate}"
        );
    }
    // No gate on this spell's kicking: a line that happens only unkicked keeps
    // the target its kicked half reads as "that permanent" too, and "counter
    // target spell if it was kicked" asks the target, not this spell.
    let unkicked_only = parse(
        "Name:X\nManaCost:U\nTypes:Instant\nK:Kicker:2 B\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand | \
         SubAbility$ DBBottom | ConditionCheckSVar$ K | ConditionSVarCompare$ EQ0\n\
         SVar:DBBottom:DB$ ChangeZone | Defined$ Targeted | Origin$ Battlefield | \
         Destination$ Library | LibraryPosition$ -1 | Condition$ Kicked\n\
         SVar:K:Count$TimesKicked",
    );
    assert_ne!(
        refusal_reason(&unkicked_only, &cats(), None).as_deref(),
        Some(
            "a target behind a kicked condition read through \
             `ConditionCheckSVar$` or `ConditionPresent$`"
        )
    );
    let on_the_target = parse(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | \
         ConditionDefined$ Targeted | ConditionPresent$ Spell.kicked",
    );
    let body = transcode(&on_the_target, &cats(), None).expect("read in full");
    assert!(
        body.abilities[0].contains("Effect::IfTargetMatches { filter: &Filter::Kicked"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn dual_kicker_reorders_reversed_script_to_match_oracle_order() {
    let body = read(
        "Name:Reversed Mage\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         K:Kicker:2 U:1 G\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked 1 | Execute$ DrawTwo\n\
         SVar:DrawTwo:DB$ Draw | NumCards$ 2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+kicked 2 | Execute$ KillFlier\n\
         SVar:KillFlier:DB$ Destroy | ValidTgts$ Creature.withFlying\n\
         Oracle:Kicker {1}{G} and/or {2}{U} (You may pay an additional {1}{G} and/or {2}{U} as you cast this spell.)\\n\
         When Reversed Mage enters, if it was kicked with its {1}{G} kicker, destroy target creature with flying.\\n\
         When Reversed Mage enters, if it was kicked with its {2}{U} kicker, draw two cards.",
    );
    assert_eq!(
        body.additional_costs,
        ["cost!(\"{1}{G}\")", "cost!(\"{2}{U}\")"]
    );
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::destroy(TargetSpec::Object(&TARGET1))], \
             targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1))), \
             condition = Some(Condition::KickedWith(0)))",
            "triggered!(Trigger::ETB, &[Effect::draw(2)], \
             condition = Some(Condition::KickedWith(1)))",
        ]
    );
}

#[test]
fn dual_kicker_spell_chains_kicked_with_conditions_and_targeted_controller() {
    let body = read(
        "Name:Dual Burn\nManaCost:X R\nTypes:Sorcery\n\
         K:Kicker:2 R:3 U\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | SubAbility$ HitCtrl\n\
         SVar:HitCtrl:DB$ DealDamage | Defined$ TargetedController | NumDmg$ X | SubAbility$ DrawX | Condition$ Kicked 1\n\
         SVar:DrawX:DB$ Draw | NumCards$ X | Defined$ You | Condition$ Kicked 2\n\
         SVar:X:Count$xPaid\n\
         Oracle:Kicker {2}{R} and/or {3}{U}",
    );
    assert_eq!(
        body.additional_costs,
        ["cost!(\"{2}{R}\")", "cost!(\"{3}{U}\")"]
    );
    assert_eq!(
        body.abilities,
        ["spell!(&[\
             Effect::DealDamage { amount: Amount::X, target: TargetSpec::Object(&Filter::CREATURE) }, \
             Effect::IfCondition { condition: Condition::KickedWith(0), then: &[Effect::DealDamage { amount: Amount::X, target: TargetSpec::Player(PlayerRel::ControllerOfTarget) }], otherwise: &[] }, \
             Effect::IfCondition { condition: Condition::KickedWith(1), then: &[Effect::DrawCards { amount: Amount::X }], otherwise: &[] }\
             ], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"]
    );
}

#[test]
fn dual_kicker_volver_etb_replacements_emit_if_kicked_with_keywords_activated_and_triggered() {
    let body = read(
        "Name:Sample Volver\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         K:Kicker:1 G:W\n\
         K:ETBReplacement:Other:First:Mandatory::Card.Self+kicked 1\n\
         K:ETBReplacement:Other:Second:Mandatory::Card.Self+kicked 2\n\
         SVar:First:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2 | ETB$ True | SubAbility$ FirstGrant\n\
         SVar:FirstGrant:DB$ Animate | Defined$ Self | Abilities$ ABRegen | Duration$ Permanent\n\
         SVar:ABRegen:AB$ Regenerate | Cost$ PayLife<3>\n\
         SVar:Second:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | ETB$ True | SubAbility$ SecondGrant\n\
         SVar:SecondGrant:DB$ Animate | Defined$ Self | Triggers$ GainTrig | Duration$ Permanent\n\
         SVar:GainTrig:Mode$ DamageDealtOnce | ValidSource$ Card.Self | Execute$ DoGain | TriggerZones$ Battlefield\n\
         SVar:DoGain:DB$ GainLife | Defined$ You | LifeAmount$ DmgX\n\
         SVar:DmgX:TriggerCount$DamageAmount\n\
         Oracle:Kicker {1}{G} and/or {W}",
    );
    assert_eq!(
        body.additional_costs,
        ["cost!(\"{1}{G}\")", "cost!(\"{W}\")"]
    );
    assert_eq!(
        body.enter_modifiers,
        [
            "EnterModifier::IfKicked { index: Some(0), counters: 2, modifiers: &[Modifier::GrantActivated { cost: cost!(PayLife(3)), effects: &[Effect::regenerate(TargetSpec::ThisObject)], mana_ability: false }] }",
            "EnterModifier::IfKicked { index: Some(1), counters: 1, modifiers: &[Modifier::GrantTriggered { trigger: Trigger::DealsDamage(&Filter::This), effects: &[Effect::GainLife { amount: Amount::EventAmount }], target: None }] }",
        ]
    );
}

#[test]
fn wave18a_kicker_damage_you_discard_hand_and_snow_mana() {
    let snow = read("Name:X\nTypes:Instant\nK:Kicker:1 S\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(snow.additional_costs, ["cost!(\"{1}{S}\")"]);

    let dmg = read("Name:X\nTypes:Instant\nK:Kicker:DamageYou<3>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(dmg.additional_costs, ["cost!(DealDamageToYou(3))"]);

    let hand = read("Name:X\nTypes:Instant\nK:Kicker:Discard<1/Hand>\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(hand.additional_costs, ["cost!(DiscardHand)"]);

    for bad in [
        "Name:X\nTypes:Instant\nK:Kicker:DamageYou<0>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nTypes:Instant\nK:Kicker:DamageYou<X>\nSVar:X:Count$xPaid\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave19a_kicker_and_spell_cost_phyrexian_and_two_generic_mana() {
    let kicker = read(
        "Name:PhyrKick\nManaCost:1 G\nTypes:Instant\n\
         K:Kicker:2W BP GUP\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(kicker.additional_costs, ["cost!(\"{2/W}{B/P}{G/U/P}\")"]);

    let spell = read(
        "Name:PhyrSpell\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 2R UP Sac<1/Creature> | NumCards$ 2",
    );
    assert_eq!(
        spell.mandatory_additional_costs,
        ["CostPart::Sacrifice(&Filter::YOUR_CREATURE)"]
    );

    for bad in [
        "Name:X\nTypes:Instant\nK:Kicker:CP\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nTypes:Instant\nK:Kicker:WWP\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nTypes:Instant\nA:SP$ Draw | Cost$ 2C | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21a_kicker_and_spell_remove_any_counter() {
    let spell_fixed = read(
        "Name:CounterSpell\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 B RemoveAnyCounter<2/Any/Permanent> | NumCards$ 2",
    );
    assert_eq!(
        spell_fixed.mandatory_additional_costs,
        ["CostPart::RemoveAnyCounter { count: 2, kind: None, filter: &Filter::Any }"]
    );

    let spell_x = read(
        "Name:CounterXSpell\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ B RemoveAnyCounter<X/P1P1/Creature> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        spell_x.mandatory_additional_costs,
        [
            "CostPart::RemoveAnyCounterX { kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }"
        ]
    );

    let spell_sub_any = read(
        "Name:SubAnySpell\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 U SubCounter<2/Any/Artifact.YouCtrl;Creature.YouCtrl> | NumCards$ 2",
    );
    assert_eq!(
        spell_sub_any.mandatory_additional_costs,
        [
            "CostPart::RemoveAnyCounter { count: 2, kind: None, filter: &Filter::ARTIFACT_OR_CREATURE }"
        ]
    );

    let kicker = read(
        "Name:CounterKicker\nManaCost:1 G\nTypes:Instant\n\
         K:Kicker:RemoveAnyCounter<2/P1P1/Creature>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        kicker.additional_costs,
        [
            "cost!(RemoveAnyCounter { count: 2, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE })"
        ]
    );

    let sub_spell = read(
        "Name:SubSpell\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ B SubCounter<1/P1P1/Creature.YouCtrl/a creature you control> | NumCards$ 1",
    );
    assert_eq!(
        sub_spell.mandatory_additional_costs,
        [
            "CostPart::RemoveAnyCounter { count: 1, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }"
        ]
    );

    let sub_kicker = read(
        "Name:SubKicker\nManaCost:G\nTypes:Instant\n\
         K:Kicker:SubCounter<1/P1P1/Creature.YouCtrl>\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        sub_kicker.additional_costs,
        [
            "cost!(RemoveAnyCounter { count: 1, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE })"
        ]
    );

    for bad in [
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B RemoveAnyCounter<1/Any/CARDNAME> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B RemoveAnyCounter<1/Any/Creature.Other> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B RemoveAnyCounter<X1+/P1P1/Creature> | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B SubCounter<1/Any/CARDNAME> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B SubCounter<1/P1P1> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B SubCounter<1/P1P1/CARDNAME> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B SubCounter<1/P1P1/Creature.OppCtrl> | NumCards$ 1",
        "Name:X\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B SubCounter<1/P1P1/Creature.YouCtrl/desc/Exile> | NumCards$ 1",
        "Name:X\nManaCost:G\nTypes:Instant\nK:Kicker:RemoveAnyCounter<1/Any/CARDNAME>\nA:SP$ Draw | NumCards$ 1",
        "Name:X\nManaCost:G\nTypes:Instant\nK:Kicker:SubCounter<1/P1P1>\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}
