//! `cards/enchantments/auras/mv_2/grave_servitude.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);

fn grave_servitude() -> CardIndex {
    card_index("6f54e7ea-bb87-4ce0-96b6-4fc82605ed2a")
}

/// Grave Servitude (6f54e7ea-bb87-4ce0-96b6-4fc82605ed2a) — {1}{B} Aura:
/// "Flash. Enchant creature. Enchanted creature gets +3/-1 and is black."
#[test]
fn grave_servitude_grants_plus_three_minus_one_and_sets_color_to_black() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), GRIZZLY_BEARS])
        .hand(0, &[grave_servitude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, grave_servitude());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(5), Some(1)));
    assert_eq!(chars.colors, ColorSet::from_slice(&[Color::Black]));
}

/// Flash is left off the card (its cleanup-step sacrifice cannot be said), so
/// the Aura is a sorcery-speed spell: castable in my own main phase with the
/// stack empty, not castable in the opponent's turn with the mana floating.
#[test]
fn grave_servitude_is_cast_at_sorcery_speed_only() {
    let table = || {
        let mut engine = Duel::new(SEED, swamp())
            .battlefield(0, &[swamp(), swamp(), GRIZZLY_BEARS])
            .hand(0, &[grave_servitude()])
            .start();
        keep_mulligans(&mut engine);
        engine
    };

    // My main phase: castable.
    let mut engine = table();
    reach_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, grave_servitude()).unwrap();
    let legal = priority_offer(&engine);
    assert!(
        legal.castable.contains(&card),
        "castable in my main phase: {legal:?}"
    );

    // The opponent's upkeep, P0 holding priority with untapped Swamps.
    let mut engine = table();
    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, grave_servitude()).unwrap();
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "no flash, so not castable on the opponent's turn: {legal:?}"
    );
}
