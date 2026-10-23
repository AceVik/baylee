#[allow(clippy::wildcard_imports)]
use super::*;

fn metrognome() -> CardIndex {
    card_index("57fe5c94-ee93-45c6-bf7e-89d177d0fe70")
}

#[test]
fn metrognome_creates_gnome_token_when_activated() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[metrognome(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, metrognome(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .len(),
        before + 1
    );
}

#[test]
fn metrognome_triggers_only_when_opponents_spell_discards_this_card() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (opponent, discard_this) in [(false, true), (true, false), (true, true)] {
        let caster = if opponent { p1 } else { p0 };
        let mut p0_hand = vec![metrognome(), index::GRIZZLY_BEARS];
        let p1_hand = if opponent {
            vec![index::THOUGHTSEIZE]
        } else {
            p0_hand.push(index::THOUGHTSEIZE);
            vec![]
        };
        let mut e = lk::game(SEED, &[], &[], &p0_hand, &p1_hand);
        let discarded = in_hand(
            &e,
            p0,
            if discard_this {
                metrognome()
            } else {
                baylee_core::generated::index::GRIZZLY_BEARS
            },
        )
        .unwrap();
        if opponent {
            reach_their_main_phase(&mut e, p1);
        }
        lk::float(&mut e, caster, &[(ManaColor::Black, 1)]);
        cast_with_floating(&mut e, caster, index::THOUGHTSEIZE);
        e.apply(caster, PlayerAction::ChoosePlayer(p0)).unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        e.apply(
            caster,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .unwrap();
        lk::settle(&mut e);
        let tokens = tokens_of(&e, p0);
        assert_eq!(tokens.len(), if opponent && discard_this { 4 } else { 0 });
        for token in tokens {
            assert_eq!(pt(&e, token), (1, 1));
        }
    }
}
