//! What an ability is worth on the board it is offered on: the take and
//! the decline for each cause `worth` closed, each through
//! [`HeuristicAgent::act`] as a seat would meet it.
//!
//! The causes, and the trainer rows each stood behind:
//!
//! - **A use that is good or bad by its target or its cost.** The old
//!   whitelist took only untargeted gains, and a target was answered by the
//!   sign of the effect list: Wasteland, Maze of Ith, Recurring Nightmare,
//!   an equip, Homeward Path and Loran of the Third Path were never used.
//! - **A draw counted off the board.** A draw whose amount was not a plain
//!   number was refused, so Sea Gate Loremaster — and every copy of it — was
//!   never tapped.
//! - **A price paid next turn.** Pact of Negation was refused outright; it
//!   is cast when next turn's lands pay it and the spell it answers is worth
//!   that turn.
//! - **A deep profile's loyalty read without the board.** Venser's −1,
//!   Elspeth's 0 and Aminatou's −1 are worth a game or a creature on one
//!   board and nothing on another.

use super::*;
use crate::worth::{Aim, Origin};
use baylee_core::generated::subtypes::{creature as creature_type, land as land_type};
use baylee_core::types::SupertypeSet;
use baylee_engine::choice::TargetPrompt;
use baylee_engine::engine::DecisionContext;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// A land with no rules of its own, on `controller`'s side.
fn land(id: u32, controller: PlayerId) -> PublicObject {
    PublicObject {
        name: "Land".into(),
        types: TypeSet::LAND,
        power: None,
        toughness: None,
        ..permanent(obj(id), controller, 0)
    }
}

/// A basic Island.
fn island(id: u32, controller: PlayerId) -> PublicObject {
    let mut object = land(id, controller);
    object.name = "Island".into();
    object.supertypes = SupertypeSet::BASIC;
    object.subtypes.insert(land_type::ISLAND);
    object
}

/// A card of the pool on `controller`'s side, as the view shows it.
fn card(id: u32, controller: PlayerId, name: &str, types: TypeSet) -> PublicObject {
    let mut object = carded(permanent(obj(id), controller, 0), name, types);
    object.name = name.into();
    if !types.contains(TypeSet::CREATURE) {
        object.power = None;
        object.toughness = None;
    }
    object
}

/// A creature of `power`/`power` that is attacking `whom` unblocked.
fn attacking(v: &mut PlayerView, id: u32, controller: PlayerId, power: i16, whom: PlayerId) {
    let mut attacker = permanent(obj(id), controller, power);
    attacker.status = ObjectStatus::TAPPED;
    v.battlefield.push(attacker);
    v.combat.attackers.push(baylee_view::AttackerView {
        creature: obj(id),
        defending: Defender::Player(whom),
        blocked: false,
    });
}

/// This seat's own main phase.
fn main_phase(v: &mut PlayerView) {
    v.active = v.seat;
    v.phase = baylee_view::Phase::FirstMain;
    v.step = baylee_view::Step::Main;
}

/// The opponent's end step, where a tap costs this seat no blocker.
fn their_end_step(v: &mut PlayerView) {
    v.active = THEM;
    v.phase = baylee_view::Phase::Ending;
    v.step = baylee_view::Step::End;
}

/// The explanation the engine gives with the target question of `card`'s
/// ability `index`, activated from `source`.
fn asked(card: &str, index: u32, source: ObjectId) -> DecisionContext<'static> {
    let at = baylee_cards::decks::by_name(card).expect("a card of that name");
    let effects = match baylee_cards::by_index(at)
        .expect("a registered card")
        .abilities_for_face(0)
        .get(usize::try_from(index).unwrap())
    {
        Some(
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Loyalty { effects, .. },
        ) => *effects,
        _ => &[],
    };
    DecisionContext {
        source: Some(source),
        printed: baylee_engine::object::PrintedFace::new(at, 0),
        ability_index: Some(index),
        effects,
        ..Default::default()
    }
}

/// One target out of `options`.
fn one_of(options: Vec<ObjectId>) -> Pending {
    Pending::ChooseTargets {
        player: ME,
        options,
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    }
}

