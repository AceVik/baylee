//! `cards/lands/check/mistrise_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mistrise Village enters tapped unless you control a Mountain or a Forest,
/// and prints `{T}: Add {U}`; the other half of its text — making the next
/// spell uncounterable — is the `Coverage::Partial` gap and nothing here
/// presses it. The entry clause is read on both sides of its own sentence:
/// over an Island alone the Village arrives tapped, and beside a single
/// Forest it arrives standing, which one game could not say on its own — a
/// printing that had simply lost the "unless" would look identical on the
/// board that satisfies it, and one that always entered tapped would look
/// identical on the board that does not. The standing Village is then tapped
/// for mana, and the blue is the reading that matters: the Forest beside it
/// is green, so there is no other source of {U} on the table.
#[test]
fn mistrise_village_enters_tapped_without_a_forest_and_taps_for_blue_beside_one() {
    let p0 = PlayerId::new(0);

    // No Mountain and no Forest: the printed replacement does fire. Played
    // rather than seeded, because `SeatSpec::starting_battlefield` places a
    // permanent instead of letting it enter, and a placement is exactly the
    // thing no entry modifier ever looks at.
    let mut bare = Duel::new(7, island())
        .battlefield(0, &[island()])
        .hand(0, &[mistrise_village()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    let tapped = play_land(&mut bare, p0, mistrise_village());
    assert!(
        entered_tapped(&bare, tapped),
        "an Island is neither a Mountain nor a Forest, so the Village enters tapped"
    );
    // And a tapped land is a land that makes nothing: the board floats the
    // Island's {U} alone.
    tap_all_mana(&mut bare, p0);
    assert_eq!(
        bare.state().players[0].mana_pool.total(),
        1,
        "the tapped Village contributes no mana"
    );

    // The other arm of the same sentence: one Forest is enough.
    let mut engine = Duel::new(7, island())
        .battlefield(0, &[forest()])
        .hand(0, &[mistrise_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let village = play_land(&mut engine, p0, mistrise_village());
    assert!(
        !entered_tapped(&engine, village),
        "a Forest beside it is exactly what the \"unless\" asks for"
    );

    // Mana first: the offer is read off the pool, not off the board.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}} — and the Forest beside it makes green, so the blue \
         has no other source on this board"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest, for contrast"
    );
    assert_eq!(pool.total(), 2, "two untapped lands, two mana");
    assert!(
        is_tapped(&engine, village),
        "paying which tapped the Village"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
}

/// Both seats float plenty of every colour, so a refused cast is never a
/// shortfall of mana.
fn flood_both_pools(e: &mut Engine<RegistryLookup>) {
    for player in &mut e.dev_state_mut(PlayerId::new(0)).unwrap().players {
        for color in ManaColor::ALL {
            player.mana_pool.add(color, 10);
        }
    }
    e.refresh_offer();
}

/// "The next spell you cast **this turn** can't be countered." The grant ends
/// with the turn: activated now and unused, a spell cast in the controller's
/// next turn is an ordinary spell and a Counterspell counters it.
#[test]
fn mistrise_village_protection_ends_with_the_turn_it_was_activated_in() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MISTRISE_VILLAGE])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    flood_both_pools(&mut e);
    activate(&mut e, p0, index::MISTRISE_VILLAGE, 1);
    pass_until(&mut e, stack_is_empty);

    // Nothing is cast this turn; the turn passes and p0's next one begins.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    flood_both_pools(&mut e);
    let bear = in_hand(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: bear }).unwrap();
    assert!(
        !e.state()
            .object(bear)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "the protection lapsed at the end of the turn it was activated in"
    );
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let counter = in_hand(&e, p1, index::COUNTERSPELL).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    e.apply(
        p1,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bear).unwrap().zone,
        Zone::Graveyard,
        "an unprotected spell in a later turn is countered"
    );
}

/// "The next spell **you** cast": a spell the opponent casts while the grant
/// is waiting is neither protected nor does it use the grant up. p1's
/// Lightning Bolt is countered by p0's Counterspell (so it was counterable);
/// p0's Counterspell is the next spell p0 cast, so p1's own Counterspell
/// cannot counter it, and the Bolt stays countered.
#[test]
fn mistrise_village_does_not_protect_or_spend_on_an_opponents_spell() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MISTRISE_VILLAGE])
        .hand(0, &[index::COUNTERSPELL])
        .hand(1, &[index::LIGHTNING_BOLT, index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    flood_both_pools(&mut e);
    let life = e.state().players[0].life;
    activate(&mut e, p0, index::MISTRISE_VILLAGE, 1);
    pass_until(&mut e, stack_is_empty);

    // p0 passes with the grant waiting; p1 casts a Bolt at p0.
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt = in_hand(&e, p1, index::LIGHTNING_BOLT).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: bolt }).unwrap();
    e.apply(
        p1,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p0],
        },
    )
    .unwrap();
    assert!(
        !e.state()
            .object(bolt)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "an opponent's spell is not the next spell *you* cast"
    );

    // p1 keeps priority after casting and passes; p0's Counterspell answers
    // the Bolt, and is p0's next spell: protected.
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    let mine = in_hand(&e, p0, index::COUNTERSPELL).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: mine }).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bolt],
            players: vec![],
        },
    )
    .unwrap();
    assert!(
        e.state()
            .object(mine)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "p1's spell did not spend the grant: p0's next spell is still protected"
    );

    // p0 passes; p1 tries to counter that Counterspell and cannot.
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let theirs = in_hand(&e, p1, index::COUNTERSPELL).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: theirs })
        .unwrap();
    e.apply(
        p1,
        PlayerAction::ChooseTargets {
            objects: vec![mine],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        life,
        "p0's protected Counterspell resolved and countered the Bolt"
    );
    assert_eq!(e.state().object(bolt).unwrap().zone, Zone::Graveyard);
}
