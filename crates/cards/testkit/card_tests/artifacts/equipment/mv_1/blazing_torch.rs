//! `cards/artifacts/equipment/mv_1/blazing_torch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn blazing_torch() -> CardIndex {
    card_index("cd3d1e5d-db12-4878-92a2-23eb8d23eaf3")
}

fn blood_artist() -> CardIndex {
    card_index("310f141c-7f37-4729-aed6-dd9c09db448d")
}

fn walking_corpse() -> CardIndex {
    card_index("fea95888-e16a-4209-9cd4-623f7f4d2f67")
}

/// Vampires and Zombies cannot block the equipped attacker, while all three
/// can still block its unequipped twin and an ordinary creature can block both.
#[test]
fn torch_forbids_only_vampire_and_zombie_blocks_against_its_wearer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = rig(
        1,
        &[],
        &[blazing_torch(), tragic_poet(), tragic_poet()],
        &[blood_artist(), walking_corpse(), llanowar_elves()],
    );
    let poets = all_on_battlefield(&engine, p0, tragic_poet());
    let (equipped, unequipped) = (poets[0], poets[1]);
    equip(&mut engine, blazing_torch(), 1, equipped);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (equipped, Defender::Player(p1)),
                    (unequipped, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the attack reached declare blockers")
    };
    for card in [blood_artist(), walking_corpse()] {
        let blocker = on_battlefield(&engine, p1, card).unwrap();
        let offer = blockers
            .iter()
            .find(|offer| offer.blocker == blocker)
            .expect("the Vampire or Zombie can block something");
        assert!(!offer.attackers.contains(&equipped), "{offer:?}");
        assert!(offer.attackers.contains(&unequipped), "{offer:?}");
    }
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let offer = blockers
        .iter()
        .find(|offer| offer.blocker == elf)
        .expect("an ordinary creature may block");
    assert!(offer.attackers.contains(&equipped));
    assert!(offer.attackers.contains(&unequipped));
}

/// Equip {1} moves both static texts to the new wearer. The granted ability
/// taps that wearer, sacrifices the Torch rather than either creature, and
/// the Torch's last-known identity deals two to any target.
#[test]
fn torch_moves_its_grant_then_sacrifices_itself_to_deal_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = rig(
        2,
        &[],
        &[blazing_torch(), tragic_poet(), tragic_poet()],
        &[walking_corpse()],
    );
    let poets = all_on_battlefield(&engine, p0, tragic_poet());
    let (first, second) = (poets[0], poets[1]);
    let torch = equip(&mut engine, blazing_torch(), 1, first);
    let granted = baylee_engine::choice::granted_ability(0);
    let offered =
        |e: &Engine<RegistryLookup>, who| priority_offer(e).abilities.contains(&(who, granted));
    assert!(offered(&engine, first));
    assert!(!offered(&engine, second));

    equip(&mut engine, blazing_torch(), 1, second);
    assert!(!offered(&engine, first), "the old wearer loses the shot");
    assert!(offered(&engine, second), "the new wearer gains it");
    let corpse = on_battlefield(&engine, p1, walking_corpse()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: second,
                ability_index: granted,
            },
        )
        .expect("the equipped creature can activate the shot");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the shot asks for any target: {:?}", engine.pending())
    };
    assert!(options.contains(&corpse));
    assert!(player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![corpse],
                players: vec![],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, second), "the wearer paid {{T}}");
    assert!(!is_tapped(&engine, first));
    assert_eq!(engine.state().object(torch).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(torch).unwrap().attached_to, None);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(corpse).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the creature was targeted"
    );
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::DamageDealt {
                    source: Some(source),
                    target: baylee_engine::event::DamageTarget::Object(target),
                    amount: 2,
                    is_combat: false,
                } if source == torch && target == corpse
            )),
        "the sacrificed Torch, not the wearer, dealt the two"
    );
}
