//! `cards/artifacts/mv_0/chrome_mox.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn chrome_mox() -> CardIndex {
    card_index("ec3d4466-547c-4e02-b1b5-a156ec4637e9")
}

/// Imprint accepts only a nonartifact, nonland hand card, and the linked mana
/// ability produces exactly that exiled card's single color without a redundant
/// color-choice question.
#[test]
fn chrome_mox_imprints_only_a_nonartifact_nonland_and_reads_its_colors() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[chrome_mox(), lightning_bolt(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bolt = in_hand(&e, p0, lightning_bolt()).expect("red card in hand");
    let artifact = in_hand(&e, p0, sol_ring()).expect("artifact in hand");
    let land = in_hand(&e, p0, forest()).expect("land in hand");
    cast_from_hand(&mut e, p0, chrome_mox());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true))
        .expect("the optional imprint may be taken");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!("the walk stopped on the imprint choice")
    };
    assert_eq!((min, max), (1, 1), "one card after choosing to imprint");
    assert!(options.contains(&bolt));
    assert!(
        !options.contains(&artifact),
        "artifacts are not imprintable"
    );
    assert!(!options.contains(&land), "lands are not imprintable");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt],
        },
    )
    .expect("the red card is a legal imprint");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(bolt).expect("the exiled card").zone,
        Zone::Exile
    );
    assert!(in_hand(&e, p0, sol_ring()).is_some());
    assert!(in_hand(&e, p0, forest()).is_some());
    activate(&mut e, p0, chrome_mox(), 1);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
}
