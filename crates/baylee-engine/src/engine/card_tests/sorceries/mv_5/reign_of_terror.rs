//! `cards/sorceries/mv_5/reign_of_terror.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::{bears as grizzly_bears, savannah_lions};

fn reign_of_terror() -> CardIndex {
    card_index("67d12a5a-9e0c-4550-86ad-cd1e173fea8f")
}

/// Reign of Terror (`{3}{B}{B}`, `Coverage::Partial`): chooses between
/// destroying all green creatures or all white creatures with no regeneration.
#[test]
fn reign_of_terror_destroys_chosen_color_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[grizzly_bears(), savannah_lions(), gray_ogre()])
        .hand(0, &[reign_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, reign_of_terror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "green creature is destroyed when mode 0 is chosen"
    );
    assert!(
        on_battlefield(&engine, p1, savannah_lions()).is_some(),
        "white creature survives mode 0"
    );
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "red creature survives mode 0"
    );
}
