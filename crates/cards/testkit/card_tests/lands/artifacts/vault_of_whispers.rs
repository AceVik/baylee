//! `cards/lands/artifacts/vault_of_whispers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vault of Whispers is an artifact land whose entire printed text is
/// "{T}: Add {B}" — it has no basic land type, so there is no CR 305.6
/// shortcut anywhere in it and the black mana can only come off the ability
/// the card itself prints. The scenario plays it as a land, so the entry
/// really happened and the permanent really is untapped, and then presses
/// that one line: a pool that was empty before reads one black and nothing
/// else afterwards, without ever using the stack (CR 605.3b). The type line
/// is read after the layer system has run, because "artifact" is the half a
/// card that counts artifacts sees and "land" the half the land drop and the
/// untap step see.
#[test]
fn vault_of_whispers_enters_as_an_artifact_land_and_taps_for_one_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[vault_of_whispers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = play_land(&mut engine, p0, vault_of_whispers());
    let kinds = types(&engine, vault);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land is both at once: {kinds:?}"
    );
    assert!(
        !entered_tapped(&engine, vault),
        "nothing on the card holds it down, so it stands untapped and its \
         {{T}} is payable this turn"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "the printed {{T}}: Add {{B}} is an ordinary indexed ability: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&vault),
        "and not the CR 305.6 shortcut: an artifact land has no basic land \
         type to read one off"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty before the Vault is tapped, so one black afterwards \
         has one possible source on this board"
    );

    activate(&mut engine, p0, vault_of_whispers(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "\"{{T}}: Add {{B}}\" — the color the card prints"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        is_tapped(&engine, vault),
        "and the {{T}} is what paid for it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
