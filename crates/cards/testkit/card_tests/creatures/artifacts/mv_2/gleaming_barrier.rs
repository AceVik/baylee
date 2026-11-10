//! `cards/creatures/artifacts/mv_2/gleaming_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gleaming_barrier() -> CardIndex {
    card_index("1eb9e401-8975-447b-839d-f7cd23897465")
}

/// Gleaming Barrier: a 0/4 with "Defender", not offered as an attacker.
#[test]
fn gleaming_barrier_is_a_zero_four_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[gleaming_barrier(), grizzly_bears()], &[]);
    let wall = on_battlefield(&engine, p0, gleaming_barrier()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 4));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&wall));
}

/// "When this creature dies, create a Treasure token": one artifact token
/// named Treasure, which taps and sacrifices for one mana of any color.
#[test]
fn gleaming_barrier_leaves_a_treasure_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[gleaming_barrier()], &[]);
    let wall = on_battlefield(&engine, p0, gleaming_barrier()).expect("seated");
    assert!(tokens_of(&engine, p0).is_empty(), "no token yet");
    kill(&mut engine, wall);
    assert!(in_graveyard(&engine, p0, gleaming_barrier()).is_some());
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "exactly one token");
    let treasure = tokens[0];
    let chars = engine.state().object(treasure).unwrap().characteristics();
    assert_eq!(engine.state().names.get(chars.name), "Treasure");
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(!chars.types.contains(TypeSet::CREATURE));

    let ability = priority_offer(&engine)
        .abilities
        .iter()
        .find(|(id, _)| *id == treasure)
        .copied()
        .expect("the Treasure's mana ability is on offer");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: treasure,
                ability_index: ability.1,
            },
        )
        .expect("{T}, sacrifice");
    name_a_color(&mut engine, ManaColor::Green);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(
        engine.state().object(treasure).is_none()
            || engine.state().object(treasure).unwrap().zone != Zone::Battlefield,
        "sacrificed"
    );
}
