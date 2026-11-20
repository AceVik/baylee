//! Storm (CR 702.40a–b): cast trigger counts all earlier spells this turn
//! across every player, each storm instance triggers separately, and spell
//! copies created on the stack never trigger storm.

use super::*;
use baylee_cards_dsl::{
    Amount, Filter, Modifier, PlayerRel, TargetReq, TargetSpec, ZoneRef, spell, static_ability,
};
use baylee_core::ids::AbilityRef;

const FILLER: u32 = 7450;
const STORM_BOLT: u32 = 7451;
const STORM_GRANTOR: u32 = 7452;
const DUAL_STORM: u32 = 7453;
const COPIER: u32 = 7454;

static GAIN_1: &[Effect] = &[Effect::gain_life(1)];
static LOSE_3_EACH_OPPONENT: &[Effect] = &[Effect::LoseLife {
    amount: Amount::Fixed(3),
    target: PlayerRel::EachOpponent,
}];
static COPY_SPELL: &[Effect] = &[Effect::CopyTargetSpell { mods: &[] }];

static FILLER_ABILITIES: &[AbilityDef] = &[spell!(GAIN_1)];
static STORM_BOLT_ABILITIES: &[AbilityDef] = &[spell!(LOSE_3_EACH_OPPONENT)];
static STORM_GRANTOR_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::HasType(TypeSet::SORCERY),
    Modifier::AddKeyword(KeywordSet::STORM),
)];
static DUAL_STORM_ABILITIES: &[AbilityDef] = &[
    spell!(LOSE_3_EACH_OPPONENT),
    static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::STORM),
        zone = ZoneRef::Stack,
    ),
];
static COPIER_ABILITIES: &[AbilityDef] = &[free(
    COPY_SPELL,
    Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
)];

fn storm_cards() -> Vec<&'static CardDef> {
    vec![
        card(
            FILLER,
            face("Filler", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            FILLER_ABILITIES,
        ),
        card(
            STORM_BOLT,
            face("Storm Bolt", "{0}", TypeSet::SORCERY),
            KeywordSet::STORM,
            STORM_BOLT_ABILITIES,
        ),
        card(
            STORM_GRANTOR,
            face("Storm Grantor", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            STORM_GRANTOR_ABILITIES,
        ),
        card(
            DUAL_STORM,
            face("Dual Storm", "{0}", TypeSet::SORCERY),
            KeywordSet::STORM,
            DUAL_STORM_ABILITIES,
        ),
        card(
            COPIER,
            face("Spell Copier", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            COPIER_ABILITIES,
        ),
    ]
}

fn synthetic_triggers(engine: &Bench, source: ObjectId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered {
                    source: s,
                    ability_index,
                    ..
                } if s == source && ability_index == AbilityRef::SYNTHETIC
            )
        })
        .count()
}

fn cast(engine: &mut Bench, seat: PlayerId, card: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("offered spell casts cleanly");
}

fn cast_two_earlier_spells(engine: &mut Bench) {
    let my_filler = the(engine, ZoneLocation::Hand(me()), FILLER);
    cast(engine, me(), my_filler);
    settle(engine, me());

    // Pass priority in our first main phase so the opponent casts an instant
    // on this same turn before our Storm sorcery.
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let (player, _) = offer(engine);
    assert_eq!(player, them(), "opponent receives priority to cast instant");
    let their_filler = the(engine, ZoneLocation::Hand(them()), FILLER);
    cast(engine, them(), their_filler);
    settle(engine, me());
}

#[test]
fn single_storm_counts_both_players_earlier_spells_and_copies_do_not_retrigger() {
    let mut engine = bench(
        7450,
        storm_cards(),
        [
            Seat::default().holding(&[FILLER, STORM_BOLT]),
            Seat::default().holding(&[FILLER]),
        ],
    );
    to_main(&mut engine, me());
    cast_two_earlier_spells(&mut engine);

    let bolt = the(&engine, ZoneLocation::Hand(me()), STORM_BOLT);
    cast(&mut engine, me(), bolt);
    settle(&mut engine, me());

    assert_eq!(
        synthetic_triggers(&engine, bolt),
        1,
        "one Storm instance triggers once on cast (CR 702.40a)"
    );
    assert_eq!(
        life(&engine, them()),
        12,
        "before=2 (1 ours + 1 opponent's) yields 2 copies + 1 original = 3x3 = 9 life lost from 21"
    );
    assert_eq!(
        engine.state().per_turn.spells_cast.as_slice(),
        &[2, 1],
        "Storm copies are not cast and must not increment per_turn.spells_cast"
    );
}

