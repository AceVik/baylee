//! `cards/enchantments/mv_5/dispersing_orb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "dabc4ba1-3f90-4cea-a737-d41537cce729"

/// Dispersing Orb prints one ability — "{3}{U}, Sacrifice a permanent: Return
/// target permanent to its owner's hand" — and both of its halves are the
/// engine's answer rather than the card's. The printed price is read off a pool
/// the nine Islands actually filled ({3}{U}{U} for the enchantment and {3}{U}
/// for the ability, one main phase and therefore one pool, CR 500.5), the
/// sacrifice menu as the permanents *this* seat controls — the Orb itself among
/// them, because "a permanent" says nothing about "another" — and the target
/// menu as every permanent on either side of the table, CR 601.2c naming the
/// target before CR 601.2h pays for it. What the bounce then does is the half
/// no board reading can see: the Elf across the table leaves the battlefield
/// and arrives in **its owner's** hand while the Orb that paid for it is in its
/// owner's graveyard.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
fn dispersing_orb_sacrifices_a_permanent_it_controls_to_return_any_permanent_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 9])
        .hand(0, &[dispersing_orb()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nine Islands are exactly the {3}{U}{U} the enchantment costs plus the
    // {3}{U} its ability charges, and the whole scenario plays inside this one
    // main phase, so CR 500.5 empties the pool only at the end of it.
    let card = in_hand(&engine, p0, dispersing_orb()).expect("the Orb is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{U}}{{U}}, and `can_afford` reads the pool \
         rather than the nine untapped Islands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Islands tapped, and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, dispersing_orb());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let orb = on_battlefield(&engine, p0, dispersing_orb()).expect("the Orb resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{3}}{{U}}{{U}} is spent and exactly the {{3}}{{U}} the ability \
         charges is left floating"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let land = on_battlefield(&engine, p0, island()).expect("an Island of mine is out");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — which is why the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(orb, 0)),
        "the one line the card prints, now that its {{3}}{{U}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dispersing_orb(), 0);

    // The two questions one activation asks: which permanent is aimed at
    // (CR 601.2c) and which permanent is being given up (CR 601.2h). Answered
    // in whichever order they arrive, and each menu is read on the spot.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one permanent, and the ability asks once"
                );
                target_menu = options;
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .expect("the permanent across the table was one of the options");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
                sacrifice_menu = options;
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![orb] })
                    .expect("the Orb is a permanent this seat controls, so it may eat itself");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Orb's activation resolves: {other:?}"),
        }
    }

    assert!(
        target_menu.contains(&elf) && target_menu.contains(&land),
        "\"target permanent\" is any permanent, on either side of the table: {target_menu:?}"
    );
    assert_eq!(
        sacrifice_menu.len(),
        10,
        "the Orb and the nine Islands are every permanent this seat controls: {sacrifice_menu:?}"
    );
    assert!(
        sacrifice_menu.contains(&orb),
        "\"Sacrifice a permanent\" does not say \"another\": the source is on \
         its own menu: {sacrifice_menu:?}"
    );
    assert!(
        sacrifice_menu.contains(&land),
        "a land is as much a permanent as the enchantment that asks: {sacrifice_menu:?}"
    );
    assert!(
        !sacrifice_menu.contains(&elf),
        "`CR 701.21a`: an opponent's permanent is not yours to sacrifice: {sacrifice_menu:?}"
    );

    // CR 601.2c before CR 601.2h: with the target named and the sacrifice the
    // last price left, the {3}{U} is what the pool is missing.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "bouncing a permanent is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, dispersing_orb()).is_some(),
        "the sacrificed permanent is in its owner's graveyard"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        9,
        "and only the permanent that was named: every Island is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\" — the card went back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and not to the seat that aimed the bounce"
    );
}
