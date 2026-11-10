//! `cards/lands/utility/plaza_of_heroes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn plaza_of_heroes_produces_restricted_legendary_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(339, forest())
        .battlefield(0, &[plaza_of_heroes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, plaza_of_heroes(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert_eq!(pool.available(ManaColor::White), 0);
    let land = on_battlefield(&engine, p0, plaza_of_heroes()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}

#[test]
fn plaza_adds_unrestricted_mana_from_current_legendary_permanents_only() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for phased in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    index::PLAZA_OF_HEROES,
                    index::ARCHANGEL_AVACYN,
                    index::GRIZZLY_BEARS,
                ],
            )
            .battlefield(1, &[index::BIRGI_GOD_OF_STORYTELLING])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let dog = on_battlefield(&e, p0, index::ARCHANGEL_AVACYN).unwrap();
        assert!(on_battlefield(&e, p1, index::BIRGI_GOD_OF_STORYTELLING).is_some());
        if phased {
            e.dev_state_mut(p0).unwrap().phase_out(&[dog]);
            e.refresh_offer();
        }
        activate(&mut e, p0, index::PLAZA_OF_HEROES, 3);
        let pool = &e.state().players[0].mana_pool;
        assert_eq!(pool.total(), u64::from(!phased));
        assert_eq!(pool.available(ManaColor::White), u32::from(!phased));
        assert!(pool.restricted().is_empty());
        assert!(stack_is_empty(&e));
    }
}

/// "{3}, {T}, Exile this land: Target legendary creature gains hexproof and
/// indestructible until end of turn." The target, not the land, gets both.
#[test]
fn plaza_gives_a_legendary_creature_hexproof_and_indestructible_for_the_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::PLAZA_OF_HEROES,
                index::BIRGI_GOD_OF_STORYTELLING,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let plaza = on_battlefield(&e, p0, index::PLAZA_OF_HEROES).unwrap();
    let birgi = on_battlefield(&e, p0, index::BIRGI_GOD_OF_STORYTELLING).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();

    activate(&mut e, p0, index::PLAZA_OF_HEROES, 2);
    let options = aim_at(&mut e, p0, birgi);
    assert!(options.contains(&birgi));
    assert!(!options.contains(&bears), "the creature must be legendary");
    pass_until(&mut e, stack_is_empty);
    let kw = keywords(&e, birgi);
    assert!(kw.contains(KeywordSet::HEXPROOF), "hexproof");
    assert!(kw.contains(KeywordSet::INDESTRUCTIBLE), "indestructible");
    assert!(
        !keywords(&e, bears).contains(KeywordSet::HEXPROOF),
        "nobody else gains anything"
    );
    assert_eq!(
        e.state().object(plaza).unwrap().zone,
        Zone::Exile,
        "the land exiles itself as part of the cost"
    );

    reach_their_main_phase(&mut e, p1);
    let kw = keywords(&e, birgi);
    assert!(!kw.contains(KeywordSet::HEXPROOF), "until end of turn");
    assert!(!kw.contains(KeywordSet::INDESTRUCTIBLE));
}

/// "{T}: Add one mana of any color. Spend this mana only to cast a legendary
/// spell." With that mana the only mana there is, a nonlegendary {W} spell
/// cannot be cast and a legendary one costing {W} can.
#[test]
fn plaza_restricted_mana_casts_only_legendary_spells() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plaza_of_heroes()])
        .hand(0, &[index::SAVANNAH_LIONS, index::LEGION_S_LANDING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, plaza_of_heroes(), 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    let lions = in_hand(&e, p0, index::SAVANNAH_LIONS).unwrap();
    let landing = in_hand(&e, p0, index::LEGION_S_LANDING).unwrap();
    assert_eq!(e.state().players[0].mana_pool.restricted().len(), 1);

    let before = e.state().snapshot_hash();
    assert!(
        e.apply(p0, PlayerAction::CastSpell { card: lions })
            .is_err(),
        "Savannah Lions is not legendary: the restricted {{W}} cannot pay for it"
    );
    assert_eq!(e.state().snapshot_hash(), before, "and nothing changed");

    e.apply(p0, PlayerAction::CastSpell { card: landing })
        .expect("Legion's Landing is legendary: the restricted {W} pays for it");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(landing).unwrap().zone, Zone::Battlefield);
    assert!(e.state().players[0].mana_pool.restricted().is_empty());
}
