//! `cards/sorceries/mv_4/mizzix_s_mastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The original and Mastery stay exiled whether the free copy is cast or
/// declined. Casting it draws two cards without paying the copied mana cost;
/// declining it leaves no copy or delayed permission behind.
#[test]
fn mizzixs_mastery_exiles_the_original_and_itself_and_optionally_casts_a_free_copy() {
    let p0 = PlayerId::new(0);
    for cast_copy in [false, true] {
        let mut engine = Duel::new(SEED, baylee_core::generated::index::COUNSEL_OF_THE_SORATAMI)
            .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
            .hand(0, &[mizzix_s_mastery()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0));
        seed_graveyard(&mut engine, p0, 2);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
            2
        );
        let mastery = in_hand(&engine, p0, mizzix_s_mastery()).unwrap();
        let library_before = library_size(&engine, p0);
        tap_all_mana(&mut engine, p0);
        cast_with_floating(&mut engine, p0, mizzix_s_mastery());
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected graveyard targets, got {:?}", engine.pending());
        };
        assert_eq!(options.len(), 2);
        let original = options[0];
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![original],
                    players: vec![],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::PlayFromExile,
                    ..
                }
            )
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!((min, max, options.len()), (0, 1, 1));
        let copy = options[0];
        assert_ne!(copy, original);
        assert_eq!(engine.state().object(original).unwrap().zone, Zone::Exile);
        assert_eq!(engine.state().object(mastery).unwrap().zone, Zone::Stack);
        assert_eq!(engine.state().players[0].mana_pool.total(), 0);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: if cast_copy { vec![copy] } else { vec![] },
                },
            )
            .unwrap();
        if cast_copy {
            assert_eq!(engine.state().object(copy).unwrap().zone, Zone::Stack);
        }
        pass_until(&mut engine, |e| at_rest(e, p0));
        assert_eq!(
            library_size(&engine, p0),
            library_before - usize::from(cast_copy) * 2
        );
        assert_eq!(engine.state().object(original).unwrap().zone, Zone::Exile);
        assert_eq!(engine.state().object(mastery).unwrap().zone, Zone::Exile);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
            1
        );
        assert!(engine.state().object(copy).is_none());
        assert!(in_graveyard(&engine, p0, mizzix_s_mastery()).is_none());
    }
}
