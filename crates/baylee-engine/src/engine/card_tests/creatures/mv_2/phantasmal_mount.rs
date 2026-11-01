//! `cards/creatures/mv_2/phantasmal_mount.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phantasmal_mount() -> CardIndex {
    card_index("8c7723a5-04c7-4241-b0ba-6a821449ca43")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Phantasmal Mount (8c7723a5-04c7-4241-b0ba-6a821449ca43) — {1}{U} 1/1 Creature — Illusion Horse:
/// - "Flying"
/// - "{T}: Target creature you control with toughness 2 or less gets +1/+1 and gains flying until
///   end of turn. When this creature leaves the battlefield this turn, sacrifice that creature.
///   When the creature leaves the battlefield this turn, sacrifice this creature."
#[test]
fn phantasmal_mount_pumps_your_small_creature_and_links_both_departures_this_turn() {
    // 1. Target restriction ("you control with toughness 2 or less"), +1/+1 + flying grant,
    // and "When this creature leaves the battlefield this turn, sacrifice that creature."
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                phantasmal_mount(),
                grizzly_bears(),
                hill_giant(),
                mountain(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mount = on_battlefield(&engine, P0, phantasmal_mount()).unwrap();
    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let my_giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    activate(&mut engine, P0, phantasmal_mount(), 0);
    let options = aim_at(&mut engine, P0, my_bears);
    assert!(
        !options.contains(&my_giant),
        "creature with toughness 3 is excluded by 'toughness 2 or less'"
    );
    assert!(
        !options.contains(&their_bears),
        "opponent's creature is excluded by 'creature you control'"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert!(keywords(&engine, my_bears).contains(KeywordSet::FLYING));

    // Destroy Phantasmal Mount this turn: the delayed trigger sacrifices my_bears.
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, mount);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(my_bears).unwrap().zone,
        Zone::Graveyard,
        "when Phantasmal Mount leaves the battlefield this turn, the targeted creature is sacrificed"
    );

    // 2. Converse delayed trigger: "When the creature leaves the battlefield this turn, sacrifice this creature."
    let mut engine2 = Duel::new(SEED, mountain())
        .battlefield(0, &[phantasmal_mount(), grizzly_bears(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, P0);

    let mount2 = on_battlefield(&engine2, P0, phantasmal_mount()).unwrap();
    let my_bears2 = on_battlefield(&engine2, P0, grizzly_bears()).unwrap();

    activate(&mut engine2, P0, phantasmal_mount(), 0);
    aim_at(&mut engine2, P0, my_bears2);
    pass_until(&mut engine2, stack_is_empty);

    cast_from_hand(&mut engine2, P0, lightning_bolt());
    aim_at(&mut engine2, P0, my_bears2);
    pass_until(&mut engine2, stack_is_empty);
    assert_eq!(
        engine2.state().object(mount2).unwrap().zone,
        Zone::Graveyard,
        "when the targeted creature leaves the battlefield this turn, Phantasmal Mount is sacrificed"
    );
}

#[test]
fn phantasmal_mount_links_last_only_this_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[phantasmal_mount(), grizzly_bears(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mount = on_battlefield(&engine, P0, phantasmal_mount()).unwrap();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    activate(&mut engine, P0, phantasmal_mount(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 3));

    // P0's next turn (turn 3): the pump and both departures-this-turn are over.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "the +1/+1 lasted until end of turn only"
    );
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, mount);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, phantasmal_mount()).is_none(),
        "Phantasmal Mount died to the Bolt"
    );
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "'this turn' is over: Mount leaving on a later turn does not sacrifice the creature"
    );
}
