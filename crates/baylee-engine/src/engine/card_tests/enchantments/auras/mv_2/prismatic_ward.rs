//! `cards/enchantments/auras/mv_2/prismatic_ward.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prismatic_ward() -> CardIndex {
    card_index("ad5e2cb0-00d7-4dad-b20e-23bd41ada398")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Prismatic Ward (ad5e2cb0-00d7-4dad-b20e-23bd41ada398) — {1}{W} Enchantment — Aura:
/// - "Enchant creature"
/// - "As this Aura enters, choose a color."
/// - "Prevent all damage that would be dealt to enchanted creature by sources of the chosen color."
#[test]
fn prismatic_ward_prevents_damage_from_chosen_color_and_not_from_other_colors() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[prismatic_ward(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();

    // Cast Prismatic Ward on Hill Giant (3/3), choosing Red as it enters.
    cast_from_hand(&mut engine, P0, prismatic_ward());
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 1. Cast Lightning Bolt (a red source) at enchanted Hill Giant: all 3 damage is prevented.
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        0,
        "damage from the chosen color (Red) is completely prevented"
    );

    // 2. Attack with Hill Giant and block with P1's green Grizzly Bears (2/2):
    // damage from a green source is NOT prevented, so Hill Giant takes 2 damage.
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(PlayerId::new(1)))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_bears, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        2,
        "damage from a non-chosen color (Green) is not prevented"
    );
}
