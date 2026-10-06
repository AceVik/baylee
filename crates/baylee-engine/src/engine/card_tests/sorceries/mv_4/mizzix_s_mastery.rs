//! `cards/sorceries/mv_4/mizzix_s_mastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mizzix's Mastery exiles a card out of its controller's own graveyard and
/// then exiles **itself**, and the second half is the one worth playing: a
/// sorcery that resolved normally would be in the graveyard it just emptied
/// a slot in, so "Exile Mizzix's Mastery" is checked by where the spell
/// itself ends up rather than by what it did.
///
/// The library is built out of Brainstorms so the graveyard the spell reads
/// holds instants and nothing else, and the target is asserted against the
/// object that actually left.
///
/// The file is `Coverage::Partial` for the copy — nothing copies a card in
/// exile and lets its controller cast the copy for free — and for overload.
/// Neither is reachable from here, and the exile is the half that is built.
#[test]
fn mizzixs_mastery_exiles_a_card_from_your_graveyard_and_then_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, brainstorm())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[mizzix_s_mastery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 2);

    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(graveyard_before, 2, "two instants are buried");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, mizzix_s_mastery());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target card that's an instant or sorcery from your graveyard\" \
             asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 2, "both buried instants are on the offer");
    let named = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![named],
                players: vec![],
            },
        )
        .expect("a card the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&named),
        "the card it named left the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, mizzix_s_mastery()).is_none(),
        "\"Exile Mizzix's Mastery\" — a sorcery that merely resolved would be \
         lying in the graveyard it just took a card out of"
    );
}
