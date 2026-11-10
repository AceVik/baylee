//! `cards/creatures/mv_3/snow_hound.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn snow_hound() -> CardIndex {
    card_index("2fe8512f-826c-400d-ba6d-1de4f315b617")
}

/// Snow Hound (2fe8512f-826c-400d-ba6d-1de4f315b617) — {2}{W} 1/1 Creature — Dog:
/// "{1}, {T}: Return this creature and target green or blue creature you control to their owner's hand."
#[test]
fn snow_hound_returns_itself_and_target_green_or_blue_creature_you_control_to_hand() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                snow_hound(),
                plains(),
                grizzly_bears(),
                merfolk_of_the_pearl_trident(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let my_merfolk = on_battlefield(&engine, P0, merfolk_of_the_pearl_trident()).unwrap();
    let my_ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, snow_hound(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Snow Hound");
    };
    assert!(
        options.contains(&my_bears) && options.contains(&my_merfolk),
        "green and blue creatures you control are legal targets"
    );
    assert!(
        !options.contains(&my_ogre),
        "red creature you control (Gray Ogre) is excluded"
    );
    assert!(
        !options.contains(&their_bears),
        "green creature an opponent controls is excluded"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![my_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, snow_hound()).is_none()
            && in_hand(&engine, P0, snow_hound()).is_some(),
        "Snow Hound returned to owner's hand"
    );
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none()
            && in_hand(&engine, P0, grizzly_bears()).is_some(),
        "targeted green creature returned to owner's hand"
    );
    assert!(
        on_battlefield(&engine, P0, merfolk_of_the_pearl_trident()).is_some()
            && on_battlefield(&engine, P0, gray_ogre()).is_some()
            && on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "only Snow Hound and its target return: the other blue creature, the red one and the opponent's stay"
    );
}
