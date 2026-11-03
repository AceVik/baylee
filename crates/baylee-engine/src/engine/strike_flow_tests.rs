//! First strike and deathtouch in a combat played through the engine, with
//! real cards: Serra Zealot (1/1 first strike), Ankle Biter (1/1
//! deathtouch), Cudgel Troll (4/3) and Llanowar Elves (1/1). A first
//! striker deals its damage in a step of its own (CR 510.4), so a creature
//! it kills there deals none; one point of deathtouch damage is lethal
//! (CR 702.2b).

use super::testkit::{
    Duel, basic_forest, card_index, keep_mulligans, on_battlefield, pass_until, quiet_creature,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender, ObjectId};

const SEED: u64 = 0x5714_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn zealot() -> CardIndex {
    card_index("989a7353-8d3d-4ea2-ab5e-8535d95dddae")
}

fn biter() -> CardIndex {
    card_index("ee8f16ba-cbf0-4286-b26c-d78dd35bee17")
}

fn troll() -> CardIndex {
    card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2")
}

/// Seat 0 attacks with its `attacker`, seat 1 blocks with its `blocker`,
/// and the combat runs to the second main phase.
fn fight(attacker: CardIndex, blocker: CardIndex) -> (Game, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[attacker])
        .battlefield(1, &[blocker])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let a = on_battlefield(&engine, P0, attacker).expect("the attacker");
    let b = on_battlefield(&engine, P1, blocker).expect("the blocker");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(a, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b, a)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    (engine, a, b)
}

fn alive(engine: &Game, seat: PlayerId, card: CardIndex) -> bool {
    on_battlefield(engine, seat, card).is_some()
}

#[test]
fn an_attacking_first_striker_kills_its_blocker_unharmed() {
    let (engine, a, _) = fight(zealot(), quiet_creature());
    assert!(alive(&engine, P0, zealot()));
    assert!(!alive(&engine, P1, quiet_creature()));
    assert_eq!(engine.state().object(a).unwrap().damage, 0);
}

#[test]
fn a_blocking_first_striker_kills_its_attacker_unharmed() {
    let (engine, _, b) = {
        let mut engine = Duel::new(SEED, basic_forest())
            .battlefield(0, &[quiet_creature()])
            .battlefield(1, &[zealot()])
            .start();
        keep_mulligans(&mut engine);
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
        );
        let a = on_battlefield(&engine, P0, quiet_creature()).unwrap();
        let b = on_battlefield(&engine, P1, zealot()).unwrap();
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(a, Defender::Player(P1))],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(b, a)],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
        (engine, a, b)
    };
    assert!(!alive(&engine, P0, quiet_creature()));
    assert!(alive(&engine, P1, zealot()));
    assert_eq!(engine.state().object(b).unwrap().damage, 0);
}

#[test]
fn a_deathtouch_blocker_trades_with_a_troll() {
    let (engine, _, _) = fight(troll(), biter());
    assert!(
        !alive(&engine, P0, troll()),
        "one point of deathtouch is lethal"
    );
    assert!(!alive(&engine, P1, biter()));
}

#[test]
fn first_strike_beats_deathtouch() {
    let (engine, _, _) = fight(biter(), zealot());
    assert!(
        !alive(&engine, P0, biter()),
        "killed in the first-strike step"
    );
    assert!(alive(&engine, P1, zealot()), "the biter dealt no damage");
}

#[test]
fn a_blocked_attacker_deals_no_damage_to_the_player() {
    let (engine, _, _) = fight(troll(), biter());
    assert_eq!(engine.state().players[1].life, 20);
}

/// Child of Night, a 2/1 with lifelink: unblocked, its two damage to the
/// player gain its controller two (CR 702.15b).
#[test]
fn an_unblocked_lifelinker_gains_what_it_deals() {
    let child = card_index("c650a7bc-e350-44a0-a698-d4a233d66156");
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[child])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let a = on_battlefield(&engine, P0, child).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(a, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(engine.state().players[1].life, 18);
}

/// Blocked by a 1/1, it still deals both points to the blocker and its
/// controller gains both: lifelink counts the damage dealt, not the
/// toughness it needed.
#[test]
fn a_blocked_lifelinker_gains_all_the_damage_it_deals_to_the_blocker() {
    let child = card_index("c650a7bc-e350-44a0-a698-d4a233d66156");
    let (engine, _, _) = fight(child, quiet_creature());
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(!alive(&engine, P0, child), "the elf's point killed the 2/1");
}
