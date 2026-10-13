//! `Effect::CreateTokenAttacking` and `Effect::WheneverYouAttackThisTurn`:
//! tokens put onto the battlefield tapped and attacking (CR 508.4), and a
//! delayed "whenever you attack this turn" (Dalkovan Encampment), played by
//! cards nobody printed.
//!
//! The boards tell each reading from its wrong twin: the token attacks (its
//! damage lands) but never "attacked" (an attack trigger does not hear it,
//! CR 508.3a); at a table of three its controller chooses whom it attacks;
//! made outside combat it only enters tapped (CR 506.3b); the delayed
//! trigger fires on each declaration of the turn and on none of the next.

use super::*;
use baylee_cards_dsl::{Amount, Filter, Trigger};
use baylee_core::ids::Defender;

const CAPTAIN: u32 = 7400;
const DRILL_HALL: u32 = 7401;
const SENTRY: u32 = 7402;
const BUGLER: u32 = 7403;

static RECRUIT: baylee_cards_dsl::TokenDef = baylee_cards_dsl::TokenDef {
    name: "Recruit",
    types: TypeSet::CREATURE,
    power: Some(1),
    toughness: Some(1),
    ..baylee_cards_dsl::TokenDef::DEFAULT
};

/// "Whenever this creature attacks, create a 1/1 Recruit token that's
/// tapped and attacking."
static CAPTAIN_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::CreateTokenAttacking {
        token: &RECRUIT,
        amount: Amount::Fixed(1),
        sacrifice_at_next_end_step: false,
    }]
)];

/// 0: "{0}: Whenever you attack this turn, create two 1/1 Recruit tokens
/// that are tapped and attacking. Sacrifice them at the beginning of the
/// next end step." 1: "{0}: Create a 1/1 Recruit token that's tapped and
/// attacking" (for the outside-combat case).
static DRILL_HALL_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::WheneverYouAttackThisTurn {
            effects: &[Effect::CreateTokenAttacking {
                token: &RECRUIT,
                amount: Amount::Fixed(2),
                sacrifice_at_next_end_step: true,
            }],
        }],
        None,
    ),
    free(
        &[Effect::CreateTokenAttacking {
            token: &RECRUIT,
            amount: Amount::Fixed(1),
            sacrifice_at_next_end_step: false,
        }],
        None,
    ),
];

/// "Whenever a creature attacks, you gain 1 life": counts what attacked.
static BUGLER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Attacks(&Filter::CREATURE),
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }]
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            CAPTAIN,
            creature_face("Captain", "{1}", 2, 2),
            KeywordSet::HASTE,
            CAPTAIN_ABILITIES,
        ),
        card(
            DRILL_HALL,
            face("Drill Hall", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DRILL_HALL_ABILITIES,
        ),
        card(
            SENTRY,
            creature_face("Sentry", "{1}", 1, 1),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            BUGLER,
            face("Bugler", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BUGLER_ABILITIES,
        ),
    ]
}

/// The Recruit tokens on the battlefield (the only tokens these boards make).
fn recruits(engine: &Bench) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).is_some_and(|o| o.card.is_none()))
        .collect()
}

#[track_caller]
fn attack(engine: &mut Bench, seat: PlayerId, attackers: &[ObjectId], defender: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
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
        .expect("the attack is legal");
}

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

/// Two seats. The Captain attacks; its trigger makes a Recruit that is
/// tapped and attacking the only defending player, without a question. The
/// Bugler, which hears every creature that attacks, hears the Captain and
/// not the Recruit (CR 508.3a), and both deal their combat damage.
#[test]
fn a_token_put_onto_the_battlefield_attacking_attacks_but_never_attacked() {
    let mut engine = bench(
        7400,
        cards(),
        [Seat::with(&[CAPTAIN, BUGLER]), Seat::with(&[])],
    );
    let captain = the(&engine, ZoneLocation::Battlefield, CAPTAIN);
    let (my_life, their_life) = (life(&engine, me()), life(&engine, them()));

    attack(&mut engine, me(), &[captain], them());
    walk_until(&mut engine, |e| {
        e.state().zones.stack_is_empty() && !recruits(e).is_empty()
    });
    let recruit = recruits(&engine)[0];
    assert!(is_tapped(&engine, recruit), "it enters tapped");
    assert_eq!(
        engine.state().combat.defender_of(recruit),
        Some(Defender::Player(them())),
        "and attacking the defending player"
    );
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        life(&engine, me()),
        my_life + 1,
        "the Bugler heard one creature attack, the Captain"
    );
    assert_eq!(
        life(&engine, them()),
        their_life - 3,
        "the Captain's 2 and the Recruit's 1"
    );
}

