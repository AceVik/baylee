//! `cards/instants/mv_1/spell_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spell Blast — {X}{U}: "Counter target spell with mana value X." X is
/// announced before targets are chosen (CR 601.2b, 601.2c), so the target
/// menu is the spells whose mana value is the X just announced — and before
/// any X, a spell of any mana value is something to cast it at. Both halves
/// used to read the card's X as it lay in hand, 0: Spell Blast was never
/// offered against Ondu Cleric (mana value 2) and could only ever be pointed
/// at a spell costing nothing.
#[test]
fn spell_blast_is_offered_for_any_spell_and_targets_the_ones_its_x_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spell_blast = card_index("04477339-7ed5-4770-9e5c-6e481ffcc858");
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[ondu_cleric()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[spell_blast])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let cleric = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: cleric })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let blast = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    assert!(
        legal.castable.contains(&blast),
        "Spell Blast was not offered against a spell of mana value 2"
    );

    // X = 1 names no spell on the stack: the cast finds no target and is
    // reversed, leaving the card in hand and priority where it was.
    engine
        .apply(p1, PlayerAction::CastSpell { card: blast })
        .unwrap();
    engine.apply(p1, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "X = 1 offered a target: {:?}",
        engine.pending()
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&blast)
    );

    // X = 2 names the cleric, and only it.
    engine
        .apply(p1, PlayerAction::CastSpell { card: blast })
        .unwrap();
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target menu, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric]);
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&cleric)
    });
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
}
