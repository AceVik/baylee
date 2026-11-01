#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;
fn slaughter() -> CardIndex {
    card_index("37de575d-15d4-42a2-a3d2-3855fb6ab110")
}
#[test]
fn slaughter_destroys_nonblack_creatures_despite_regeneration_and_excludes_black_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[],
        &[index::PYGMY_TROLL, index::DAUTHI_CUTTHROAT],
        &[slaughter()],
        &[],
    );
    let troll = lk::obj(&e, p1, index::PYGMY_TROLL);
    let black = lk::obj(&e, p1, index::DAUTHI_CUTTHROAT);
    lk::float(&mut e, p0, &[(ManaColor::Black, 4)]);
    lk::pass_once(&mut e);
    lk::float(&mut e, p1, &[(ManaColor::Green, 1)]);
    kit::activate_object(&mut e, p1, troll, 1);
    lk::settle(&mut e);
    cast_with_floating(&mut e, p0, slaughter());
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("targets")
    };
    assert!(options.contains(&troll));
    assert!(!options.contains(&black));
    lk::aim(&mut e, &[troll], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, p1, index::PYGMY_TROLL).is_some());
    assert!(on_battlefield(&e, p1, index::DAUTHI_CUTTHROAT).is_some());
    assert!(
        in_graveyard(&e, p0, slaughter()).is_some(),
        "buyback is omitted"
    );
}
