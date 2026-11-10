//! `cards/enchantments/mv_4/glare_of_subdual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glare_of_subdual() -> CardIndex {
    card_index("bcfc112e-0482-4616-8ca9-32338a37fa82")
}

/// "Tap an untapped creature you control: Tap target artifact or creature.":
/// one of my Elves pays by tapping, the opponent's Bears is tapped, and
/// their Forest is not on the target menu.
#[test]
fn glare_of_subdual_taps_a_creature_of_mine_to_tap_an_artifact_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[glare_of_subdual(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");

    activate(&mut engine, p0, glare_of_subdual(), 0);
    let mut target_menu = None;
    let mut cost_menu = None;
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                target_menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![bears],
                            players: vec![],
                        },
                    )
                    .expect("Bears are on the menu");
            }
            Pending::ChooseCards {
                options,
                prompt: ChoicePrompt::CostTap,
                ..
            } => {
                cost_menu = Some(options.clone());
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("a creature of mine pays");
            }
            _ => break,
        }
    }
    let target_menu = target_menu.expect("a target was asked");
    assert!(target_menu.contains(&bears) && target_menu.contains(&ring));
    assert!(!target_menu.contains(&land), "a land is neither");
    assert_eq!(cost_menu.expect("the tap cost was asked"), vec![elf]);
    assert!(is_tapped(&engine, elf), "the cost tapped my creature");
    assert!(!is_tapped(&engine, bears), "nothing yet: the ability waits");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, bears), "the target is tapped");
    assert!(!is_tapped(&engine, ring), "only the target");
}
