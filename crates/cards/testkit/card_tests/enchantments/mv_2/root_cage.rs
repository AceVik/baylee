//! `cards/enchantments/mv_2/root_cage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Root Cage` is an enchantment costing `{1}{G}` under `Coverage::Implemented`.
/// It prints "Mercenaries don't untap during their controllers' untap steps."
/// When both a Mercenary (`moggcatcher()`) and a non-Mercenary (`llanowar_elves()`) attack and tap,
/// advancing through the opponent's turn to the controller's next turn untaps the non-Mercenary,
/// while `moggcatcher()` remains tapped due to `Root Cage`.
#[test]
fn root_cage_stops_mercenaries_from_untapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[root_cage(), moggcatcher(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mogg = on_battlefield(&engine, p0, moggcatcher()).expect("moggcatcher deployed");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    let def = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mogg, def), (elf, def)],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, mogg),
        "moggcatcher is tapped from attacking"
    );
    assert!(is_tapped(&engine, elf), "elf is tapped from attacking");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, elf),
        "non-Mercenary elf untaps as normal"
    );
    assert!(
        is_tapped(&engine, mogg),
        "Mercenary moggcatcher stays tapped under Root Cage"
    );
}
