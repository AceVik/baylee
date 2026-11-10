//! Broken Visage — `{4}{B}` Instant (Homelands)
//!
//! Destroy target nonartifact attacking creature. It can't be regenerated.
//! Create a black Spirit creature token. Its power is equal to that creature's
//! power and its toughness is equal to that creature's toughness. Sacrifice the
//! token at the beginning of the next end step.
//!
//! (`Coverage::Partial`: dynamic P/T Spirit token matching the destroyed
//! creature's power/toughness with delayed end-step sacrifice is not in
//! `Effect`; Broken Visage destroys target nonartifact attacking creature
//! without regeneration.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn broken_visage() -> CardIndex {
    card_index("8e1d750e-44a7-47b1-8e0a-5f8b15d48b41")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

#[test]
fn broken_visage_destroys_only_nonartifact_attacking_creature_without_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(1, &[broken_visage()])
        .battlefield(
            0,
            &[
                sedge_troll(),    // attacking nonartifact creature (with regen shield)
                obsianus_golem(), // attacking artifact creature (illegal target)
                grizzly_bears(),  // nonattacking creature (illegal target)
                swamp(),
            ],
        )
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let troll = on_battlefield(&engine, p0, sedge_troll()).unwrap();
    let golem = on_battlefield(&engine, p0, obsianus_golem()).unwrap();
    let stay_home = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Put a regeneration shield on Sedge Troll ({B}: Regenerate, ability 1).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Attack with Sedge Troll and Obsianus Golem; leave Grizzly Bears back.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troll, Defender::Player(p1)), (golem, Defender::Player(p1))],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts Broken Visage ({4}{B}).
    let visage_hand = in_hand(&engine, p1, broken_visage()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: visage_hand })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&troll),
        "nonartifact attacking creature is a legal target"
    );
    assert!(
        !options.contains(&golem),
        "artifact attacking creature is not a legal target"
    );
    assert!(
        !options.contains(&stay_home),
        "nonattacking creature is not a legal target"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![troll],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(troll).unwrap().zone,
        Zone::Graveyard,
        "Sedge Troll is destroyed through its regeneration shield"
    );
}