/// Three seats. The Captain attacks the second seat; its controller is asked
/// which player the Recruit attacks, among both opponents, and sends it at
/// the third seat, who takes its damage.
#[test]
fn at_a_table_of_three_the_controller_chooses_whom_the_token_attacks() {
    let third = PlayerId::new(2);
    let mut engine = bench(
        7401,
        cards(),
        [Seat::with(&[CAPTAIN]), Seat::with(&[]), Seat::with(&[])],
    );
    let captain = the(&engine, ZoneLocation::Battlefield, CAPTAIN);
    let (second_life, third_life) = (life(&engine, them()), life(&engine, third));

    attack(&mut engine, me(), &[captain], them());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, me(), "the token's controller chooses");
    assert_eq!(options, vec![them(), third], "either defending player");
    engine
        .apply(me(), PlayerAction::ChoosePlayer(third))
        .expect("an offered player is taken");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(life(&engine, them()), second_life - 2, "the Captain's 2");
    assert_eq!(life(&engine, third), third_life - 1, "the Recruit's 1");
}

/// Made in a main phase, the Recruit only enters tapped (CR 506.3b): no
/// combat, nothing to attack.
#[test]
fn a_token_made_outside_combat_only_enters_tapped() {
    let mut engine = bench(7402, cards(), [Seat::with(&[DRILL_HALL]), Seat::with(&[])]);
    let hall = the(&engine, ZoneLocation::Battlefield, DRILL_HALL);
    to_main(&mut engine, me());
    activate(&mut engine, me(), hall, 1);
    settle(&mut engine, me());

    let made = recruits(&engine);
    assert_eq!(made.len(), 1);
    assert!(is_tapped(&engine, made[0]));
    assert!(!engine.state().combat.is_attacking(made[0]));
}

/// "Whenever you attack this turn": activated in the first main phase, it
/// fires as the Sentry attacks, making two Recruits tapped and attacking,
/// and both are sacrificed at the beginning of the end step. On the
/// controller's next turn, attacking again makes nothing: the watch lapsed
/// with its turn.
#[test]
fn whenever_you_attack_this_turn_fires_this_turn_only() {
    let mut engine = bench(
        7403,
        cards(),
        [Seat::with(&[DRILL_HALL, SENTRY]), Seat::with(&[])],
    );
    let hall = the(&engine, ZoneLocation::Battlefield, DRILL_HALL);
    let sentry = the(&engine, ZoneLocation::Battlefield, SENTRY);
    let their_life = life(&engine, them());

    to_main(&mut engine, me());
    activate(&mut engine, me(), hall, 0);
    settle(&mut engine, me());
    assert!(recruits(&engine).is_empty(), "nothing before the attack");

    attack(&mut engine, me(), &[sentry], them());
    walk_until(&mut engine, |e| {
        e.state().zones.stack_is_empty() && recruits(e).len() == 2
    });
    for recruit in recruits(&engine) {
        assert!(engine.state().combat.is_attacking(recruit));
    }
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        life(&engine, them()),
        their_life - 3,
        "the Sentry and two Recruits"
    );

    to_main(&mut engine, them());
    assert!(
        recruits(&engine).is_empty(),
        "sacrificed at the beginning of the end step"
    );

    to_main(&mut engine, me());
    attack(&mut engine, me(), &[sentry], them());
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        recruits(&engine).is_empty(),
        "the watch lapsed with its turn"
    );
}
