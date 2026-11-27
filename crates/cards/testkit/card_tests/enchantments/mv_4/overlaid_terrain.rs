//! `cards/enchantments/mv_4/overlaid_terrain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn overlaid_terrain() -> CardIndex {
    card_index("e5818ab3-8c37-4063-9e9d-cbe4ae0bba14")
}

/// Overlaid Terrain ({2}{G}{G} Enchantment):
/// "As this enchantment enters, sacrifice all lands you control."
/// "Lands you control have '{T}: Add two mana of any one color.'"
#[test]
fn overlaid_terrain_sacrifices_lands_on_entry_and_grants_two_mana_to_lands() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[island()])
        .hand(0, &[overlaid_terrain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, overlaid_terrain());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, forest()).is_none(),
        "all lands sacrificed on entry"
    );
    assert!(
        on_battlefield(&engine, P1, island()).is_some(),
        "opponent lands untouched"
    );

    // Play a Forest and tap it via granted mana ability (index 1 after printed {T}: Add {G}) -> two Blue mana!
    play_land(&mut engine, P0, forest());
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: baylee_engine::choice::granted_ability(0),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Blue), 2);
}
