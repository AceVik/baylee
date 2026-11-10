//! `cards/instants/mv_1/final_showdown.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// ---------------------------------------------------------------------------
// Maik's European Highlander: Final Showdown and Three Steps Ahead (spree).
// ---------------------------------------------------------------------------

/// "Spree (Choose one or more additional costs.)" Off three Plains, Final
/// Showdown's {W} with one "+ {1}" or both is what the pool pays for, and
/// "+ {3}{W}{W}" is not offered at all: each row is {W} plus its modes' own
/// costs (CR 702.172a, 700.2h). The second mode alone, with no creature to
/// choose, resolves without a question (CR 609.3) and all {1}{W} is spent.
#[test]
fn final_showdown_offers_the_sets_of_modes_its_mana_pays_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 3])
        .hand(0, &[final_showdown()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_mana_where(&mut engine, p0, |_| true);
    cast_with_floating(&mut engine, p0, final_showdown());
    let offered = choose_modes(&mut engine, p0, 0b010);
    let cost = baylee_core::mana::ManaCost::parse;
    assert_eq!(
        offered.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Modes(0b001), cost("{1}{W}")),
            (CastModeKind::Modes(0b010), cost("{1}{W}")),
            (CastModeKind::Modes(0b011), cost("{2}{W}")),
        ]
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p0, final_showdown()).is_some());
}

/// All three modes, in the order they are printed (CR 608.2c): every
/// creature loses its abilities, then the chosen one gains indestructible,
/// then all creatures are destroyed. The Darksteel Gargoyle's printed
/// indestructible is gone by then and it dies; the Elf that was chosen was
/// given indestructible after the abilities were taken, and lives. The
/// choice is a choice and not a target: only the caster's own creatures are
/// offered, one of them must be taken (CR 608.2d), and it costs
/// {5}{W}{W}{W} in all.
#[test]
fn final_showdown_in_full_spares_only_the_creature_it_chose() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut mine = vec![plains(); 8];
    mine.extend([llanowar_elves(), llanowar_elves()]);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &mine)
        .battlefield(1, &[darksteel_gargoyle(), llanowar_elves()])
        .hand(0, &[final_showdown()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, final_showdown());
    let offered = choose_modes(&mut engine, p0, 0b111);
    assert!(offered.iter().any(|o| o.kind == CastModeKind::Modes(0b111)
        && o.cost == baylee_core::mana::ManaCost::parse("{5}{W}{W}{W}")));
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!()
    };
    let mine: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == p0 && o.card.is_some_and(|c| c.index == llanowar_elves())
            })
        })
        .collect();
    assert_eq!((player, min, max), (p0, 1, 1));
    assert_eq!(options, mine, "the caster's own creatures, and only those");
    let (kept, lost) = (mine[1], mine[0]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![kept],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().object(kept).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert!(keywords_of(&engine, kept).contains(KeywordSet::INDESTRUCTIBLE));
    assert_ne!(
        engine.state().object(lost).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert!(in_graveyard(&engine, p1, darksteel_gargoyle()).is_some());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}
