//! `cards/creatures/mv_4/kess_dissident_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

/// Kess, Dissident Mage prints two sentences and the DSL carries the first
/// one: `Flying`, with the once-per-turn graveyard cast left to the
/// `// NOT SUPPORTED` line. What this plays is that half — she is cast off
/// `{1}{U}{B}{R}`, resolves, and stands as the printed 3/4 with flying
/// projected onto her by the keyword itself and by nothing else on the
/// board. The Llanowar Elves beside her are the counter-half: a projection
/// that had lost flying's subject and handed it to every creature under the
/// seat would read exactly the same on Kess alone, and reads wrong here.
#[test]
fn kess_resolves_as_a_flying_three_four_and_grants_flying_to_nobody_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, island())
        .battlefield(
            0,
            &[island(), swamp(), mountain(), mountain(), llanowar_elves()],
        )
        .hand(0, &[kess_dissident_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the ground-bound bystander flies before she lands"
    );

    // The four lands pay {1}{U}{B}{R} exactly. The Elf is kept back so that
    // "exactly" stays true: it makes a fifth mana that would still be
    // floating at the assertion below.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, kess_dissident_mage());
    pass_until(&mut engine, stack_is_empty);

    let kess = on_battlefield(&engine, p0, kess_dissident_mage()).expect("she resolved");
    assert_eq!(pt(&engine, kess), (3, 4), "the body the card prints");
    let types = engine
        .state()
        .object(kess)
        .expect("she is still an object")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::CREATURE),
        "a legendary Human Wizard is a creature: {types:?}"
    );
    assert!(
        keywords(&engine, kess).contains(KeywordSet::FLYING),
        "\"Flying\" is the half of the card the DSL carries"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}}{{B}}{{R}} was paid out of the pool, not left floating"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "and flying stayed where it was printed: the Elves are still on the ground"
    );
}

/// Kess in play for `p0`, whose hand holds the instant, the sorcery and the
/// creature that go to the graveyard, with `p1`'s hand as given.
fn kess_with_a_full_graveyard(
    p1_hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[index::KESS_DISSIDENT_MAGE],
        &[],
        &[
            index::GIANT_GROWTH,
            index::COUNSEL_OF_THE_SORATAMI,
            index::GRIZZLY_BEARS,
        ],
        p1_hand,
    );
    let instant = hand_to_graveyard(&mut e, p0, index::GIANT_GROWTH);
    let sorcery = hand_to_graveyard(&mut e, p0, index::COUNSEL_OF_THE_SORATAMI);
    let creature = hand_to_graveyard(&mut e, p0, index::GRIZZLY_BEARS);
    (e, instant, sorcery, creature)
}

fn offered_to_cast(e: &Engine<RegistryLookup>, seat: PlayerId, card: ObjectId) -> bool {
    matches!(e.pending(), Pending::Priority { player, legal }
        if *player == seat && legal.castable.contains(&card))
}

fn all_the_mana(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    kit::float(
        e,
        seat,
        &[
            (ManaColor::White, 3),
            (ManaColor::Blue, 3),
            (ManaColor::Black, 3),
            (ManaColor::Red, 3),
            (ManaColor::Green, 3),
        ],
    );
}

/// "You may cast an instant or sorcery spell from your graveyard": the
/// instant and the sorcery of your own graveyard, not the creature beside
/// them and not an instant lying in the opponent's graveyard.
#[test]
fn kess_offers_your_instants_and_sorceries_and_nothing_else_in_any_graveyard() {
    let p1 = kit::P1;
    let (mut e, instant, sorcery, creature) = kess_with_a_full_graveyard(&[index::LIGHTNING_BOLT]);
    let theirs = hand_to_graveyard(&mut e, p1, index::LIGHTNING_BOLT);
    all_the_mana(&mut e, kit::P0);
    assert!(
        offered_to_cast(&e, kit::P0, instant),
        "your instant may be cast from your graveyard"
    );
    assert!(offered_to_cast(&e, kit::P0, sorcery), "and your sorcery");
    assert!(
        !offered_to_cast(&e, kit::P0, creature),
        "a creature card is neither instant nor sorcery"
    );
    assert!(
        !offered_to_cast(&e, kit::P0, theirs),
        "\"from your graveyard\": the opponent's instant is out of reach"
    );
}

/// "Once during each of your turns": one cast, then none until your next
/// turn. The spell cast is exiled instead of going to the graveyard.
#[test]
fn kess_allows_one_cast_per_turn_and_the_next_turn_another() {
    let p0 = kit::P0;
    let (mut e, instant, sorcery, _creature) = kess_with_a_full_graveyard(&[]);
    let kess = kit::obj(&e, p0, index::KESS_DISSIDENT_MAGE);
    all_the_mana(&mut e, p0);
    e.apply(p0, PlayerAction::CastSpell { card: instant })
        .unwrap();
    kit::aim(&mut e, &[kess], &[]);
    kit::settle(&mut e);
    assert_eq!(
        kit::zone_of(&e, instant),
        Zone::Exile,
        "\"exile it instead\" of putting it into the graveyard"
    );

    all_the_mana(&mut e, p0);
    assert!(
        !offered_to_cast(&e, p0, sorcery),
        "\"once during each of your turns\": the second cast of this turn is refused"
    );

    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    all_the_mana(&mut e, p0);
    assert!(
        offered_to_cast(&e, p0, sorcery),
        "a new turn of yours, a new cast"
    );
}

/// "During each of YOUR turns": on the opponent's turn the instant stays put,
/// mana or no mana.
#[test]
fn kess_does_not_open_the_graveyard_on_the_opponents_turn() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut e, instant, _sorcery, _creature) = kess_with_a_full_graveyard(&[]);
    all_the_mana(&mut e, p0);
    assert!(
        offered_to_cast(&e, p0, instant),
        "on your own turn the same instant is offered"
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    all_the_mana(&mut e, p0);
    assert!(
        !offered_to_cast(&e, p0, instant),
        "an instant could be cast now by timing, but Kess works only in your own turns"
    );
}

/// "If a spell cast this way would be put into your graveyard, exile it
/// instead": a spell cast this way and countered goes to exile as well.
#[test]
fn kess_exiles_the_spell_she_cast_even_when_it_is_countered() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut e, instant, _sorcery, _creature) = kess_with_a_full_graveyard(&[index::COUNTERSPELL]);
    let kess = kit::obj(&e, p0, index::KESS_DISSIDENT_MAGE);
    all_the_mana(&mut e, p0);
    e.apply(p0, PlayerAction::CastSpell { card: instant })
        .unwrap();
    kit::aim(&mut e, &[kess], &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    kit::float(&mut e, p1, &[(ManaColor::Blue, 2)]);
    kit::cast_at(&mut e, p1, index::COUNTERSPELL, &[instant], &[]);
    assert_eq!(
        kit::zone_of(&e, instant),
        Zone::Exile,
        "countered, it would go to the graveyard, so it is exiled instead"
    );
}

/// Kess's permission is her controller's: the opponent's Kess opens nobody
/// else's graveyard.
#[test]
fn kess_opens_only_her_controllers_graveyard() {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[],
        &[index::KESS_DISSIDENT_MAGE],
        &[index::GIANT_GROWTH],
        &[],
    );
    let instant = hand_to_graveyard(&mut e, p0, index::GIANT_GROWTH);
    all_the_mana(&mut e, p0);
    assert!(
        !offered_to_cast(&e, p0, instant),
        "the opponent's Kess gives p0 nothing: p0 has no Kess"
    );
}
