//! `cards/sorceries/mv_4/forbidden_ritual.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn forbidden_ritual() -> CardIndex {
    card_index("4b5d9c76-8be8-4e25-b978-9750df3d8c15")
}

/// P0: four Swamps, Grizzly Bears and the Ritual in hand. P1: Grizzly Bears, a
/// Swamp and a Mountain in hand. The Ritual is cast at P1 and its first
/// question is waiting.
fn ritual_is_cast() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), GRIZZLY_BEARS])
        .battlefield(1, &[GRIZZLY_BEARS, swamp()])
        .hand(0, &[forbidden_ritual()])
        .hand(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, forbidden_ritual());
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("expected target opponent choice, got {other:?}"),
    }
    engine
}

/// Runs to the next question that is a card choice or a yes-or-no, without
/// answering the yes-or-no for the player the way `pass_until` would.
fn next_question(engine: &mut Engine<RegistryLookup>) -> Pending {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::YesNo { .. }
        )
    });
    engine.pending().clone()
}

/// The card choice that is waiting: for whom, and among what.
fn card_choice(engine: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    let Pending::ChooseCards {
        player, options, ..
    } = next_question(engine)
    else {
        panic!("expected a card choice, got {:?}", engine.pending())
    };
    (player, options)
}

fn choose(engine: &mut Engine<RegistryLookup>, who: PlayerId, objects: Vec<ObjectId>) {
    engine
        .apply(who, PlayerAction::ChooseObjects { objects })
        .unwrap();
}

/// P0's first pass: asked what to sacrifice (their own nontoken permanents
/// only), sacrifices the Bears. Returns the opponent's Bears and Swamp.
fn sacrifice_the_bears(engine: &mut Engine<RegistryLookup>) -> (ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let own_bears = on_battlefield(engine, p0, GRIZZLY_BEARS).unwrap();
    let their_bears = on_battlefield(engine, p1, GRIZZLY_BEARS).unwrap();
    let their_swamp = on_battlefield(engine, p1, swamp()).unwrap();
    let (who, options) = card_choice(engine);
    assert_eq!(who, p0, "the caster sacrifices");
    assert_eq!(options.len(), 5, "four Swamps and Bears: {options:?}");
    assert!(
        !options.contains(&their_bears) && !options.contains(&their_swamp),
        "never the opponent's permanents"
    );
    choose(engine, p0, vec![own_bears]);
    (their_bears, their_swamp)
}

/// P1's two questions, in order: sacrifice a permanent (their own two are on
/// the menu), then discard a card (their one card is).
fn opponent_may_sacrifice(engine: &mut Engine<RegistryLookup>, their: (ObjectId, ObjectId)) {
    let (who, options) = card_choice(engine);
    assert_eq!(
        who,
        PlayerId::new(1),
        "the opponent chooses, not the caster"
    );
    assert_eq!(
        options.len(),
        2,
        "any permanent of theirs, their choice: {options:?}"
    );
    assert!(options.contains(&their.0) && options.contains(&their.1));
}

fn opponent_may_discard(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let (who, options) = card_choice(engine);
    assert_eq!(who, PlayerId::new(1));
    assert_eq!(options.len(), 1, "their one card in hand: {options:?}");
    options[0]
}

fn caster_is_asked_to_repeat(engine: &mut Engine<RegistryLookup>) {
    let Pending::YesNo { player, prompt, .. } = next_question(engine) else {
        panic!("expected the repeat question, got {:?}", engine.pending())
    };
    assert_eq!(player, PlayerId::new(0), "the caster decides to repeat");
    assert_eq!(prompt, crate::choice::YesNoPrompt::MayDo);
}

fn life(engine: &Engine<RegistryLookup>, seat: u8) -> i32 {
    engine.state().players[usize::from(seat)].life
}

