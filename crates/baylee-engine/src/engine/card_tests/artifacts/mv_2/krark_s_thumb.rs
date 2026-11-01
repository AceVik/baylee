//! Krark's Thumb flips twice and honors the chosen ignored result.

use super::*;

fn krark_s_thumb() -> CardIndex {
    card_index("a97c8482-775c-4f11-9872-25b4f9bfcb1a")
}

fn goblin_lyre() -> CardIndex {
    card_index("0709e9c8-dbe7-4806-ae75-d2e2d745d6a4")
}

fn flip_and_ignore(seed: u64, ignore_first: bool) -> Option<Engine<RegistryLookup>> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                krark_s_thumb(),
                goblin_lyre(),
                gray_ogre(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, goblin_lyre(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Generic,
                ..
            }
        )
    });
    let outcomes: Vec<bool> = e
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            crate::event::GameEvent::CoinFlipped { player, won } if player == p0 => Some(won),
            _ => None,
        })
        .collect();
    if outcomes.as_slice() != [true, false] && outcomes.as_slice() != [false, true] {
        return None;
    }
    let won = if ignore_first {
        outcomes[1]
    } else {
        outcomes[0]
    };
    e.apply(p0, PlayerAction::YesNo(ignore_first)).unwrap();
    pass_until(&mut e, stack_is_empty);
    if won {
        assert_eq!(e.state().players[1].life, 18);
        assert_eq!(e.state().players[0].life, 20);
    } else {
        assert_eq!(e.state().players[0].life, 18);
        assert_eq!(e.state().players[1].life, 20);
    }
    Some(e)
}

#[test]
fn krark_s_thumb_flips_two_and_uses_the_result_not_ignored() {
    let mut mixed = 0;
    for seed in 1..40 {
        for ignore_first in [false, true] {
            if flip_and_ignore(seed, ignore_first).is_some() {
                mixed += 1;
            }
        }
        if mixed >= 2 {
            break;
        }
    }
    assert!(mixed >= 2, "both results of a mixed pair were exercised");
}
