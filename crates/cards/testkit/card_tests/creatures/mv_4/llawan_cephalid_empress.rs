//! `cards/creatures/mv_4/llawan_cephalid_empress.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn llawan_bounces_opponents_blue_creatures_on_etb_and_forbids_opponents_casting_blue_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("55f9a9a2-2842-4555-8b54-03aebc8d6fb8");
    let mut game = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                index::PHANTOM_MONSTER,
            ],
        )
        .battlefield(
            1,
            &[
                index::PHANTOM_MONSTER,
                grizzly_bears(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[card])
        .hand(1, &[index::PHANTOM_MONSTER])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    let llawan = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, llawan), (2, 3));
    assert!(
        on_battlefield(&game, p0, index::PHANTOM_MONSTER).is_some(),
        "your own blue creature is not bounced"
    );
    assert!(
        on_battlefield(&game, p1, index::PHANTOM_MONSTER).is_none(),
        "opponent's blue creature is returned to hand"
    );
    assert!(
        on_battlefield(&game, p1, grizzly_bears()).is_some(),
        "opponent's nonblue creature stays on the battlefield"
    );

    // Move to p1's main phase and verify p1 cannot cast a blue creature spell.
    reach_their_main_phase(&mut game, p1);
    tap_all_mana(&mut game, p1);
    game.refresh_offer();
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority for p1");
    };
    let blue_in_hand = in_hand(&game, p1, index::PHANTOM_MONSTER).unwrap();
    assert!(
        !legal.castable.contains(&blue_in_hand),
        "opponents cannot cast blue creature spells while Llawan is on the battlefield"
    );
}