/// `Forbidden Ritual` ({2}{B}{B}, Sorcery, `Coverage::Implemented`), the
/// opponent declines both ways out: the Bears are sacrificed, the opponent
/// loses 2 and keeps their permanents and their card, and the caster is asked
/// whether to go again.
#[test]
fn forbidden_ritual_costs_the_opponent_two_life_when_they_pay_nothing_else() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = ritual_is_cast();
    let theirs = sacrifice_the_bears(&mut engine);
    opponent_may_sacrifice(&mut engine, theirs);
    choose(&mut engine, p1, vec![]);
    let _ = opponent_may_discard(&mut engine);
    choose(&mut engine, p1, vec![]);

    caster_is_asked_to_repeat(&mut engine);
    assert_eq!(life(&engine, 1), 18, "exactly 2 life");
    assert_eq!(life(&engine, 0), 20, "the caster loses nothing");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some(),
        "the sacrificed permanent"
    );
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, swamp()).is_some());
    assert!(
        in_hand(&engine, p1, mountain()).is_some(),
        "nothing discarded"
    );
    assert_eq!(life(&engine, 1), 18, "and no second loss after declining");
}

/// "...unless that player sacrifices a permanent of their choice": the
/// opponent gives up their Swamp; they lose no life, are not asked about a
/// discard, and keep their card.
#[test]
fn forbidden_ritual_lets_the_opponent_sacrifice_a_permanent_instead_of_losing_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = ritual_is_cast();
    let theirs = sacrifice_the_bears(&mut engine);
    opponent_may_sacrifice(&mut engine, theirs);
    choose(&mut engine, p1, vec![theirs.1]);

    caster_is_asked_to_repeat(&mut engine);
    assert_eq!(
        life(&engine, 1),
        20,
        "paid with a permanent: no life lost, and no discard question in between"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, swamp()).is_some(), "their Swamp");
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(
        in_hand(&engine, p1, mountain()).is_some(),
        "nothing discarded"
    );
}

/// "...or discards a card": declined the sacrifice, discarded the card; no
/// life lost and both permanents stay.
#[test]
fn forbidden_ritual_lets_the_opponent_discard_a_card_instead_of_losing_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = ritual_is_cast();
    let theirs = sacrifice_the_bears(&mut engine);
    opponent_may_sacrifice(&mut engine, theirs);
    choose(&mut engine, p1, vec![]);
    let card = opponent_may_discard(&mut engine);
    choose(&mut engine, p1, vec![card]);

    caster_is_asked_to_repeat(&mut engine);
    assert_eq!(life(&engine, 1), 20, "paid with a card: no life lost");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, mountain()).is_some(),
        "the discard"
    );
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, swamp()).is_some());
}

/// "You may repeat this process any number of times": saying yes sacrifices a
/// second permanent of the caster's and offers the opponent the same three
/// ways out; the opponent pays 2 life twice, and the caster stops when they
/// say no.
#[test]
fn forbidden_ritual_can_be_repeated_and_the_opponent_pays_each_time() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = ritual_is_cast();
    let theirs = sacrifice_the_bears(&mut engine);
    opponent_may_sacrifice(&mut engine, theirs);
    choose(&mut engine, p1, vec![]);
    let _ = opponent_may_discard(&mut engine);
    choose(&mut engine, p1, vec![]);
    caster_is_asked_to_repeat(&mut engine);
    assert_eq!(life(&engine, 1), 18);

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let (who, options) = card_choice(&mut engine);
    assert_eq!(who, p0, "the second sacrifice is the caster's");
    assert_eq!(
        options.len(),
        4,
        "the four Swamps are what is left: {options:?}"
    );
    choose(&mut engine, p0, vec![options[0]]);
    opponent_may_sacrifice(&mut engine, theirs);
    choose(&mut engine, p1, vec![]);
    let _ = opponent_may_discard(&mut engine);
    choose(&mut engine, p1, vec![]);

    caster_is_asked_to_repeat(&mut engine);
    assert_eq!(life(&engine, 1), 16, "2 more life");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life(&engine, 1), 16, "nothing after the caster stops");
    assert_eq!(life(&engine, 0), 20);
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, swamp()).is_some());
}
