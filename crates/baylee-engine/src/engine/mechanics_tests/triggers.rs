//! Three trigger events the pool prints and no rule test played:
//! `Trigger::AttacksAlone`, `Trigger::DealsCombatDamageToPlayer` and
//! `Trigger::DrawsExceptFirst`.
//!
//! Each is a matcher that has to say no far more often than yes — to the
//! second attacker, to the blocked attacker and the burn spell, to the first
//! card of a draw step — and the tests are mostly about the no.

use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, Trigger};
use baylee_core::ids::Defender;

const CATHEDRAL: u32 = 7100;
const SCOUT: u32 = 7101;
const RAIDER: u32 = 7102;
const SQUIRE: u32 = 7103;
const WALL: u32 = 7104;
const BOWMASTER: u32 = 7105;
const STACKS: u32 = 7106;

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];
static YOUR_CREATURE: Filter = Filter::YOUR_CREATURE;
static THIS: Filter = Filter::This;

/// Exalted's trigger (CR 702.83a) with a life point for its effect:
/// "Whenever a creature you control attacks alone, you gain 1 life."
static CATHEDRAL_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::AttacksAlone(&YOUR_CREATURE),
    GAIN_ONE
)];

/// "Whenever this creature deals combat damage to a player, you gain 1
/// life", and "{0}: this creature deals 1 damage to each opponent" — the
/// same source dealing the same damage to the same player, outside combat.
static RAIDER_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::triggered!(Trigger::DealsCombatDamageToPlayer(&THIS), GAIN_ONE),
    free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }],
        None,
    ),
];

/// Orcish Bowmasters' second trigger with a life point for its effect:
/// "Whenever an opponent draws a card except the first one they draw in each
/// of their draw steps, you gain 1 life."
static BOWMASTER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::DrawsExceptFirst(PlayerRel::Opponent),
    GAIN_ONE
)];

/// "{0}: Draw a card."
static STACKS_ABILITIES: &[AbilityDef] = &[free(&[Effect::draw(1)], None)];

fn cards() -> Vec<&'static CardDef> {
    let land = super::super::synthetic::land;
    vec![
        land(CATHEDRAL, "Cathedral", CATHEDRAL_ABILITIES),
        card(
            SCOUT,
            creature_face("Scout", "{1}", 1, 1),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            RAIDER,
            creature_face("Raider", "{1}", 2, 2),
            KeywordSet::HASTE,
            RAIDER_ABILITIES,
        ),
        card(
            SQUIRE,
            creature_face("Squire", "{1}", 1, 1),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            WALL,
            creature_face("Wall", "{1}", 0, 4),
            KeywordSet::EMPTY,
            &[],
        ),
        land(BOWMASTER, "Bowmaster", BOWMASTER_ABILITIES),
        land(STACKS, "Stacks", STACKS_ABILITIES),
    ]
}

/// Walks to `seat`'s declaration of attackers and declares `attackers`
/// against the other seat.
#[track_caller]
fn attack(engine: &mut Bench, seat: PlayerId, attackers: &[ObjectId]) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let Pending::ChooseAttackers {
        attackers: offered, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    for a in attackers {
        assert!(offered.contains(a), "{a:?} may attack: {offered:?}");
    }
    let defender = if seat == me() { them() } else { me() };
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("the attack the engine offered");
}

/// Walks until combat is over on the active player's turn.
#[track_caller]
fn finish_combat(engine: &mut Bench) {
    let seat = engine.state().turn.active;
    walk_until(engine, |e| {
        holds_priority_in(e, seat, crate::turn::Step::Main)
    });
}

// ------------------------------------------------------------ attacks alone

/// "Attacks alone" is the only creature declared as an attacker
/// (CR 506.5): one Scout alone triggers, and two together trigger nothing.
#[test]
fn attacking_alone_triggers_and_attacking_beside_another_does_not() {
    for together in [false, true] {
        let mut engine = bench(
            7100,
            cards(),
            [Seat::with(&[CATHEDRAL, SCOUT, SCOUT]), Seat::default()],
        );
        let cathedral = the(&engine, ZoneLocation::Battlefield, CATHEDRAL);
        let scouts = objects(&engine, ZoneLocation::Battlefield, SCOUT);
        let start = life(&engine, me());
        let attackers = if together { &scouts[..] } else { &scouts[..1] };

        attack(&mut engine, me(), attackers);
        finish_combat(&mut engine);

        let expected = usize::from(!together);
        assert_eq!(
            triggered(&engine, cathedral),
            expected,
            "{} attacker(s)",
            attackers.len()
        );
        assert_eq!(life(&engine, me()), start + expected as i32);
    }
}

/// The filter is asked of the creature that attacked alone: a creature the
/// Cathedral's controller does not control attacks alone, and only its own
/// controller's Cathedral answers.
#[test]
fn a_lone_attacker_the_filter_does_not_name_triggers_nothing() {
    let mut engine = bench(
        7101,
        cards(),
        [Seat::with(&[CATHEDRAL]), Seat::with(&[CATHEDRAL, SCOUT])],
    );
    let [a, b] = objects(&engine, ZoneLocation::Battlefield, CATHEDRAL)[..] else {
        panic!("two Cathedrals")
    };
    let controller = |id: ObjectId| engine.state().object(id).map(|o| o.controller);
    let (mine, theirs) = if controller(a) == Some(me()) {
        (a, b)
    } else {
        (b, a)
    };
    assert_eq!(controller(theirs), Some(them()));
    let scout = the(&engine, ZoneLocation::Battlefield, SCOUT);

    to_main(&mut engine, them());
    attack(&mut engine, them(), &[scout]);
    finish_combat(&mut engine);

    assert_eq!(
        triggered(&engine, theirs),
        1,
        "their creature, their trigger"
    );
    assert_eq!(triggered(&engine, mine), 0, "not a creature I control");
}

