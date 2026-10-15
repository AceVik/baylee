//! `cards/enchantments/mv_4/strands_of_night.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn strands_of_night() -> CardIndex {
    card_index("0e8ae1f7-8926-414a-8434-db0b0f877505")
}

/// "{B}{B}, Pay 2 life, Sacrifice a Swamp: Return target creature card from
/// your graveyard to the battlefield.": the menu holds my graveyard's
/// creature and not the opponent's, the Swamp and 2 life are the cost.
#[test]
fn strands_of_night_returns_a_creature_from_my_graveyard_for_a_swamp() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[strands_of_night(), swamp()])
        .hand(0, &[grizzly_bears()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = hand_to_graveyard(&mut engine, p0, grizzly_bears());
    let theirs = hand_to_graveyard(&mut engine, p1, llanowar_elves());
    float(&mut engine, p0, ManaColor::Black, 2);

    activate(&mut engine, p0, strands_of_night(), 0);
    let mut menu = None;
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![mine],
                            players: vec![],
                        },
                    )
                    .expect("my creature card was on the menu");
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("the Swamp pays");
            }
            _ => break,
        }
    }
    let menu = menu.expect("a target was asked");
    assert!(menu.contains(&mine), "my creature card");
    assert!(!menu.contains(&theirs), "not the opponent's graveyard");
    assert_eq!(life_of(&engine, p0), 18, "2 life is part of the cost");
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "the Swamp was sacrificed"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the creature card returned to the battlefield"
    );
}