fn activates(id: u32, index: u32) -> PlayerAction {
    PlayerAction::ActivateAbility {
        source: obj(id),
        ability_index: index,
    }
}

fn names(id: u32) -> PlayerAction {
    PlayerAction::ChooseTargets {
        objects: vec![obj(id)],
        players: vec![],
    }
}

fn expert() -> HeuristicAgent {
    HeuristicAgent::new(AIProfile::EXPERT)
}

// --- A use that is good or bad by its target or its cost ------------------

/// Five lands of this seat's against six of theirs, one of them a Maze of
/// Ith: Wasteland is a land given up for their best land, and it names the
/// Maze over the plain dual listed before it.
#[test]
fn a_wasteland_is_spent_on_the_land_that_does_something() {
    let mut board = vec![card(1, ME, "Wasteland", TypeSet::LAND)];
    board.extend((2..6).map(|id| land(id, ME)));
    board.push(land(20, THEM));
    board.push(card(21, THEM, "Maze of Ith", TypeSet::LAND));
    board.extend((22..26).map(|id| land(id, THEM)));
    let mut v = view(0, &[20, 20], board);
    main_phase(&mut v);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        activates(1, 1)
    );
    assert_eq!(
        agent().act_with_context(
            &v,
            &one_of(vec![obj(20), obj(21)]),
            &asked("Wasteland", 1, obj(1))
        ),
        names(21),
        "the Maze, not the first land listed"
    );
}

/// The same Maze across the table from a seat on three lands: its own
/// Wasteland is a third of its mana, and it keeps it.
#[test]
fn a_wasteland_is_kept_while_its_seat_is_short_of_land() {
    let mut board = vec![card(1, ME, "Wasteland", TypeSet::LAND)];
    board.extend((2..4).map(|id| land(id, ME)));
    board.push(card(21, THEM, "Maze of Ith", TypeSet::LAND));
    board.extend((22..27).map(|id| land(id, THEM)));
    let mut v = view(0, &[20, 20], board);
    main_phase(&mut v);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        PlayerAction::PassPriority
    );
}

/// A 5/5 and a 1/1 attack this seat: Maze of Ith turns the 5/5 away.
#[test]
fn maze_of_ith_turns_away_the_attacker_that_would_hit_hardest() {
    let mut v = view(
        0,
        &[20, 20],
        vec![card(1, ME, "Maze of Ith", TypeSet::LAND)],
    );
    v.active = THEM;
    attacking(&mut v, 30, THEM, 1, ME);
    attacking(&mut v, 31, THEM, 5, ME);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        activates(1, 0)
    );
    assert_eq!(
        agent().act_with_context(
            &v,
            &one_of(vec![obj(30), obj(31)]),
            &asked("Maze of Ith", 0, obj(1))
        ),
        names(31)
    );
}

/// On this seat's own attack the Maze could only take its own damage away,
/// so it stays untapped.
#[test]
fn maze_of_ith_is_not_turned_on_its_own_attack() {
    let mut v = view(
        0,
        &[20, 20],
        vec![card(1, ME, "Maze of Ith", TypeSet::LAND)],
    );
    attacking(&mut v, 30, ME, 3, THEM);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

/// A 1/1 on the table and a 6/6 in the graveyard: Recurring Nightmare
/// trades up, and brings back the 6/6 rather than the 2/2 beside it.
#[test]
fn recurring_nightmare_trades_a_small_creature_for_a_big_one() {
    let mut nightmare = card(1, ME, "Recurring Nightmare", TypeSet::ENCHANTMENT);
    nightmare.mana_value = 3;
    let mut v = view(0, &[20, 20], vec![nightmare, permanent(obj(2), ME, 1)]);
    main_phase(&mut v);
    v.graveyards[0] = vec![permanent(obj(40), ME, 2), permanent(obj(41), ME, 6)];

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        activates(1, 0)
    );
    assert_eq!(
        agent().act_with_context(
            &v,
            &one_of(vec![obj(40), obj(41)]),
            &asked("Recurring Nightmare", 0, obj(1))
        ),
        names(41)
    );
}

