//! `cards/creatures/mv_6/carnage_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Carnage Tyrant: "This spell can't be countered." / "Trample, hexproof"
/// The green 7/6 Dinosaur is cast into two untapped Islands holding Counterspell.
/// Counterspell resolves but cannot counter the spell; Carnage Tyrant arrives
/// safely on the battlefield with 7/6 power/toughness, trample, and hexproof.
#[test]
fn carnage_tyrant_cannot_be_countered_and_enters_with_keywords() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(42, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[carnage_tyrant()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let tyrant = in_hand(&engine, p0, carnage_tyrant()).expect("Carnage Tyrant in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: tyrant })
        .expect("six Forests pay {4}{G}{G}");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("Counterspell in hand");
    // Refused outright until #243, which read "can't be countered" as "can't
    // be targeted". The Gatherer ruling says otherwise: the counterspell may
    // point at it, and resolves without countering it.
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .expect("the Tyrant is a legal target for Counterspell");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![tyrant],
            },
        )
        .expect("Counterspell points at the Tyrant");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, counterspell()).is_some()
    });

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, carnage_tyrant()).is_some()
    });
    let tyrant_obj = on_battlefield(&engine, p0, carnage_tyrant()).expect("entered battlefield");
    assert!(
        in_graveyard(&engine, p0, carnage_tyrant()).is_none(),
        "uncounterable spell does not go to graveyard"
    );
    assert_eq!(pt(&engine, tyrant_obj), (7, 6));
    let kw = keywords(&engine, tyrant_obj);
    assert!(kw.contains(KeywordSet::TRAMPLE));
    assert!(kw.contains(KeywordSet::HEXPROOF));
}
