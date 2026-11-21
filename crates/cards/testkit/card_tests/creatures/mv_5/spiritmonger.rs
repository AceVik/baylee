//! `cards/creatures/mv_5/spiritmonger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spiritmonger() -> CardIndex {
    card_index("3913c7f2-86b7-4dd4-84a6-9bd4c1c66781")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

/// Spiritmonger (3913c7f2-86b7-4dd4-84a6-9bd4c1c66781) — `{3}{B}{G}` 6/6 Creature — Beast:
/// - Whenever this creature deals damage to a creature, put a +1/+1 counter on this creature.
/// - `{B}`: Regenerate this creature.
/// - `{G}`: This creature becomes the color of your choice until end of turn.
#[test]
fn spiritmonger_gains_counter_per_damaged_creature_and_not_on_player_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .hand(0, &[spiritmonger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, spiritmonger());
    pass_until(&mut engine, stack_is_empty);
    let monger = on_battlefield(&engine, P0, spiritmonger()).expect("Spiritmonger entered");
    assert_eq!(pt(&engine, monger), (6, 6));

    // Unblocked attack deals damage to a player, not a creature -> 0 counters.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(monger, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, monger), (6, 6));

    // Blocked by Grizzly Bears -> deals combat damage to a creature -> gets +1/+1 counter (7/7).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(monger, Defender::Player(P1))],
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
                blockers: vec![(bear, monger)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, monger), (7, 7));
}

#[test]
fn spiritmonger_regenerates_for_black_and_overwrites_color_for_green_until_eot() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[spiritmonger(), swamp(), forest()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let monger = on_battlefield(&engine, P0, spiritmonger()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(monger)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Black, Color::Green])
    );

    // Pay {G}: becomes ONLY White until end of turn; pay {B}: regeneration shield.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, spiritmonger(), 2);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let white_idx = options
        .iter()
        .find(|o| o.kind == CastModeKind::Colors(ColorSet::of(Color::White)))
        .unwrap()
        .index;
    engine
        .apply(
            P0,
            PlayerAction::ChooseMode(usize::try_from(white_idx).unwrap()),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(monger)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::White),
        "`becomes the color of your choice` overwrites printed Black/Green"
    );

    activate(&mut engine, P0, spiritmonger(), 1);
    pass_until(&mut engine, stack_is_empty);

    // P1 casts Murder in P0's main phase? Wait for P1's priority or P1's response; cast in SecondMain?
    // Shield lasts until EOT, so let P1 cast Murder in P0's end step / SecondMain priority.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, murder());
    aim_at(&mut engine, P1, monger);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, spiritmonger()).is_some());
    assert!(
        engine
            .state()
            .object(monger)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine
            .state()
            .object(monger)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Black, Color::Green]),
        "color change ends at cleanup"
    );
}