/// A 6/6 on the table and a 2/2 in the graveyard is a trade down.
#[test]
fn recurring_nightmare_does_not_trade_down() {
    let mut nightmare = card(1, ME, "Recurring Nightmare", TypeSet::ENCHANTMENT);
    nightmare.mana_value = 3;
    let mut v = view(0, &[20, 20], vec![nightmare, permanent(obj(2), ME, 6)]);
    main_phase(&mut v);
    v.graveyards[0] = vec![permanent(obj(40), ME, 2)];

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

/// A 4/4 of this seat's fights for the opponent: Homeward Path brings it
/// home (CR 108.3).
#[test]
fn homeward_path_brings_home_what_was_stolen() {
    let mut stolen = permanent(obj(30), THEM, 4);
    stolen.owner = ME;
    let v = view(
        0,
        &[20, 20],
        vec![card(1, ME, "Homeward Path", TypeSet::LAND), stolen],
    );

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        activates(1, 1)
    );
}

/// Nothing stolen — or, as here, more stolen by this seat than from it —
/// and the Path stays a land.
#[test]
fn homeward_path_stays_a_land_while_it_would_give_more_back() {
    let mut mine = permanent(obj(30), THEM, 2);
    mine.owner = ME;
    let mut theirs = permanent(obj(31), ME, 5);
    theirs.owner = THEM;
    let v = view(
        0,
        &[20, 20],
        vec![card(1, ME, "Homeward Path", TypeSet::LAND), mine, theirs],
    );

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        PlayerAction::PassPriority
    );
}

/// "You and target opponent each draw a card" is a wash, until a draw of
/// theirs is a trigger of this seat's: Orcish Bowmasters pings for it.
#[test]
fn loran_draws_for_both_when_their_draw_is_punished() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            card(1, ME, "Loran of the Third Path", TypeSet::CREATURE),
            card(2, ME, "Orcish Bowmasters", TypeSet::CREATURE),
        ],
    );
    their_end_step(&mut v);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        activates(1, 1)
    );
}

/// Without one it is a card for them for every card for this seat.
#[test]
fn loran_does_not_hand_the_opponent_a_card_for_nothing() {
    let mut v = view(
        0,
        &[20, 20],
        vec![card(1, ME, "Loran of the Third Path", TypeSet::CREATURE)],
    );
    their_end_step(&mut v);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        PlayerAction::PassPriority
    );
}

/// Making a permanent an artifact does nothing on a board where no card
/// asks what is an artifact.
#[test]
fn liquimetal_coating_is_left_alone_where_no_card_asks() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            card(1, ME, "Liquimetal Coating", TypeSet::ARTIFACT),
            land(20, THEM),
            permanent(obj(21), THEM, 3),
        ],
    );
    main_phase(&mut v);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

/// An unattached Sword with the mana for its equip floating: it goes on,
/// and on the creature that can attack with it this turn.
#[test]
fn a_sword_is_equipped_to_the_creature_that_can_swing_with_it() {
    let mut sick = permanent(obj(3), ME, 2);
    sick.summoning_sick = true;
    let mut v = view(
        0,
        &[20, 20],
        vec![
            card(1, ME, "Sword of Hearth and Home", TypeSet::ARTIFACT),
            permanent(obj(2), ME, 2),
            sick,
        ],
    );
    main_phase(&mut v);
    v.seats[0].mana_pool.colorless = 2;

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 4)])),
        activates(1, 4)
    );
    assert_eq!(
        agent().act_with_context(
            &v,
            &one_of(vec![obj(3), obj(2)]),
            &asked("Sword of Hearth and Home", 4, obj(1))
        ),
        names(2)
    );
}

/// On the only creature there is, moving it again is mana for nothing.
#[test]
fn a_sword_is_not_moved_onto_the_creature_already_wearing_it() {
    let mut sword = card(1, ME, "Sword of Hearth and Home", TypeSet::ARTIFACT);
    sword.attached_to = Some(obj(2));
    let mut v = view(0, &[20, 20], vec![sword, permanent(obj(2), ME, 2)]);
    main_phase(&mut v);
    v.seats[0].mana_pool.colorless = 2;

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 4)])),
        PlayerAction::PassPriority
    );
}

// --- A draw counted off the board ------------------------------------------

