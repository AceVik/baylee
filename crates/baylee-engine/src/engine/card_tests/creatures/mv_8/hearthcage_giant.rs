//! `cards/creatures/mv_8/hearthcage_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hearthcage_giant() -> CardIndex {
    card_index("7e523d42-c7d8-47d1-8874-230920f03345")
}

/// Hearthcage Giant is a 5/5 for {6}{R}{R}; entering it makes two 3/1 red
/// Elemental Shaman tokens.
#[test]
fn hearthcage_giant_enters_with_two_three_one_elementals() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[mountain(); 8], &[hearthcage_giant()]);
    assert!(tokens_of(&engine, p0).is_empty());
    let giant = cast_and_settle(&mut engine, hearthcage_giant());
    assert_eq!(pt(&engine, giant), (5, 5));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "two tokens");
    for token in tokens {
        assert_eq!(pt(&engine, token), (3, 1));
        let printed = engine
            .state()
            .object(token)
            .and_then(|o| o.token)
            .expect("a token");
        assert!(printed.colors.contains(baylee_core::color::Color::Red));
        assert!(printed.name.contains("Elemental"), "{}", printed.name);
    }
}

/// "Sacrifice an Elemental: Target Giant creature gets +3/+1 until end of
/// turn": a token pays, the Giant (and only a Giant) is aimed at and grows to
/// 8/6.
#[test]
fn hearthcage_giant_sacrifices_an_elemental_to_pump_a_giant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 8])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[hearthcage_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let giant = cast_and_settle(&mut engine, hearthcage_giant());
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("their bear");
    let tokens = tokens_of(&engine, p0);

    activate(&mut engine, p0, hearthcage_giant(), 1);
    let mut aimed = false;
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => {
                assert!(options.iter().all(|o| tokens.contains(o)), "{options:?}");
                assert!(!options.contains(&giant), "the Giant is not an Elemental");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => {
                let menu = aim_at(&mut engine, p0, giant);
                assert!(!menu.contains(&bear), "only Giants: {menu:?}");
                aimed = true;
            }
            _ => break,
        }
    }
    assert!(aimed, "a target was asked for");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (8, 6), "+3/+1");
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "one Elemental was sacrificed"
    );
}
