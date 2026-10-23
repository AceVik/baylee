#[allow(clippy::wildcard_imports)]
use super::*;

fn mana_leech() -> CardIndex {
    card_index("cad05528-917b-4296-9d23-ba9433612709")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn mana_leech_locks_only_its_land_until_it_chooses_to_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[mana_leech()],
        &[forest(), grizzly_bears()],
        &[],
        &[],
    );
    let leech = lk::obj(&e, p0, mana_leech());
    let land = lk::obj(&e, p1, forest());
    let bear = lk::obj(&e, p1, grizzly_bears());
    activate(&mut e, p0, mana_leech(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target land")
    };
    assert!(options.contains(&land) && !options.contains(&bear));
    lk::aim(&mut e, &[land], &[]);
    lk::settle(&mut e);
    reach_their_main_phase(&mut e, p1);
    assert!(is_tapped(&e, land));
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![leech],
        },
    )
    .unwrap();
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    assert!(is_tapped(&e, land));
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    reach_main_phase(&mut e, p0);
    assert!(!is_tapped(&e, leech));
    reach_their_main_phase(&mut e, p1);
    assert!(!is_tapped(&e, land));
}
