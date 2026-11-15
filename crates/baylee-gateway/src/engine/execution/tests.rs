use super::*;
use crate::engine::{Agent, Agents};
use tokio::sync::mpsc;

fn agent(engine: &Identity, occupied: bool) -> Agent {
    Agent {
        name: "synthetic".into(),
        local: true,
        capacity: 1,
        engines: vec![engine.clone()],
        tx: mpsc::unbounded_channel().0,
        games: if occupied {
            vec!["game".into()]
        } else {
            Vec::new()
        },
    }
}

#[test]
fn tower_inventory_and_assignment_never_substitute_another_build_or_full_agent() {
    let identity = native(Variant::Tower);
    let mut old = identity.clone();
    old.build = "retained-tower-build".into();
    let mut agents = Agents::default();
    agents
        .connected
        .insert("standard".into(), agent(&native(Variant::Standard), false));
    assert!(agents.tower_inventory().unwrap().installed.is_empty());
    assert!(agents.pick_exact(&identity).is_none());
    agents.connected.insert("old".into(), agent(&old, false));
    assert!(agents.pick_exact(&identity).is_none());
    agents
        .connected
        .insert("current".into(), agent(&identity, true));
    let inventory = agents.tower_inventory().unwrap();
    assert_eq!(
        inventory.installed.len(),
        2,
        "busy agents retain compatibility"
    );
    assert!(inventory.installed.contains(&identity));
    assert_eq!(inventory.available, vec![old.clone()]);
    assert!(agents.pick_exact(&identity).is_none());
    agents.connected.get_mut("current").unwrap().games.clear();
    agents
        .connected
        .insert("copy".into(), agent(&identity, false));
    assert_eq!(
        agents.tower_inventory().unwrap().available.len(),
        2,
        "deduplicate exact inventories"
    );
    assert_eq!(
        agents.pick_exact(&identity).unwrap(),
        ("copy".into(), identity.clone())
    );
    assert_eq!(agents.pick_exact(&old).unwrap(), ("old".into(), old));
    let mut changed = identity;
    changed.cards = "a".repeat(40);
    assert!(agents.pick_exact(&changed).is_none());
}
