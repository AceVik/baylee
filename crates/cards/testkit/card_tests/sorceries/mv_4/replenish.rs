//! `cards/sorceries/mv_4/replenish.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn replenish() -> CardIndex {
    card_index("523ae937-5535-490d-96ea-07f331b5e5ad")
}

/// Replenish (523ae937-5535-490d-96ea-07f331b5e5ad) — {3}{W} Sorcery (Partial):
/// "Return all enchantment cards from your graveyard to the battlefield.
/// (Auras with nothing to enchant remain in your graveyard.)"
///
/// Returns all non-Aura enchantment cards from your graveyard to the
/// battlefield while leaving Aura cards and non-enchantment cards in your
/// graveyard and leaving opponent's graveyard enchantments untouched.
#[test]
fn replenish_returns_all_your_non_aura_enchantments_and_leaves_auras_and_opponent_cards() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), index::GRIZZLY_BEARS],
        )
        .hand(
            0,
            &[
                replenish(),
                index::GLORIOUS_ANTHEM,
                index::FERVOR,
                index::HOLY_STRENGTH,
                index::HILL_GIANT,
            ],
        )
        .hand(1, &[index::GLORIOUS_ANTHEM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, index::GLORIOUS_ANTHEM);
    hand_to_graveyard(&mut engine, p0, index::FERVOR);
    hand_to_graveyard(&mut engine, p0, index::HOLY_STRENGTH);
    hand_to_graveyard(&mut engine, p0, index::HILL_GIANT);
    hand_to_graveyard(&mut engine, p1, index::GLORIOUS_ANTHEM);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));

    cast_from_hand(&mut engine, p0, replenish());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, index::GLORIOUS_ANTHEM).is_some(),
        "own non-Aura enchantment Glorious Anthem returned to battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, index::FERVOR).is_some(),
        "own non-Aura enchantment Fervor returned to battlefield"
    );
    assert_eq!(
        pt(&engine, bears),
        (3, 3),
        "returned Glorious Anthem buffs Grizzly Bears"
    );
    assert!(
        in_graveyard(&engine, p0, index::HOLY_STRENGTH).is_some(),
        "Aura card remains in your graveyard under Partial coverage"
    );
    assert!(
        in_graveyard(&engine, p0, index::HILL_GIANT).is_some(),
        "non-enchantment creature card remains in your graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, index::GLORIOUS_ANTHEM).is_some(),
        "opponent's enchantment card remains in opponent's graveyard"
    );
}
