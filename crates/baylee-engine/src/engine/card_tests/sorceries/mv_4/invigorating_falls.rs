//! `cards/sorceries/mv_4/invigorating_falls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn invigorating_falls_gains_life_for_creature_cards_across_all_graveyards() {
    let p0 = PlayerId::new(0);
    let card = card_index("c5c98759-e009-4340-9326-9f5ff70a86bc");
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, swamp()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bear0 = on_battlefield(&game, p0, index::GRIZZLY_BEARS).unwrap();
    let elf0 = on_battlefield(&game, p0, llanowar_elves()).unwrap();
    let art0 = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let bear1 = on_battlefield(&game, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();
    let swamp1 = on_battlefield(&game, PlayerId::new(1), swamp()).unwrap();

    // Put 2 creatures + 1 artifact into p0's graveyard, and 1 creature + 1 land into p1's graveyard.
    bury(&mut game, &[bear0, elf0, art0, bear1, swamp1]);
    game.refresh_offer();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    // 3 creature cards in all graveyards -> p0 gains 3 life (20 -> 23).
    assert_eq!(game.state().players[0].life, 23);
}
