//! `cards/artifacts/equipment/mv_1/lead_pipe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lead_pipe() -> CardIndex {
    card_index("ed2ef95e-8851-4226-9c63-4c19ea323b82")
}

/// Lead Pipe — "Equipped creature gets +2/+0. Whenever equipped creature
/// dies, each opponent loses 1 life." and "Equip {2}". Only the equipped
/// creature's death drains.
#[test]
fn lead_pipe_drains_when_its_host_dies() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[lead_pipe(), elf, elf], &[]);
    let p0 = PlayerId::new(0);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    equip(&mut engine, lead_pipe(), 2, host);
    assert_eq!(pt(&engine, host), (3, 1), "+2/+0");

    kill(&mut engine, other);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "a bystander's death is free"
    );
    kill(&mut engine, host);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the host's death drains 1"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

/// "{2}, Sacrifice this Equipment: Draw a card." The Pipe goes to the
/// graveyard and its controller draws; the host loses the +2.
#[test]
fn lead_pipe_sacrifices_itself_to_draw() {
    let elf = llanowar_elves();
    let mut engine = rig(4, &[], &[lead_pipe(), elf], &[]);
    let p0 = PlayerId::new(0);
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    let pipe = equip(&mut engine, lead_pipe(), 2, host);
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let before = hand(&engine);

    float(&mut engine, 2);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let (_, sacrifice) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, idx)| *src == pipe && *idx != 0)
        .expect("the sacrifice ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pipe,
                ability_index: sacrifice,
            },
        )
        .expect("{2} and the Pipe itself pay");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand(&engine), before + 1, "one card drawn");
    assert!(on_battlefield(&engine, p0, lead_pipe()).is_none());
    assert!(in_graveyard(&engine, p0, lead_pipe()).is_some());
    assert_eq!(pt(&engine, host), (1, 1), "the +2/+0 left with it");
    assert_eq!(pool_total(&engine), 0, "the {{2}} was paid");
}
