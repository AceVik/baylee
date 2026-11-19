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
fn kicker_refuses_dual_non_mana_variable_x_empty_and_duplicate_costs() {
    for (script, expected) in [
        (
            "Name:X\nTypes:Instant\nK:Kicker:W:2 B\nA:SP$ Draw | NumCards$ 1",
            "a dual `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:Sac<1/Land>\nA:SP$ Draw | NumCards$ 1",
            "a non-mana `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:PayLife<3>\nA:SP$ Draw | NumCards$ 1",
            "a non-mana `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:2 R Sac<1/Land>\nA:SP$ Draw | NumCards$ 1",
            "a non-mana `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Instant\nK:Kicker:Forage\nA:SP$ Draw | NumCards$ 1",
            "a non-mana `Kicker` cost",
        ),
        (
            "Name:X\nTypes:Creature\nK:Kicker:X\nSVar:X:Count$xPaid",
            "a variable-X `Kicker` cost",
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
        [
            "triggered!(Trigger::EntersBattlefield(&TRIGGER1), \
             &[Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }])"
        ]
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
        [
            "triggered!(Trigger::ETB, &[Effect::IfKicked { \
             then: &[Effect::destroy_all(&Filter::LAND)], \
             otherwise: &[Effect::destroy_all(&Filter::YOUR_LAND)] }])"
        ]
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
        [
            "spell!(&[Effect::IfKicked { \
             then: &[Effect::DealDamage { amount: Amount::Fixed(4), target: TargetSpec::AnyTarget }], \
             otherwise: &[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::AnyTarget }] }], \
             targets = Some(TargetReq::one(TargetSpec::AnyTarget)))"
        ]
    );

    let pump_with_kw = read(
        "Name:X\nManaCost:1 R\nTypes:Sorcery\n\
         K:Kicker:G\n\
         A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | KW$ First Strike | NumAtt$ +N | NumDef$ +N\n\
         SVar:N:Count$Kicked.1.0",
    );
    assert_eq!(
        pump_with_kw.abilities,
        [
            "spell!(&[Effect::IfKicked { \
             then: &[Effect::PumpFilter { filter: &Filter::YOUR_CREATURE, controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(1), keywords: KeywordSet::FIRST_STRIKE, duration: Duration::UntilEndOfTurn }], \
             otherwise: &[Effect::PumpFilter { filter: &Filter::YOUR_CREATURE, controlled_by: None, power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::FIRST_STRIKE, duration: Duration::UntilEndOfTurn }] }])"
        ]
    );

    let neg_pump = read(
        "Name:X\nManaCost:1 B\nTypes:Instant\n\
         K:Kicker:2 B\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -N | NumDef$ -N | IsCurse$ True\n\
         SVar:N:Count$Kicked.5.2",
    );
    assert_eq!(
        neg_pump.abilities,
        [
            "spell!(&[Effect::IfKicked { \
             then: &[Effect::PumpTarget { power: Amount::NegXFixed(5), toughness: Amount::NegXFixed(5), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }], \
             otherwise: &[Effect::PumpTarget { power: Amount::NegXFixed(2), toughness: Amount::NegXFixed(2), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }] }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
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
        [
            "spell!(&[Effect::draw(3), \
             Effect::DiscardForPlayers { who: PlayerRel::You, count: 2 }, \
             Effect::IfKicked { then: &[Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 2 }], otherwise: &[] }])"
        ]
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
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 3 } }"
        ]
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
        ["mana_ability!(&[Effect::mana(ManaColor::Green, 2).restricted(&Filter::Kicked, SpendRider::None)])"]
    );

    let cast_trig = read(
        "Name:X\nTypes:Creature\n\
         T:Mode$ SpellCast | ValidCard$ Card.kicked | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ Trig\n\
         SVar:Trig:DB$ Scry | ScryNum$ 2",
    );
    assert!(
        cast_trig
            .statics
            .contains("static TRIGGER1: Filter = Filter::And(&[Filter::Kicked, Filter::ControlledByYou]);"),
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
        [
            "triggered!(Trigger::ETB, &[Effect::GainLife { \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 2 } }])"
        ]
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
        [
            "triggered!(Trigger::ETB, &[Effect::GainLife { \
             amount: Amount::Times { base: &Amount::TimesKicked, factor: 3 } }])"
        ]
    );
}
