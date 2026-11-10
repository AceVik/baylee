//! `cards/creatures/mv_4/nightscape_master.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e5f9bd6b-0aa6-4f61-b101-ec194af6d632"

/// Nightscape Master is a 2/2 for {2}{B}{B} printing two activated abilities:
/// "{U}{U}, {T}: Return target creature to its owner's hand" and
/// "{R}{R}, {T}: This creature deals 2 damage to target creature."
///
/// Both are played on one board, because the tap symbol is half of both prices
/// and a single Wizard can pay for only one of them between untap steps. The two
/// copies are seated rather than cast, the way the equipment tests seat theirs:
/// the {2}{B}{B} would spend the very Islands and Mountains the two abilities
/// are measured against, and a creature cast this turn has not been under its
/// controller's control since the turn began (CR 302.6), so it could not pay
/// either tap. What the board proves is the pair of prices — two blue and then
/// two red leave the pool while the two black stay — and the pair of effects,
/// read off a hand, a graveyard and two life totals that never moved.
#[test]
#[allow(clippy::too_many_lines)] // two printed abilities, each with a price and a target question
fn nightscape_master_pays_two_mana_of_its_own_colors_and_its_tap_for_each_ability() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nightscape_master(),
                nightscape_master(),
                swamp(),
                swamp(),
                island(),
                island(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wizards = all_on_battlefield(&engine, p0, nightscape_master());
    assert_eq!(wizards.len(), 2, "two Wizards stand on the table");
    for wizard in &wizards {
        assert_eq!(pt(&engine, *wizard), (2, 2), "the body the card prints");
        assert!(
            types(&engine, *wizard).contains(TypeSet::CREATURE),
            "and it is a creature: {:?}",
            types(&engine, *wizard)
        );
    }

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // mana pool rather than the untapped lands: with nothing floating neither
    // {U}{U} nor {R}{R} is payable, so neither line is offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| wizards.contains(src)),
        "an empty pool pays no two coloured mana: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 6, "two Swamps, two Islands, two Mountains");
    assert_eq!(pool.available(ManaColor::Blue), 2, "two blue");
    assert_eq!(pool.available(ManaColor::Red), 2, "two red");
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "and two black that neither line of the card asks for"
    );

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves across the table");
    let (bounced, kept) = (elves[0], elves[1]);

    // Ability 0 is the blue line in the card def: "{U}{U}, {T}: Return target
    // creature to its owner's hand." CR 601.2c names the target before
    // CR 601.2h pays, so while the question stands every Wizard is still
    // standing and all six mana is still floating.
    activate(&mut engine, p0, nightscape_master(), 0);
    assert!(
        wizards.iter().all(|id| !is_tapped(&engine, *id)),
        "the cost is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "and the mana is still in the pool for the same reason"
    );
    let menu = aim_at(&mut engine, p0, bounced);
    assert!(
        menu.contains(&bounced) && menu.contains(&kept),
        "\"target creature\" names no side of the table: {menu:?}"
    );

    assert_eq!(
        wizards.iter().filter(|id| is_tapped(&engine, **id)).count(),
        1,
        "one activation, one {{T}} paid"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0, "and two blue with it");
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "while the two black are untouched: the price is {{U}}{{U}} and not two \
         mana of any kind"
    );
    assert_eq!(pool.total(), 4, "six less the two the price costs");

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&bounced),
        "\"to its owner's hand\": the Elf the question named is in the hand of \
         the seat that owns it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, llanowar_elves()),
        vec![kept],
        "and the creature the ability did not name never moved"
    );

    // Ability 1: "{R}{R}, {T}: This creature deals 2 damage to target
    // creature." The other Wizard is the one that can pay it, because the tap
    // symbol is half of both prices and the first one has spent its own — a
    // tapped permanent is not in the offer at all.
    let acting = wizards
        .iter()
        .copied()
        .find(|id| !is_tapped(&engine, *id))
        .expect("the Wizard that has not tapped yet");
    activate(&mut engine, p0, nightscape_master(), 1);
    let menu = aim_at(&mut engine, p0, kept);
    assert!(
        menu.contains(&kept) && menu.contains(&acting),
        "\"target creature\" is any creature, the Wizard dealing the damage \
         included: {menu:?}"
    );

    assert_eq!(
        wizards.iter().filter(|id| is_tapped(&engine, **id)).count(),
        2,
        "both lines of the card have now been paid for, one tap each"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0, "and the two red with it");
    assert_eq!(
        pool.total(),
        2,
        "lifting two and then two out of the six, leaving the black"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature and never to the seat whose board it \
         stood on"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nor to the seat that paid for it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, nightscape_master()).len(),
        2,
        "and the Wizard that dealt it is still standing, because the ability \
         was aimed at the Elf"
    );
}