/// Sea Gate Loremaster and two more Allies: three cards, and the same for
/// a Sakashima that entered as a copy of the Loremaster (CR 707.2).
#[test]
fn sea_gate_loremaster_draws_one_for_each_ally_and_so_does_its_copy() {
    let ally = |mut object: PublicObject| {
        object.subtypes.insert(creature_type::ALLY);
        object
    };
    let allies = || {
        vec![
            ally(card(2, ME, "Kazandu Blademaster", TypeSet::CREATURE)),
            ally(card(3, ME, "Kazandu Blademaster", TypeSet::CREATURE)),
        ]
    };
    let loremaster = ally(card(1, ME, "Sea Gate Loremaster", TypeSet::CREATURE));
    let sakashima = ally(copying(
        card(1, ME, "Sakashima of a Thousand Faces", TypeSet::CREATURE),
        "Sea Gate Loremaster",
    ));
    for (source, what) in [(loremaster, "the Loremaster"), (sakashima, "its copy")] {
        let mut board = allies();
        board.push(source);
        let mut v = view(0, &[20, 20], board);
        their_end_step(&mut v);
        assert_eq!(
            agent().act(&v, &offering(vec![(obj(1), 0)])),
            activates(1, 0),
            "{what} is tapped to draw"
        );
    }
}

/// Three Allies and three cards left: the draw would be the last three, and
/// the next draw step would lose the game (CR 704.5b).
#[test]
fn sea_gate_loremaster_does_not_draw_the_library_out() {
    let ally = |mut object: PublicObject| {
        object.subtypes.insert(creature_type::ALLY);
        object
    };
    let mut v = view(
        0,
        &[20, 20],
        vec![
            ally(card(1, ME, "Sea Gate Loremaster", TypeSet::CREATURE)),
            ally(card(2, ME, "Kazandu Blademaster", TypeSet::CREATURE)),
            ally(card(3, ME, "Kazandu Blademaster", TypeSet::CREATURE)),
        ],
    );
    their_end_step(&mut v);
    v.seats[0].library_count = 3;

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

// --- A price paid next turn ------------------------------------------------

/// An opponent's six-drop on the stack, Pact of Negation in hand and five
/// Islands that untap before the pact's upkeep: it is countered.
fn pact_table(islands: u32, threat: PublicObject) -> (PlayerView, Pending) {
    let mut v = view(
        0,
        &[20, 20],
        (1..=islands).map(|id| island(id, ME)).collect(),
    );
    v.active = THEM;
    v.phase = baylee_view::Phase::FirstMain;
    v.step = baylee_view::Step::Main;
    v.stack.push(threat);
    v.hand = vec![hand_card(50, "Pact of Negation")];
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(50)],
            ..Default::default()
        }),
    };
    (v, pending)
}

fn six_drop() -> PublicObject {
    let mut spell = permanent(obj(60), THEM, 6);
    spell.mana_value = 6;
    spell.stack_item = Some(baylee_view::StackItem::Spell);
    spell
}

#[test]
fn a_pact_is_cast_when_next_turn_pays_it_and_the_spell_is_worth_that_turn() {
    let (v, pending) = pact_table(5, six_drop());
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::CastSpell { card: obj(50) }
    );
}

/// Three Islands cannot pay {3}{U}{U} next upkeep, and a Lightning Bolt at
/// the opponent's own creature is not worth a turn's mana even when five
/// can.
#[test]
fn a_pact_is_not_cast_when_its_price_loses_or_outweighs_the_spell() {
    let (v, pending) = pact_table(3, six_drop());
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "three Islands"
    );

    let bolt = stack_spell(60, "Lightning Bolt", THEM, obj(61));
    let (mut v, pending) = pact_table(5, bolt);
    v.battlefield.push(permanent(obj(61), THEM, 2));
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "a Bolt at their own creature"
    );
}

