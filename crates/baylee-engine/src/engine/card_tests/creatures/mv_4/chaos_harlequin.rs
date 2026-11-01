//! Chaos Harlequin — `{2}{R}{R}` 2/4 Creature — Human (Alliances)
//!
//! `{R}`: Exile the top card of your library. If that card is a land card, this creature gets
//! -4/-0 until end of turn. Otherwise, this creature gets +2/+0 until end of turn.
//!
//! The ability is off the card: nothing exiles the top card and branches on its type, and a flat
//! `{R}: +2/+0` would be better than printed (never -4/-0, nothing exiled). This test pins that the
//! creature has no ability yet.

#[allow(clippy::wildcard_imports)]
use super::*;

fn chaos_harlequin() -> CardIndex {
    card_index("c8e96a51-24c9-4955-8bf0-6ef448fa8011")
}

#[test]
fn chaos_harlequin_has_no_ability_it_could_play_wrongly() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[chaos_harlequin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, chaos_harlequin());
    pass_until(&mut engine, stack_is_empty);
    let harlequin = on_battlefield(&engine, p0, chaos_harlequin()).expect("Chaos Harlequin enters");
    assert_eq!(pt(&engine, harlequin), (2, 4));

    let library = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: harlequin,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the exile-and-branch ability is not on the card"
    );
    assert_eq!(pt(&engine, harlequin), (2, 4));
    assert_eq!(library_size(&engine, p0), library);
}
