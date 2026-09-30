//! What effects add to the declaration of attackers: requirements
//! (CR 508.1d, "attacks each combat if able") and the restrictions about a
//! creature and what it attacks (CR 508.1c, "can't attack unless defending
//! player controls an Island").
//!
//! Played with creatures built for it, each with one sentence, beside a
//! vanilla body that shows the board can tell the difference.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Modifier,
    static_ability,
};
use baylee_core::color::ColorSet;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 5/3 that attacks each combat if able.
const EAGER: u32 = 1130;
/// A 5/5 that can't attack unless defending player controls an Island.
const SEAFARER: u32 = 1131;
/// A vanilla 2/2.
const BEAR: u32 = 1132;

static ISLAND: Filter = Filter::HasSubtype(land::ISLAND);
static EAGER_ABILITIES: &[AbilityDef] =
    &[static_ability!(Filter::This, Modifier::AttacksEachCombat)];
static SEAFARER_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CantAttackUnlessDefenderControls(&ISLAND)
)];

fn fighter(
    index: u32,
    name: &'static str,
    power: i16,
    toughness: i16,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            abilities,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        fighter(EAGER, "Eager", 5, 3, EAGER_ABILITIES),
        fighter(SEAFARER, "Seafarer", 5, 5, SEAFARER_ABILITIES),
        fighter(BEAR, "Bear", 2, 2, &[]),
    ])
}

/// The pool's Island.
fn island() -> u32 {
    baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get()
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// Seat 0 with `mine`, seat 1 with `theirs`, both hands kept, and seat 0
/// asked to declare attackers on its first turn.
fn combat(mine: &[u32], theirs: &[u32]) -> Engine<SyntheticLookup> {
    combat_after(mine, theirs, |_| {})
}

/// As [`combat`], with `prepare` run on seat 0's board in its first main
/// phase, after the untap step and before the question is built.
fn combat_after(
    mine: &[u32],
    theirs: &[u32],
    prepare: impl FnOnce(&mut Engine<SyntheticLookup>),
) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset_both(37, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let mut prepare = Some(prepare);
    for _ in 0..40 {
        if engine.state().turn.step == crate::turn::Step::Main
            && let Some(prepare) = prepare.take()
        {
            prepare(&mut engine);
        }
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME => return engine,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question on the way to combat: {other:?}"),
        }
    }
    panic!("seat 0 was never asked to attack");
}

fn one(engine: &Engine<SyntheticLookup>, index: u32) -> ObjectId {
    permanents(engine, index)[0]
}

fn declare(
    engine: &mut Engine<SyntheticLookup>,
    attackers: &[ObjectId],
) -> Result<(), EngineError> {
    engine.apply(
        ME,
        PlayerAction::DeclareAttackers {
            attackers: attackers
                .iter()
                .map(|a| (*a, Defender::Player(THEM)))
                .collect(),
        },
    )
}

// ------------------------------------------------------------ requirements

/// A creature that attacks each combat if able is named in the question,
/// and a declaration without it is refused while one with it is taken.
/// The vanilla creature beside it is offered and never required.
#[test]
fn a_creature_that_attacks_each_combat_if_able_must_attack() {
    let mut engine = combat(&[EAGER, BEAR], &[]);
    let eager = one(&engine, EAGER);
    let bear = one(&engine, BEAR);
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(attackers.contains(&eager) && attackers.contains(&bear));
    assert_eq!(required, vec![eager], "only the eager creature must attack");

    assert!(
        declare(&mut engine, &[]).is_err(),
        "attacking with nothing leaves a requirement unobeyed that could be obeyed (CR 508.1d)"
    );
    assert!(
        declare(&mut engine, &[bear]).is_err(),
        "attacking with only the other creature leaves it unobeyed too"
    );
    declare(&mut engine, &[eager]).expect("the eager creature alone obeys it");
    assert!(engine.state().combat.is_attacking(eager));
    assert!(!engine.state().combat.is_attacking(bear));
}

/// "If able": a tapped creature cannot attack, and asks nothing of the
/// declaration.
#[test]
fn a_tapped_creature_that_attacks_if_able_asks_nothing() {
    let mut engine = combat_after(&[EAGER, BEAR], &[], |engine| {
        let eager = one(engine, EAGER);
        engine
            .dev_state_mut(ME)
            .expect("seat 0 may seed")
            .set_tapped(eager, true);
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        required.is_empty(),
        "a tapped creature is not able to attack"
    );
    declare(&mut engine, &[]).expect("attacking with nothing is legal again");
}

/// The clock's answer to the question is the least the rules accept: the
/// creature that must attack, and nothing else.
#[test]
fn the_clocks_answer_attacks_with_what_must_attack() {
    let engine = combat(&[EAGER, BEAR], &[]);
    let eager = one(&engine, EAGER);
    let Some(PlayerAction::DeclareAttackers { attackers }) =
        crate::choice::timeout_answer(engine.pending())
    else {
        panic!("the clock answers an attack question")
    };
    assert_eq!(attackers, vec![(eager, Defender::Player(THEM))]);
}

// ------------------------------------------------------------ restrictions

/// "Can't attack unless defending player controls an Island": with no
/// Island across the table the creature is not offered and a declaration
/// naming it is refused; the vanilla creature beside it attacks as usual.
#[test]
fn a_creature_that_needs_an_island_across_the_table_cannot_attack_without_one() {
    let mut engine = combat(&[SEAFARER, BEAR], &[]);
    let seafarer = one(&engine, SEAFARER);
    let bear = one(&engine, BEAR);
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        !attackers.contains(&seafarer),
        "no Island, no attack (CR 508.1c)"
    );
    assert!(attackers.contains(&bear));
    assert!(declare(&mut engine, &[seafarer]).is_err());
    declare(&mut engine, &[bear]).expect("the bear attacks");
}

/// With an Island on the defending player's side the same creature is
/// offered and attacks.
#[test]
fn a_creature_that_needs_an_island_across_the_table_attacks_when_there_is_one() {
    let island = island();
    let mut engine = combat(&[SEAFARER], &[island]);
    let seafarer = one(&engine, SEAFARER);
    let Pending::ChooseAttackers {
        attackers, limits, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(attackers.contains(&seafarer));
    assert!(
        limits.is_empty(),
        "it may attack everything the question offers"
    );
    declare(&mut engine, &[seafarer]).expect("an Island across the table lets it attack");
    assert!(engine.state().combat.is_attacking(seafarer));
}

/// The Island has to be the defending player's: one on the attacker's own
/// side does nothing.
#[test]
fn an_island_of_ones_own_does_not_let_it_attack() {
    let island = island();
    let engine = combat(&[SEAFARER, island], &[]);
    let seafarer = one(&engine, SEAFARER);
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(!attackers.contains(&seafarer));
}
