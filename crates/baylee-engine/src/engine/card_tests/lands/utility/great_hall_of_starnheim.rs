//! `cards/lands/utility/great_hall_of_starnheim.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Hall of Starnheim prints `This land enters tapped.`, `{{T}}: Add {{B}}.`, and `{{W}}{{W}}{{B}}, {{T}}, Sacrifice this land and a creature you control: Create a 4/4 white Angel Warrior creature token with flying and vigilance. Activate only as a sorcery.`
///
/// The land enters tapped and taps for black mana, and this is the test of
/// the **offer**: after untapping, with `{{W}}{{W}}{{B}}` floating from two
/// `plains()` and one `swamp()` and a creature standing for the second half
/// of the cost, both abilities are on the list. Activating ability 0 adds
/// one black mana and takes the other line away with the `{{T}}` it shares.
#[test]
fn great_hall_of_starnheim_enters_tapped_and_offers_its_angel_once_it_untaps() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), swamp(), quiet_creature()])
        .hand(0, &[great_hall_of_starnheim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = play_land(&mut engine, p0, great_hall_of_starnheim());
    assert!(
        entered_tapped(&engine, hall),
        "great hall of starnheim enters tapped"
    );

    // Advance to the next turn so Great Hall of Starnheim untaps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, hall), "untaps on next turn");

    // Float {{W}}{{W}}{{B}} — plus the Elves' own {{G}}, which is why this is
    // four — while keeping Great Hall untapped.
    tap_mana_except(&mut engine, p0, hall);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 2);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 4);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(hall, 0)),
        "ability 0 ({{T}}: Add {{B}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(hall, 1)),
        "with {{W}}{{W}}{{B}} floating, a creature to sacrifice and the Hall untapped, \
         the whole printed price can be paid"
    );

    activate(&mut engine, p0, great_hall_of_starnheim(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2);
    assert_eq!(pool.available(ManaColor::White), 2);
    assert_eq!(pool.total(), 5, "and the Elves' {{G}} beside them");
    assert!(is_tapped(&engine, hall));
}

/// Great Hall of Starnheim charges two sacrifices in one cost: "{W}{W}{B},
/// {T}, Sacrifice this land and a creature you control: Create a 4/4 white
/// Angel Warrior creature token with flying and vigilance." The land pays
/// one of them itself and may not be offered for the other — the menu for
/// the creature half has exactly the Elves in it — and the Angel that comes
/// out carries both keywords, which is what separates the pool's two 4/4
/// white flying Angel Warriors from each other.
#[test]
fn great_hall_of_starnheim_spends_itself_and_a_creature_on_a_vigilant_angel() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9104, forest())
        .battlefield(
            0,
            &[
                great_hall_of_starnheim(),
                plains(),
                plains(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    // A turn each way, so the Hall — which prints "enters tapped" — is
    // standing whatever the harness did with it as the game was set up.
    reach_their_main_phase(&mut engine, p1);
    walk_to_own_main(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, great_hall_of_starnheim()).expect("the Hall is seated");
    assert!(!is_tapped(&engine, hall), "it untapped in the untap step");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");

    tap_mana_except(&mut engine, p0, hall);
    activate(&mut engine, p0, great_hall_of_starnheim(), 1);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the sacrifice choice, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1), "one creature, and it is not optional");
    assert_eq!(
        options,
        vec![elves],
        "the Hall pays its own half of the cost and is not offered for the other"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves are a legal answer");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, great_hall_of_starnheim()).is_none()
            && on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "both halves of the cost were paid"
    );
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 1, "one Angel");
    let angel = made[0];
    assert_eq!(pt(&engine, angel), (4, 4));
    let wings = keywords(&engine, angel);
    assert!(
        wings.contains(KeywordSet::FLYING) && wings.contains(KeywordSet::VIGILANCE),
        "\"with flying and vigilance\": {wings:?}"
    );
    let printed = engine
        .state()
        .object(angel)
        .expect("the Angel is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Angel Warrior", "not a plain Angel");
}
