//! `cards/lands/gates/gateway_plaza.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Every row: the land is played, the price is named, and what was named
/// has paid.
///
/// The assertion is on the **object**, not on the ability's effect: what
/// this rule wrote is the cost, and a sacrifice that drew a card while
/// leaving the creature on the battlefield is exactly the failure a test on
/// "did I draw" would pass. Which zone the object lands in is read off the
/// part it paid, because a discard and a sacrifice both reach a graveyard
/// and a tap reaches nothing at all.
#[test]
fn a_land_whose_cost_names_an_object_is_paid_with_that_object() {
    for (i, (oracle, feed)) in PAYS_BY_NAMING_AN_OBJECT.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 960 + u64::try_from(i).expect("eleven rows");
        // Enough of every colour for the dearest of them, `{4}{R}`.
        let mut board = vec![
            plains(),
            plains(),
            island(),
            island(),
            swamp(),
            swamp(),
            mountain(),
            mountain(),
            forest(),
            forest(),
        ];
        let mut hand = vec![card];
        match feed {
            Feed::Strix => board.push(baleful_strix()),
            Feed::Plaza => board.push(gateway_plaza()),
            Feed::HandCard => hand.push(plains()),
            Feed::Itself => {}
        }
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &board)
            .hand(0, &hand)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        // Round the turn before activating. Three of these lands come down
        // tapped — The Shire unless you control a legendary creature, and
        // it is not one — and their own `{T}` is part of the price, so a
        // test that activated the turn they arrived would be measuring
        // summoning-sick lands rather than costs.
        cross_into_the_next_own_main(&mut engine, p0);
        // Everything but the land itself: its `{T}` is part of the price.
        // And never the Plaza. It is the one thing on this board that can
        // answer Heap Gate's "tap an untapped Gate you control", because
        // Heap Gate cannot be that Gate: its own `{T}` has already spent it
        // (CR 118.3). This row used to pass with the Plaza tapped, paying
        // the Gate with the source a second time, which is the defect
        // `cost_wizard::menu` closes.
        let plaza = on_battlefield(&engine, p0, gateway_plaza());
        tap_mana_where(&mut engine, p0, |id| id != land && Some(id) != plaza);

        let (source, index) = ability_that_asks(&engine, card)
            .unwrap_or_else(|| panic!("{oracle} offers no ability that asks for an object"));
        assert_eq!(source, land, "the ability is on the land just played");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: index,
                },
            )
            .unwrap();

        // CR 601.2c puts targets before costs, so three of these rows ask
        // where the ability points before they ask what pays for it. The
        // first legal answer will do — this test is about the price.
        if let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = engine.pending().clone()
        {
            let (objects, players) = match (options.first(), player_options.first()) {
                (Some(&object), _) => (vec![object], vec![]),
                (None, Some(&player)) => (vec![], vec![player]),
                (None, None) => panic!("{oracle} asked for a target and offered none"),
            };
            engine
                .apply(p0, PlayerAction::ChooseTargets { objects, players })
                .unwrap();
        }
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "{oracle} asked no cost question, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(player, p0);
        assert_eq!(
            (min, max),
            (1, 1),
            "an activation cost is not optional — one object, and exactly one"
        );
        if matches!(feed, Feed::Strix | Feed::Itself) {
            no_basic_land_on_the_menu(&engine, &options, oracle);
        }
        let paid = *options.first().unwrap_or_else(|| {
            panic!("{oracle} put an empty menu up, which is a dead end for a client")
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![paid],
                },
            )
            .unwrap();

        the_object_paid(&engine, card, index, paid, p0, oracle);
    }
}
