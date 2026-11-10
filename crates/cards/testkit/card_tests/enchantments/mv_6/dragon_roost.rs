//! `cards/enchantments/mv_6/dragon_roost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "6dc98143-7c4c-4b75-9bbb-5226d800b1d6"

/// Dragon Roost prints one line — "{5}{R}{R}: Create a 5/5 red Dragon creature
/// token with flying" — and every word of it has to be read off a board rather
/// than off the card file. Thirteen Mountains pay the {4}{R}{R} that brings the
/// enchantment to the table and leave exactly the seven the ability charges, so
/// the empty pool afterwards says the printed price was really paid, and the
/// token that arrives is checked for the body, the creature type, the keyword
/// and the colour the sentence names — a colorless 0/0 would satisfy "a token
/// was created" and nothing else. The Roost still standing after the activation
/// is the other half: the ability costs mana and no sacrifice, so the same
/// enchantment is a repeatable engine rather than a one-shot.
#[test]
fn dragon_roost_spends_seven_mana_for_a_five_five_red_flying_dragon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 13])
        .hand(0, &[dragon_roost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool rather than the thirteen untapped Mountains: with nothing floating
    // the {4}{R}{R} is unpayable, so the Roost is not among the castable cards
    // at all.
    let card = in_hand(&engine, p0, dragon_roost()).expect("the Roost is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}{{R}}{{R}}, so the Roost is not offered: {:?}",
        legal.castable
    );

    // Thirteen Mountains into the pool: {4}{R}{R} brings the enchantment to the
    // table and the {5}{R}{R} the ability charges is what is left floating
    // beside it — a pool survives until the step ends (CR 500.5) and this whole
    // scenario plays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        13,
        "thirteen Mountains tapped, thirteen red"
    );
    cast_with_floating(&mut engine, p0, dragon_roost());
    pass_until(&mut engine, stack_is_empty);

    let roost = on_battlefield(&engine, p0, dragon_roost()).expect("the Roost resolved");
    assert!(
        types(&engine, roost).contains(TypeSet::ENCHANTMENT),
        "what arrived is the enchantment the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the cast's {{4}}{{R}}{{R}} is spent and exactly the seven the ability \
         charges is left"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so no Dragon has been made"
    );

    // `LegalActions::abilities` is filtered through `can_afford` too, which is
    // why the claim about the offer is made with the seven already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(roost, 0)),
        "with seven red floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dragon_roost(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{5}}{{R}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Dragon arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Dragon");
    let dragon = tokens[0];
    let kinds = types(&engine, dragon);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "\"creature token\": {kinds:?}"
    );
    assert_eq!(
        pt(&engine, dragon),
        (5, 5),
        "the body the card prints, read off the battlefield"
    );
    assert!(
        keywords(&engine, dragon).contains(KeywordSet::FLYING),
        "\"with flying\" — the keyword reaches the permanent the ability made"
    );

    // The token's own ledger, which the projection cannot show: the name the
    // card gives it and the colour of a Dragon that is red and nothing else.
    let printed = engine
        .state()
        .object(dragon)
        .expect("the Dragon is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Dragon", "the name the card gives it");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Red),
        "\"red Dragon\" — and never a colorless body with the right numbers"
    );

    assert!(
        on_battlefield(&engine, p0, dragon_roost()).is_some(),
        "the price was mana and no sacrifice, so the Roost is still standing to \
         make another Dragon"
    );
}