#[test]
fn two_storm_instances_via_layer_6_grant_trigger_separately() {
    let mut engine = bench(
        7451,
        storm_cards(),
        [
            Seat::with(&[STORM_GRANTOR]).holding(&[FILLER, STORM_BOLT]),
            Seat::default().holding(&[FILLER]),
        ],
    );
    to_main(&mut engine, me());
    cast_two_earlier_spells(&mut engine);

    let bolt = the(&engine, ZoneLocation::Hand(me()), STORM_BOLT);
    cast(&mut engine, me(), bolt);
    settle(&mut engine, me());

    assert_eq!(
        synthetic_triggers(&engine, bolt),
        2,
        "printed Storm + Layer 6 AddKeyword(STORM) are two separate instances (CR 702.40b)"
    );
    assert_eq!(
        life(&engine, them()),
        6,
        "two Storm triggers x before=2 = 4 copies + 1 original = 5x3 = 15 life lost from 21"
    );
}

#[test]
fn duplicate_printed_storm_triggers_twice_even_at_zero_before() {
    let mut engine = bench(
        7452,
        storm_cards(),
        [
            Seat::default().holding(&[DUAL_STORM, DUAL_STORM, FILLER]),
            Seat::default().at(40).holding(&[FILLER]),
        ],
    );
    to_main(&mut engine, me());

    // First spell of the turn: before=0, both Storm instances still trigger.
    let first = objects(&engine, ZoneLocation::Hand(me()), DUAL_STORM)[0];
    cast(&mut engine, me(), first);
    settle(&mut engine, me());
    assert_eq!(
        synthetic_triggers(&engine, first),
        2,
        "duplicate Storm still creates two triggers when before=0"
    );
    assert_eq!(
        life(&engine, them()),
        37,
        "0 copies + 1 original = 3 life lost from 40"
    );

    // Cast our filler and opponent's filler (3 prior spells total on this turn:
    // first Dual Storm, our Filler, opponent's Filler; opponent gains 1 to 38).
    cast_two_earlier_spells(&mut engine);
    let second = the(&engine, ZoneLocation::Hand(me()), DUAL_STORM);
    cast(&mut engine, me(), second);
    settle(&mut engine, me());

    assert_eq!(
        synthetic_triggers(&engine, second),
        2,
        "second cast with duplicate printed Storm fires 2 triggers (CR 702.40b)"
    );
    assert_eq!(
        life(&engine, them()),
        17,
        "before=3 x 2 instances = 6 copies + 1 original = 7x3 = 21 life lost from 38"
    );
}

#[test]
fn copying_a_storm_spell_on_the_stack_does_not_trigger_storm() {
    let mut engine = bench(
        7453,
        storm_cards(),
        [
            Seat::with(&[COPIER]).holding(&[FILLER, STORM_BOLT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let filler = the(&engine, ZoneLocation::Hand(me()), FILLER);
    cast(&mut engine, me(), filler);
    settle(&mut engine, me());

    let bolt = the(&engine, ZoneLocation::Hand(me()), STORM_BOLT);
    cast(&mut engine, me(), bolt);
    let copier = the(&engine, ZoneLocation::Battlefield, COPIER);
    activate(&mut engine, me(), copier, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        synthetic_triggers(&engine, bolt),
        1,
        "only the cast of Storm Bolt triggered Storm; Copier's spell copy did not"
    );
    // Original Storm Bolt (1) + Storm trigger copy (1) + Copier copy (1) = 3 resolutions -> 9 damage.
    assert_eq!(
        life(&engine, them()),
        11,
        "copied Storm spell resolves cleanly without adding its own Storm trigger (CR 702.40a)"
    );
}