/// A pact's price (a delayed trigger, CR 603.7) is demanded by the engine
/// once a round of passes on an empty stack closes the upkeep
/// (`upkeep_payments`), from the lands this seat has then. Nothing in the view says one is owed, so this seat spends
/// nothing in its own upkeep on an empty stack — here a fetchland, which
/// the draw step cracks as before.
#[test]
fn a_seat_spends_nothing_in_its_own_upkeep() {
    let mut v = view(0, &[20, 20], vec![card(1, ME, "Arid Mesa", TypeSet::LAND)]);
    v.phase = baylee_view::Phase::Beginning;
    v.step = baylee_view::Step::Upkeep;
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority,
        "in the upkeep"
    );

    v.step = baylee_view::Step::Draw;
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        activates(1, 0),
        "in the draw step"
    );
}

// --- A deep profile's loyalty read with the board --------------------------

/// Three 3/3s into three 3/3 blockers at nine life: Venser's −1 makes the
/// attack lethal, and a deep profile takes it over the +2.
#[test]
fn venser_makes_the_team_unblockable_for_the_kill() {
    let mut board = vec![carded(
        walker(obj(1), ME, 3),
        "Venser, the Sojourner",
        TypeSet::PLANESWALKER,
    )];
    board.extend((2..5).map(|id| permanent(obj(id), ME, 3)));
    board.extend((20..23).map(|id| permanent(obj(id), THEM, 3)));
    let mut v = view(0, &[20, 9], board);
    main_phase(&mut v);

    assert_eq!(
        expert().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1)])),
        activates(1, 1)
    );
}

/// With no creature of this seat's, "creatures can't be blocked" is a
/// loyalty spent on nothing: the +2 instead.
#[test]
fn venser_does_not_spend_loyalty_on_an_attack_nobody_makes() {
    let v = {
        let mut v = view(
            0,
            &[20, 9],
            vec![
                carded(
                    walker(obj(1), ME, 3),
                    "Venser, the Sojourner",
                    TypeSet::PLANESWALKER,
                ),
                land(5, ME),
            ],
        );
        main_phase(&mut v);
        v
    };

    assert_eq!(
        expert().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1)])),
        activates(1, 0)
    );
}

/// Two 3/3s against two 4/4 ground blockers at six life: Elspeth's 0 gives
/// them flying and the attack is lethal.
#[test]
fn elspeth_takes_to_the_air_for_the_kill() {
    let mut v = view(
        0,
        &[20, 6],
        vec![
            carded(
                walker(obj(1), ME, 5),
                "Elspeth, Storm Slayer",
                TypeSet::PLANESWALKER,
            ),
            permanent(obj(2), ME, 3),
            permanent(obj(3), ME, 3),
            permanent(obj(20), THEM, 4),
            permanent(obj(21), THEM, 4),
        ],
    );
    main_phase(&mut v);

    assert_eq!(
        expert().act(&v, &offering(vec![(obj(1), 1), (obj(1), 2), (obj(1), 3)])),
        activates(1, 2)
    );
}

/// With no creature of this seat's to put counters on or lift, the 0 does
/// nothing and is not what Elspeth does.
#[test]
fn elspeth_does_not_lift_an_army_she_does_not_have() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            carded(
                walker(obj(1), ME, 5),
                "Elspeth, Storm Slayer",
                TypeSet::PLANESWALKER,
            ),
            permanent(obj(20), THEM, 4),
        ],
    );
    main_phase(&mut v);

    assert_ne!(
        expert().act(&v, &offering(vec![(obj(1), 1), (obj(1), 2), (obj(1), 3)])),
        activates(1, 2)
    );
}

/// A 4/4 this seat owns fights for the opponent: Aminatou's −1 flickers it
/// home as a new object (CR 400.7), and names it rather than this seat's
/// own land.
#[test]
fn aminatou_flickers_home_what_was_stolen() {
    let mut stolen = permanent(obj(30), THEM, 4);
    stolen.owner = ME;
    let mut v = view(
        0,
        &[20, 20],
        vec![
            carded(
                walker(obj(1), ME, 3),
                "Aminatou, the Fateshifter",
                TypeSet::PLANESWALKER,
            ),
            land(5, ME),
            stolen,
        ],
    );
    main_phase(&mut v);

    assert_eq!(
        expert().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1)])),
        activates(1, 1)
    );
    assert_eq!(
        expert().act_with_context(
            &v,
            &one_of(vec![obj(5), obj(30)]),
            &asked("Aminatou, the Fateshifter", 1, obj(1))
        ),
        names(30)
    );
}

