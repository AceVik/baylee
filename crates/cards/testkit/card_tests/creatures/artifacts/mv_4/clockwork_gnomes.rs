//! Clockwork Gnomes — `{4}` 2/2 Artifact Creature — Gnome (Homelands)
//!
//! `{3}`, `{T}`: Regenerate target artifact creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn clockwork_gnomes() -> CardIndex {
    card_index("21d269f2-c11c-4094-b526-500f0785f483")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

#[test]
fn clockwork_gnomes_regenerates_target_artifact_creature_from_destruction() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[shatter()])
        .battlefield(
            0,
            &[
                clockwork_gnomes(),
                obsianus_golem(),
                grizzly_bears(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gnomes = on_battlefield(&engine, p0, clockwork_gnomes()).unwrap();
    let golem = on_battlefield(&engine, p0, obsianus_golem()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let shatter_hand = in_hand(&engine, p0, shatter()).unwrap();
    assert_eq!(pt(&engine, gnomes), (2, 2));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gnomes,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&golem));
    assert!(options.contains(&gnomes));
    assert!(
        !options.contains(&bears),
        "nonartifact creature cannot be targeted by Clockwork Gnomes"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![golem],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Shatter on Obsianus Golem: its regeneration shield saves it and taps it.
    engine
        .apply(p0, PlayerAction::CastSpell { card: shatter_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![golem],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(golem).unwrap().zone,
        Zone::Battlefield,
        "Obsianus Golem regenerates instead of being destroyed"
    );
    assert!(is_tapped(&engine, golem));
}
