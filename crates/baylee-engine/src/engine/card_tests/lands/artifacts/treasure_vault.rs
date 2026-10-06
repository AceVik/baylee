//! `cards/lands/artifacts/treasure_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Treasure Vault: "{T}: Add {C}." and "{X}{X}, {T}, Sacrifice this land: Create X Treasure tokens."
/// Treasure Vault is played as an artifact land, entering untapped with both types.
/// Its mana ability is offered and tapped to produce {C}, and its activated sacrifice ability is offered.
#[test]
fn treasure_vault_enters_as_artifact_land_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(88, forest()).hand(0, &[treasure_vault()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = play_land(&mut engine, p0, treasure_vault());
    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, vault));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "ability 0 is the printed colorless mana ability"
    );
    assert!(
        legal.abilities.contains(&(vault, 1)),
        "ability 1 is the sacrifice ability"
    );

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
    activate(&mut engine, p0, treasure_vault(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, vault));
}

/// Treasure Vault's second ability, which is the one the card is played
/// for: "{{X}}{{X}}, {{T}}, Sacrifice this land: Create X Treasure tokens."
///
/// `{{X}}{{X}}` is the shape that says the substitution is per **symbol**
/// and not per cost: CR 107.3i makes every instance of X on the object one
/// value, so four mana buys X = 2 and not X = 4. The neighbouring test asserts only
/// that the ability is *offered* with nothing floating, which is true and
/// stays true — an announcement of 0 is legal and creates no Treasure.
/// Nothing said what the number was worth until CR 602.2b was implemented;
/// before it, this card sacrificed itself for nothing at all.
#[test]
fn treasure_vault_pays_two_x_symbols_out_of_one_announced_number() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[treasure_vault(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, treasure_vault()).expect("vault on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(treasure_vault()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(
        !is_tapped(&engine, vault),
        "the {{T}} in the cost is still there to pay"
    );

    activate(&mut engine, p0, treasure_vault(), 1);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the announced X, got {:?}", engine.pending());
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "four mana pays two X symbols at X = 2, not one at X = 4"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("announce X = 2");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, treasure_vault()).is_some());
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "X Treasures, with X the number announced as the cost was paid"
    );
}
