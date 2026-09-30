//! Power and toughness counted off the board, in two shapes the plain
//! counts (`PtCount::YouControl`, `PtCount::OnBattlefield`) do not reach:
//! half a count, rounded down for power and up for toughness
//! (`Modifier::ModifyPTHalfCount`, Aspect of Wolf), and a count of what the
//! defending player controls while the counted creature attacks
//! (`PtCount::DefendingPlayerControls`, Gaea's Liege; CR 508.5).
//!
//! Played with permanents built for it beside real Forests.

use super::synthetic::{
    SyntheticLookup, creature_face, forest, keep_mulligans, permanents, preset_both, walk_past,
};
use super::*;
use crate::turn::{Phase, Step};
use crate::zone::Zone;
use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Condition, CounterKind, Coverage, FaceDef, Filter,
    KeywordSet, Modifier, PtCount, static_ability,
};
use baylee_core::color::ColorSet;
use baylee_core::generated::subtypes;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 1/1: "This creature gets +X/+Y, where X is half the number of Forests
/// you control, rounded down, and Y is half that number, rounded up."
const HALF: u32 = 1190;
/// A 1/1: "Creatures your opponents control get +X/+Y, where X is half the
/// number of Forests you control, rounded down, and Y is half that number,
/// rounded up." Whose Forests: the effect's controller's.
const HALF_FOR_THEM: u32 = 1191;
/// A 0/0 with Gaea's Liege's two sentences: "As long as this creature
/// isn't attacking, its power and toughness are each equal to the number
/// of Forests you control. As long as it is attacking, its power and
/// toughness are each equal to the number of Forests defending player
/// controls."
const LIEGE: u32 = 1192;
/// A 1/1: "This creature gets +X/+Y, where X is half the number of
/// Forests defending player controls, rounded down, and Y is half that
/// number, rounded up" — the count without a condition to swap it in, so
/// nothing but the attack itself changes what it reads.
const RAIDER: u32 = 1193;
/// A vanilla 2/2.
const BEAR: u32 = 1194;

static FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);
static THEIR_CREATURES: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);
static NOT_ATTACKING: Filter = Filter::Not(&Filter::Attacking);

static HALVING: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::ModifyPTHalfCount(PtCount::YouControl(&FOREST)),
)];
static HALVING_FOR_THEM: &[AbilityDef] = &[static_ability!(
    THEIR_CREATURES,
    Modifier::ModifyPTHalfCount(PtCount::YouControl(&FOREST)),
)];
static LIEGE_TEXT: &[AbilityDef] = &[
    static_ability!(
        Filter::This,
        Modifier::SetPTToCount(PtCount::YouControl(&FOREST)),
        condition = Some(Condition::SourceMatches(&NOT_ATTACKING)),
    ),
    static_ability!(
        Filter::This,
        Modifier::SetPTToCount(PtCount::DefendingPlayerControls(&FOREST)),
        condition = Some(Condition::SourceMatches(&Filter::Attacking)),
    ),
];
static RAIDING: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::ModifyPTHalfCount(PtCount::DefendingPlayerControls(&FOREST)),
)];

fn body(
    index: u32,
    name: &'static str,
    (power, toughness): (i16, i16),
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
        body(HALF, "Half", (1, 1), HALVING),
        body(HALF_FOR_THEM, "Half for them", (1, 1), HALVING_FOR_THEM),
        body(LIEGE, "Liege", (0, 0), LIEGE_TEXT),
        body(RAIDER, "Raider", (1, 1), RAIDING),
        body(BEAR, "Bear", (2, 2), &[]),
    ])
}

/// The pool's Jace, the Mind Sculptor: a planeswalker to attack.
fn jace() -> u32 {
    baylee_cards::by_oracle_id("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
        .expect("Jace exists")
        .index
        .get()
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn start(mine: &[u32], theirs: &[u32]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset_both(508, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    engine
}

fn with_forests(card: u32, n: usize) -> Vec<u32> {
    let mut cards = vec![card];
    cards.extend(std::iter::repeat_n(forest(), n));
    cards
}

fn permanent(engine: &Engine<SyntheticLookup>, seat: PlayerId, card: u32) -> ObjectId {
    permanents(engine, card)
        .into_iter()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat)
        })
        .expect("seated")
}

fn pt(engine: &Engine<SyntheticLookup>, id: ObjectId) -> (i16, i16) {
    let c = engine.state().object(id).expect("seated").characteristics();
    (c.power.unwrap_or(0), c.toughness.unwrap_or(0))
}

/// Walks turn 1 until `stop` holds, `ME` declaring `attackers` and nobody
/// blocking.
fn walk(
    engine: &mut Engine<SyntheticLookup>,
    attackers: &[(ObjectId, Defender)],
    stop: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..400 {
        if stop(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. }
                if player == ME && engine.state().turn.number == 1 =>
            {
                engine
                    .apply(
                        ME,
                        PlayerAction::DeclareAttackers {
                            attackers: attackers.to_vec(),
                        },
                    )
                    .unwrap();
            }
            other => assert!(
                walk_past(engine, &other),
                "an unexpected question: {other:?}"
            ),
        }
    }
    panic!("the game never got there");
}

/// `ME` holds priority in `step` of turn 1.
fn my_priority_in(step: Step) -> impl Fn(&Engine<SyntheticLookup>) -> bool {
    move |engine| {
        engine.state().turn.number == 1
            && engine.state().turn.step == step
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == ME)
    }
}

