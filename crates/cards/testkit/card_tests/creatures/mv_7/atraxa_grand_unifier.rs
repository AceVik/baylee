//! `cards/creatures/mv_7/atraxa_grand_unifier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "For each card type, you may put a card of that type from among the
/// revealed cards into your hand. Put the rest on the bottom of your
/// library in a random order." Ten revealed: an artifact creature, an
/// artifact, two creatures, an enchantment, two instants, a land, a
/// planeswalker and a sorcery. One question per type a revealed card has,
/// in CR 205.2a's order, and none for the three types nothing has (battle,
/// kindred and, after the answers, nothing left); the artifact creature,
/// taken as the artifact, is not offered again as a creature; the
/// planeswalker is declined; the four cards not taken go to the bottom.
#[test]
fn atraxa_grand_unifier_asks_once_per_card_type_in_the_rules_order() {
    let p0 = PlayerId::new(0);
    let (mut engine, ids) = atraxa_reveals(&[
        copper_myr(),
        sol_ring(),
        llanowar_elves(),
        thundering_giant(),
        counterspell(),
        mountain(),
        damn(),
        underworld_breach(),
        karn_the_great_creator(),
        swords_to_plowshares(),
    ]);
    let [
        myr,
        ring,
        elves,
        giant,
        counter,
        land,
        damn_id,
        breach,
        karn,
        swords,
    ] = ids[..]
    else {
        panic!("ten stacked")
    };
    let answers: [(TypeSet, Vec<ObjectId>, Option<ObjectId>); 7] = [
        (TypeSet::ARTIFACT, vec![myr, ring], Some(myr)),
        (TypeSet::CREATURE, vec![elves, giant], Some(elves)),
        (TypeSet::ENCHANTMENT, vec![breach], Some(breach)),
        (TypeSet::INSTANT, vec![counter, swords], Some(counter)),
        (TypeSet::LAND, vec![land], Some(land)),
        (TypeSet::PLANESWALKER, vec![karn], None),
        (TypeSet::SORCERY, vec![damn_id], Some(damn_id)),
    ];
    for (card_type, options, take) in answers {
        let (asked, offered) = atraxa_question(&engine)
            .unwrap_or_else(|| panic!("asked about {card_type:?}, got {:?}", engine.pending()));
        assert_eq!(asked, card_type);
        assert_eq!(sorted(offered), sorted(options), "{card_type:?}'s menu");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: take.into_iter().collect(),
                },
            )
            .unwrap();
    }
    assert!(atraxa_question(&engine).is_none(), "no eighth question");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    for taken in [myr, elves, breach, counter, land, damn_id] {
        assert!(hand.contains(&taken));
    }
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        sorted(library[..4].to_vec()),
        sorted(vec![ring, giant, karn, swords]),
        "the rest at the bottom"
    );
    assert!(!library[4..].iter().any(|id| ids.contains(id)));
    assert!(on_battlefield(&engine, p0, atraxa_grand_unifier()).is_some());
}
