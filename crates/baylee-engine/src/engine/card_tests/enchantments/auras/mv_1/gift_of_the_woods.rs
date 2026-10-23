//! `cards/enchantments/auras/mv_1/gift_of_the_woods.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gift_of_the_woods() -> CardIndex {
    card_index("491c6f69-9b75-4d5d-b41f-a14b2fbfdd07")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Player 0's Grizzly Bears wears Gift of the Woods; its Gray Ogre does not. Player 1 has a Hill
/// Giant and a Grizzly Bears. Returns at player 0's first main phase with the Aura resolved, and
/// the two creatures of player 0 (the enchanted one first).
fn gifted_table() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), grizzly_bears(), gray_ogre()])
        .battlefield(1, &[hill_giant(), grizzly_bears()])
        .hand(0, &[gift_of_the_woods()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, gift_of_the_woods());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, bears, ogre)
}

/// `attacker` attacks `defender` with `attacking`; the defender blocks as `blocks`
/// (blocker, attacker) says. Returns once the block's triggers have resolved.
fn fight(
    engine: &mut Engine<RegistryLookup>,
    attacker: PlayerId,
    defender: PlayerId,
    attacking: &[ObjectId],
    blocks: Vec<(ObjectId, ObjectId)>,
) {
    pass_until(engine, |e| {
        e.state().turn.active == attacker && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            attacker,
            PlayerAction::DeclareAttackers {
                attackers: attacking
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| e.state().turn.step == Step::DeclareBlockers);
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers: blocks })
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// "Whenever enchanted creature ... becomes blocked, it gets +0/+3 until end of turn and you gain
/// 1 life": both attackers are blocked, and only the enchanted one is pumped and pays out, once.
#[test]
fn gift_of_the_woods_pumps_and_gains_life_when_only_the_enchanted_creature_becomes_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears, ogre) = gifted_table();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    fight(
        &mut engine,
        p0,
        p1,
        &[bears, ogre],
        vec![(giant, bears), (their_bears, ogre)],
    );

    assert_eq!(
        pt(&engine, bears),
        (2, 5),
        "+0/+3 for the enchanted creature"
    );
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "the unenchanted attacker is untouched"
    );
    assert_eq!(engine.state().players[0].life, 21, "one life, not two");
    assert_eq!(engine.state().players[1].life, 20);
}

/// "... blocks": on the other player's turn the enchanted creature blocks one attacker and the
/// unenchanted one blocks the other, and again only the enchanted blocker is pumped and pays out.
#[test]
fn gift_of_the_woods_pumps_and_gains_life_when_the_enchanted_creature_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears, ogre) = gifted_table();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    fight(
        &mut engine,
        p1,
        p0,
        &[giant, their_bears],
        vec![(bears, giant), (ogre, their_bears)],
    );

    assert_eq!(pt(&engine, bears), (2, 5));
    assert_eq!(pt(&engine, ogre), (2, 2));
    assert_eq!(engine.state().players[0].life, 21);
    assert_eq!(engine.state().players[1].life, 20);
}

/// Unblocked, the enchanted creature neither blocked nor became blocked: nothing happens.
#[test]
fn gift_of_the_woods_does_nothing_for_an_unblocked_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears, _ogre) = gifted_table();

    fight(&mut engine, p0, p1, &[bears], vec![]);

    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(engine.state().players[0].life, 20);
}

/// "Until end of turn": the +0/+3 carried it through the Giant's 3 damage, and it is a plain 2/2
/// again in the next turn, still alive.
#[test]
fn gift_of_the_woods_pump_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears, ogre) = gifted_table();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    fight(
        &mut engine,
        p0,
        p1,
        &[bears, ogre],
        vec![(giant, bears), (their_bears, ogre)],
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        3,
        "the Giant hit it, and the +0/+3 kept it alive"
    );

    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("it survived the turn");
    assert_eq!(pt(&engine, bears), (2, 2));
}

/// "Enchant creature": a land is no target. (The card enchants only a creature its controller
/// controls, so the opponent's Hill Giant is refused too, which is less than printed.)
#[test]
fn gift_of_the_woods_enchants_only_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[gift_of_the_woods()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, gift_of_the_woods());
    for illegal in [land, giant] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "{illegal:?} is not a creature you control"
        );
    }
}
