//! `cards/creatures/mv_4/major_teroh.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn major_teroh_has_flying_and_sacrifices_itself_to_exile_all_black_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("fb52dbf1-0cbc-4e1d-8ded-0498b529b4b8");
    let mut game = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                card,
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                index::DRUDGE_SKELETONS,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::SENGIR_VAMPIRE, index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let teroh = on_battlefield(&game, p0, card).unwrap();
    let skel = on_battlefield(&game, p0, index::DRUDGE_SKELETONS).unwrap();
    let vamp = on_battlefield(&game, p1, index::SENGIR_VAMPIRE).unwrap();
    assert_eq!(pt(&game, teroh), (2, 3));
    assert!(keywords(&game, teroh).contains(KeywordSet::FLYING));

    tap_all_mana(&mut game, p0);
    activate(&mut game, p0, card, 0);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(in_graveyard(&game, p0, card).is_some());
    assert_eq!(game.state().object(skel).map(|o| o.zone), Some(Zone::Exile));
    assert_eq!(game.state().object(vamp).map(|o| o.zone), Some(Zone::Exile));
    assert!(on_battlefield(&game, p0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&game, p1, index::SAVANNAH_LIONS).is_some());
}
