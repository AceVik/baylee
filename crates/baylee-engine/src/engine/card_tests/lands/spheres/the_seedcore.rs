//! `cards/lands/spheres/the_seedcore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Seedcore prints `{{T}}: Add {{C}}.`, `{{T}}: Add one mana of any color.
/// Spend this mana only to cast Phyrexian creature spells.`, and `Corrupted — {{T}}:
/// Target 1/1 creature gets +2/+1 until end of turn. Activate only if an opponent
/// has three or more poison counters.`
///
/// Activating
/// ability 1 prompts for a color choice via `Pending::ChooseColor` and produces one
/// restricted mana in `pool.restricted()` rather than `pool.available()`.
#[test]
fn the_seedcore_produces_restricted_phyrexian_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_seedcore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let seedcore = on_battlefield(&engine, p0, the_seedcore()).expect("seedcore on battlefield");
    activate(&mut engine, p0, the_seedcore(), 1);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Green);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, seedcore));
}

#[test]
fn seedcore_requires_an_opponents_poison_and_exactly_one_one_at_both_target_checks() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (poison, change) in [(2, false), (3, false), (3, true)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[the_seedcore(), index::LLANOWAR_ELVES, index::GRIZZLY_BEARS],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, the_seedcore()).unwrap();
        let elf = on_battlefield(&e, p0, index::LLANOWAR_ELVES).unwrap();
        let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        let s = e.dev_state_mut(p0).unwrap();
        s.players[0].poison = 3;
        s.players[usize::from(p1.get())].poison = poison;
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.abilities.contains(&(land, 2)), poison >= 3);
        if poison < 3 {
            continue;
        }
        activate(&mut e, p0, the_seedcore(), 2);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("targets")
        };
        assert!(options.contains(&elf));
        assert!(!options.contains(&bear));
        aim_at(&mut e, p0, elf);
        if change {
            crate::replacement::put_counters(
                e.dev_state_mut(p0).unwrap(),
                elf,
                CounterKind::P1P1,
                1,
            );
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, elf), if change { (2, 2) } else { (3, 2) });
    }
}