/// `ME` holds priority in turn 1's second main phase.
fn my_second_main(engine: &Engine<SyntheticLookup>) -> bool {
    engine.state().turn.number == 1
        && engine.state().turn.phase == Phase::SecondMain
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == ME)
}

// ------------------------------------------------------------ half a count

/// Half of an odd count is split, the smaller half to power: three Forests
/// give +1/+2, one gives +0/+1. Four give +2/+2, and none gives nothing.
#[test]
fn half_a_count_rounds_down_for_power_and_up_for_toughness() {
    for (forests, expected) in [(3, (2, 3)), (4, (3, 3)), (1, (1, 2)), (0, (1, 1))] {
        let engine = start(&with_forests(HALF, forests), &[]);
        let half = permanent(&engine, ME, HALF);
        assert_eq!(pt(&engine, half), expected, "{forests} Forests");
    }
}

/// "You" in the count is whoever controls the effect: the opponents' Bear
/// is pumped by half of *my* three Forests (+1/+2), not of its controller's
/// one (+0/+1).
#[test]
fn half_a_count_counts_the_effect_controller_s_forests() {
    let engine = start(&with_forests(HALF_FOR_THEM, 3), &with_forests(BEAR, 1));
    let bear = permanent(&engine, THEM, BEAR);
    assert_eq!(pt(&engine, bear), (3, 4));
}

// ------------------------------------------------- the defending player's

/// Gaea's Liege's two sentences: its controller's Forests while it stands
/// at home, the defending player's while it attacks, and its controller's
/// again once combat is over. Read in the declare attackers step, again
/// after blockers, and in the second main phase, so a pass that failed to
/// swap the two sentences shows; the damage it deals is the attacking
/// size.
#[test]
fn a_liege_counts_the_defending_player_s_forests_while_it_attacks() {
    let mut engine = start(&with_forests(LIEGE, 2), &with_forests(BEAR, 3));
    let liege = permanent(&engine, ME, LIEGE);
    let attack = [(liege, Defender::Player(THEM))];
    assert_eq!(pt(&engine, liege), (2, 2), "at home: my two Forests");

    walk(&mut engine, &attack, my_priority_in(Step::DeclareAttackers));
    assert_eq!(pt(&engine, liege), (3, 3), "attacking: their three");

    walk(&mut engine, &attack, my_priority_in(Step::DeclareBlockers));
    assert_eq!(pt(&engine, liege), (3, 3), "still attacking, unblocked");

    walk(&mut engine, &attack, my_second_main);
    assert_eq!(
        engine.state().players[THEM.get() as usize].life,
        20 - 3,
        "it dealt damage as a 3/3"
    );
    assert_eq!(pt(&engine, liege), (2, 2), "combat over: mine again");
}

/// Attacking a planeswalker, the defending player is its controller
/// (CR 508.5) — and still is once the planeswalker has left, because the
/// creature goes on attacking (CR 506.4c): the Liege keeps counting that
/// player's Forests rather than dying as a 0/0.
#[test]
fn a_liege_attacking_a_planeswalker_that_leaves_still_counts_its_controller_s_forests() {
    let mut engine = start(&with_forests(LIEGE, 2), &with_forests(jace(), 3));
    let liege = permanent(&engine, ME, LIEGE);
    let walker = permanent(&engine, THEM, jace());
    let attack = [(liege, Defender::Planeswalker(walker))];

    walk(&mut engine, &attack, my_priority_in(Step::DeclareAttackers));
    assert_eq!(
        pt(&engine, liege),
        (3, 3),
        "attacking Jace: its controller's three"
    );

    engine
        .dev_state_mut(ME)
        .unwrap()
        .object_mut(walker)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 0);
    engine.refresh_offer();
    walk(&mut engine, &attack, my_priority_in(Step::DeclareBlockers));
    assert_ne!(
        engine.state().object(walker).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Jace, out of loyalty, has left"
    );
    assert_eq!(
        engine.state().object(liege).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Liege did not die as a 0/0"
    );
    assert_eq!(pt(&engine, liege), (3, 3), "still their three Forests");
}

/// A creature that stays home is not attacking, so a defending player's
/// count is nothing to it: the Raider that does not attack stays 1/1 while
/// the one that does counts the defending player's four Forests (+2/+2).
/// No effect begins or ends as the attack is declared or as combat ends,
/// so this is the declaration itself telling the projection
/// (`GameState::board_state_changed`).
#[test]
fn a_count_of_the_defending_player_s_is_read_as_the_attack_is_declared() {
    let mut engine = start(&[RAIDER, RAIDER], &with_forests(BEAR, 4));
    let raiders = permanents(&engine, RAIDER);
    assert_eq!(raiders.len(), 2);
    assert_eq!(pt(&engine, raiders[0]), (1, 1), "nobody defends yet");
    let attack = [(raiders[0], Defender::Player(THEM))];

    walk(&mut engine, &attack, my_priority_in(Step::DeclareAttackers));
    assert_eq!(
        pt(&engine, raiders[0]),
        (3, 3),
        "attacking: half their four each"
    );
    assert_eq!(
        pt(&engine, raiders[1]),
        (1, 1),
        "at home: nobody it attacks"
    );

    walk(&mut engine, &attack, my_second_main);
    assert_eq!(pt(&engine, raiders[0]), (1, 1), "combat over: nobody again");
}
