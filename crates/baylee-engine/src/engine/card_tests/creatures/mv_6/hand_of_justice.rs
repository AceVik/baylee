//! `cards/creatures/mv_6/hand_of_justice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).expect("the source is on the battlefield");
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .expect("the ability is activated");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::ChooseObjects { objects })
        .expect("the choice is on the menu");
}

fn hand_of_justice() -> CardIndex {
    card_index("af8eb20a-dfa2-49a4-892b-26d7e276fcd2")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn mesa_pegasus() -> CardIndex {
    card_index("8161f5b8-6aab-4133-ba2c-2e7b5774153e")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Hand of Justice (af8eb20a-dfa2-49a4-892b-26d7e276fcd2) — "{T}, Tap three
/// untapped white creatures you control: Destroy target creature."
#[test]
fn hand_of_justice_taps_itself_and_three_white_creatures_to_destroy_a_creature() {
    let mut e = prepared(
        &[],
        &[
            hand_of_justice(),
            savannah_lions(),
            mesa_pegasus(),
            serra_angel(),
            grizzly_bears(),
        ],
        &[hill_giant()],
    );
    let hand = on_battlefield(&e, P0, hand_of_justice()).expect("Hand of Justice is out");
    let whites = [
        on_battlefield(&e, P0, savannah_lions()).expect("Savannah Lions is out"),
        on_battlefield(&e, P0, mesa_pegasus()).expect("Mesa Pegasus is out"),
        on_battlefield(&e, P0, serra_angel()).expect("Serra Angel is out"),
    ];
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");
    assert!(priority_offer(&e).abilities.contains(&(hand, 0)));

    activate_by_card(&mut e, P0, hand_of_justice(), 0);
    targets(&mut e, vec![giant]);
    choose_objects(&mut e, whites.to_vec());
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P1, hill_giant()).is_none(), "destroyed");
    assert!(in_graveyard(&e, P1, hill_giant()).is_some());
    assert!(is_tapped(&e, hand), "{{T}} of Hand of Justice itself");
    for white in whites {
        assert!(is_tapped(&e, white), "a white creature paid the tap cost");
    }
    assert!(!is_tapped(&e, bears), "the green creature did not");
}

/// Hand of Justice — the three must be white, untapped, and not the Hand.
#[test]
fn hand_of_justice_needs_three_other_untapped_white_creatures() {
    // Two white creatures and a green one: not enough.
    let e = prepared(
        &[],
        &[
            hand_of_justice(),
            savannah_lions(),
            mesa_pegasus(),
            grizzly_bears(),
        ],
        &[hill_giant()],
    );
    let hand = on_battlefield(&e, P0, hand_of_justice()).expect("Hand of Justice is out");
    assert!(
        !priority_offer(&e).abilities.contains(&(hand, 0)),
        "only two other white creatures"
    );

    // Three white creatures, one of them tapped: not enough.
    let mut e = prepared(
        &[],
        &[
            hand_of_justice(),
            savannah_lions(),
            mesa_pegasus(),
            serra_angel(),
        ],
        &[hill_giant()],
    );
    let hand = on_battlefield(&e, P0, hand_of_justice()).expect("Hand of Justice is out");
    assert!(
        priority_offer(&e).abilities.contains(&(hand, 0)),
        "three untapped white creatures"
    );
    let lions = on_battlefield(&e, P0, savannah_lions()).expect("Savannah Lions is out");
    e.state.set_tapped(lions, true);
    e.refresh_offer();
    assert!(
        !priority_offer(&e).abilities.contains(&(hand, 0)),
        "a tapped white creature does not count"
    );

    // Three untapped white creatures but the Hand itself tapped: not offered.
    let mut e = prepared(
        &[],
        &[
            hand_of_justice(),
            savannah_lions(),
            mesa_pegasus(),
            serra_angel(),
        ],
        &[hill_giant()],
    );
    let hand = on_battlefield(&e, P0, hand_of_justice()).expect("Hand of Justice is out");
    e.state.set_tapped(hand, true);
    e.refresh_offer();
    assert!(
        !priority_offer(&e).abilities.contains(&(hand, 0)),
        "the Hand must be able to tap"
    );
}