// --------------------------------------------- combat damage to a player

/// Unblocked, the Raider's damage is combat damage to a player and it
/// triggers — once, for its own damage and not for the Squire's beside it
/// (the filter is "this creature").
#[test]
fn combat_damage_to_a_player_triggers_for_the_source_it_names() {
    let mut engine = bench(
        7102,
        cards(),
        [Seat::with(&[RAIDER, SQUIRE]), Seat::default()],
    );
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);
    let squire = the(&engine, ZoneLocation::Battlefield, SQUIRE);
    let (mine, theirs) = (life(&engine, me()), life(&engine, them()));

    attack(&mut engine, me(), &[raider, squire]);
    finish_combat(&mut engine);

    assert_eq!(life(&engine, them()), theirs - 3, "both were unblocked");
    assert_eq!(triggered(&engine, raider), 1, "the Raider's damage, once");
    assert_eq!(life(&engine, me()), mine + 1);
}

/// Blocked, the Raider deals its combat damage to a creature (CR 510.1c),
/// and a creature is not a player.
#[test]
fn combat_damage_to_a_blocker_triggers_nothing() {
    let mut engine = bench(7103, cards(), [Seat::with(&[RAIDER]), Seat::with(&[WALL])]);
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);
    let wall = the(&engine, ZoneLocation::Battlefield, WALL);
    let theirs = life(&engine, them());

    attack(&mut engine, me(), &[raider]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, raider)],
            },
        )
        .expect("the Wall may block the Raider");
    finish_combat(&mut engine);

    assert_eq!(life(&engine, them()), theirs, "the Wall took it");
    assert_eq!(
        engine.state().object(wall).map(|o| o.damage),
        Some(2),
        "all of it"
    );
    assert_eq!(triggered(&engine, raider), 0);
}

/// Damage the same creature deals to the same player outside combat is
/// damage dealt by an effect (CR 120.2b), not combat damage (CR 120.2a).
#[test]
fn noncombat_damage_to_a_player_triggers_nothing() {
    let mut engine = bench(7104, cards(), [Seat::with(&[RAIDER]), Seat::default()]);
    to_main(&mut engine, me());
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);
    let theirs = life(&engine, them());

    activate(&mut engine, me(), raider, 1);
    settle(&mut engine, me());

    assert_eq!(life(&engine, them()), theirs - 1, "the Raider dealt it");
    assert_eq!(triggered(&engine, raider), 0);
}

// ----------------------------------------------------- draws except first

/// Seat 1's first card in its draw step (CR 504.1) is the exception; a
/// second card in the same step is not, and neither is a card drawn on
/// somebody else's turn.
#[test]
fn an_opponents_draws_trigger_except_the_first_of_their_draw_step() {
    let mut engine = bench(
        7105,
        cards(),
        [Seat::with(&[BOWMASTER]), Seat::with(&[STACKS])],
    );
    let bowmaster = the(&engine, ZoneLocation::Battlefield, BOWMASTER);
    let stacks = the(&engine, ZoneLocation::Battlefield, STACKS);
    let start = life(&engine, me());

    to_main(&mut engine, me());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), stacks, 0);
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, bowmaster),
        1,
        "a card drawn on my turn is in nobody's draw step of theirs"
    );

    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Draw)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(them())).len(),
        2,
        "the draw step's card is in hand"
    );
    assert_eq!(
        triggered(&engine, bowmaster),
        1,
        "and it was the first of their draw step"
    );

    activate(&mut engine, them(), stacks, 0);
    settle(&mut engine, them());
    assert_eq!(
        triggered(&engine, bowmaster),
        2,
        "the second card of the same draw step"
    );
    assert_eq!(life(&engine, me()), start + 2);
}

/// "An opponent": its controller's own draws, in and out of its draw step,
/// trigger nothing.
#[test]
fn its_controllers_own_draws_trigger_nothing() {
    let mut engine = bench(
        7106,
        cards(),
        [Seat::with(&[BOWMASTER, STACKS]), Seat::default()],
    );
    let bowmaster = the(&engine, ZoneLocation::Battlefield, BOWMASTER);
    let stacks = the(&engine, ZoneLocation::Battlefield, STACKS);

    to_main(&mut engine, me());
    activate(&mut engine, me(), stacks, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), stacks, 0);
    settle(&mut engine, me());
    walk_until(&mut engine, |e| {
        e.state().turn.number == 3 && holds_priority_in(e, me(), crate::turn::Step::Main)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        3,
        "two cards drawn on turn one and one in my own draw step"
    );
    activate(&mut engine, me(), stacks, 0);
    settle(&mut engine, me());

    assert_eq!(triggered(&engine, bowmaster), 0);
}