/// Nothing stolen and nothing that does anything as it enters: the −1 is a
/// loyalty spent on a flicker that changes nothing, and the +1 is taken.
#[test]
fn aminatou_does_not_flicker_what_gains_nothing_by_it() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            carded(
                walker(obj(1), ME, 3),
                "Aminatou, the Fateshifter",
                TypeSet::PLANESWALKER,
            ),
            land(5, ME),
            permanent(obj(6), ME, 2),
        ],
    );
    main_phase(&mut v);

    assert_eq!(
        expert().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1)])),
        activates(1, 0)
    );
}

// --- What the table reads ---------------------------------------------------

/// How many of the pool's activated and loyalty abilities the table reads,
/// as `(read, of)`, on an empty board.
///
/// An unread ability is one the house leaves alone (a deep profile's
/// loyalty falls back to `effect_value`). The floor catches a row lost; the
/// ceiling catches a wildcard that would read everything as worth
/// something, which is the one change that would make every unread ability
/// a guess.
#[test]
fn the_table_reads_most_of_what_the_pool_activates() {
    let v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let (mut read, mut of) = (0usize, 0usize);
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                let effects = match ability {
                    AbilityDef::Activated {
                        effects,
                        mana_ability: false,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        effects,
                        mana_ability: false,
                        ..
                    }
                    | AbilityDef::Loyalty { effects, .. } => *effects,
                    _ => continue,
                };
                of += 1;
                if agent()
                    .effects_worth(&v, origin, effects, Aim::Source, 0)
                    .is_some()
                {
                    read += 1;
                }
            }
        }
    }
    // 878 of 1010 when the table was written, and 950 with a wildcard arm
    // in place of the table's `None`. A share and not a count, because card
    // batches grow the pool under it; the population has its own floor and
    // ceiling so a walk that misses faces, or counts one twice, cannot pass
    // by moving both numbers together.
    eprintln!("the table reads {read} of {of} activated and loyalty abilities");
    assert!((900..=1500).contains(&of), "{of} abilities in the pool");
    let share = read * 100 / of;
    assert!((80..=92).contains(&share), "{read} of {of} read");

    // And one sentence it has no row for stays unread: Aminatou's −6
    // exchanges every nontoken permanent's controller around the table,
    // which is worth what the table looks like after, and nothing here
    // works that out.
    let aminatou = baylee_cards::decks::by_name("Aminatou, the Fateshifter").unwrap();
    let Some(AbilityDef::Loyalty { effects, .. }) = baylee_cards::by_index(aminatou)
        .unwrap()
        .abilities_for_face(0)
        .get(2)
    else {
        panic!("Aminatou's third ability is her −6");
    };
    assert_eq!(
        agent().effects_worth(&v, origin, effects, Aim::Source, 0),
        None
    );
}

/// A clause inside a sequence or behind a price is still read.
///
/// The whitelist this table replaced had to be taught both (#109, Sensei's
/// Divining Top's draw inside a `Sequence`, a draw behind "unless you pay"
/// in `PlayerMayPayOr`, which carries one effect rather than a list), and
/// four tests pinned the descent there. Here it is `Effect::branches`, and
/// a draw that may happen still counts against the library: half of a
/// deck-out is still a refusal. Synthetic on purpose, as those were: no
/// activated ability in the pool prints the priced draw.
#[test]
fn a_clause_in_a_sequence_or_behind_a_price_is_read() {
    use baylee_cards_dsl::{Amount, Effect, PlayerRel};
    static DRAW: Effect = Effect::DrawCards {
        amount: Amount::Fixed(1),
    };
    static IN_SEQUENCE: [Effect; 1] = [Effect::DrawCards {
        amount: Amount::Fixed(1),
    }];
    let sequence = [Effect::Sequence(&IN_SEQUENCE)];
    let priced = [Effect::PlayerMayPayOr {
        player: PlayerRel::Opponent,
        mana: Amount::Fixed(1),
        effect: &DRAW,
    }];
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let worth = |v: &PlayerView, effects: &[Effect]| {
        agent().effects_worth(v, origin, effects, Aim::Source, 0)
    };

    let v = view(0, &[20, 20], vec![]);
    assert!(
        worth(&v, &sequence).is_some_and(|w| w > 0),
        "the draw inside the sequence was missed"
    );
    assert!(
        worth(&v, &priced).is_some_and(|w| w > 0),
        "the draw behind the price was missed"
    );

    let mut short = v;
    short.seats[0].library_count = 1;
    assert!(
        worth(&short, &priced).is_some_and(|w| w < 0),
        "a draw this seat may have to take from a one-card library is not a gain"
    );
}

// --- Loops the house must not make (self-play mac-d001) --------------------

/// A creature of the pool on this seat's side, at `power`/`toughness`.
fn body(id: u32, name: &str, power: i16, toughness: i16) -> PublicObject {
    let mut object = card(id, ME, name, TypeSet::CREATURE);
    object.power = Some(power);
    object.toughness = Some(toughness);
    object
}

/// Ability `index` of the permanent `source`, waiting on the stack.
fn waiting(id: u32, source: &PublicObject, index: u32) -> PublicObject {
    let mut ability = permanent(obj(id), source.controller, 0);
    ability.types = TypeSet::EMPTY;
    ability.power = None;
    ability.toughness = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: source.id,
        ability: source
            .rules
            .map(|r| baylee_core::ids::AbilityRef::new(r.card, index)),
        text: None,
        rules: source.rules,
    });
    ability
}

/// Flowstone Hellion's `{0}: +1/+0 -1/-0` is the lethal attack on an
/// empty stack. With one activation already waiting it is passed: the view
/// shows nothing of an ability before it resolves, so a second one would
/// be judged on the same board and taken again, and again — thousands of
/// times in one priority round, until the game ran into its cap.
#[test]
fn an_ability_waiting_on_the_stack_is_not_activated_again() {
    let hellion = body(1, "Flowstone Hellion", 3, 3);
    let mut v = view(0, &[20, 4], vec![hellion.clone()]);
    main_phase(&mut v);
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        activates(1, 0),
        "one pump makes the attack lethal"
    );

    v.stack.push(waiting(5, &hellion, 0));
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority,
        "the pump already on the stack resolves first"
    );
}

/// Taken one resolution at a time, the Hellion's pump stops where the next
/// one would take its toughness to nothing: a 6/0 attacks nobody.
#[test]
fn a_free_pump_stops_before_it_kills_its_own_creature() {
    let mut v = view(0, &[20, 6], vec![body(1, "Flowstone Hellion", 5, 1)]);
    main_phase(&mut v);
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

/// Shuko's `{0}` equip on one of two creatures that can both attack: moving
/// it is worth nothing, since the creature it leaves could have swung with
/// it as well. Counting only where it went, the house moved it back and
/// forth between the two for ever. Onto a bare creature it still goes.
#[test]
fn a_free_equip_does_not_move_between_two_creatures_as_good() {
    let mut shuko = card(1, ME, "Shuko", TypeSet::ARTIFACT);
    shuko.attached_to = Some(obj(2));
    let mut v = view(
        0,
        &[20, 20],
        vec![shuko, permanent(obj(2), ME, 2), permanent(obj(3), ME, 2)],
    );
    main_phase(&mut v);
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );

    v.battlefield[0].attached_to = None;
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        activates(1, 0),
        "an unattached Shuko is still worn"
    );
}

/// Shifting Wall with no mana to spend is a 0/0 at the only X the seat can
/// pay, and dies as it arrives (CR 704.5f); cast again every time it came
/// back, it held a game in one main phase. With mana it is a wall.
#[test]
fn an_x_creature_is_not_cast_for_an_x_of_nothing() {
    let mut v = view(0, &[20, 20], vec![]);
    main_phase(&mut v);
    v.hand.push(crate::tests::hand_card(1, "Shifting Wall"));
    let castable = || Pending::Priority {
        player: ME,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(1)],
            ..Default::default()
        }),
    };
    assert_eq!(agent().act(&v, &castable()), PlayerAction::PassPriority);

    v.seats[0].mana_pool.colorless = 3;
    assert_eq!(
        agent().act(&v, &castable()),
        PlayerAction::CastSpell { card: obj(1) }
    );
}
