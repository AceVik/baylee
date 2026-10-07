//! `cards/enchantments/auras/mv_3/strength_of_the_harvest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Enchant creature. Enchanted creature gets +1/+1 for each creature
/// and/or enchantment you control."
///
/// Three printed claims, and the board is built so that each one moves a
/// number the others cannot. The Aura spell *targets* as it is cast — an
/// Aura spell requires a target, defined by its enchant ability
/// (CR 303.4a) — and the offer is asserted as a whole rather than searched:
/// four Forests stand beside the one creature, so "enchant **creature**"
/// is the difference between one option and five.
///
/// The permanent then arrives attached to what it chose, which is read off
/// `attached_to` and confirmed a second way by the Aura still being on the
/// battlefield at all: an Aura attached to nothing is put into its owner's
/// graveyard by a state-based action (CR 704.5m).
///
/// The count is the half that says which two types are read. A Llanowar
/// Elves alone would leave a 2/2 whether the Aura counted enchantments or
/// not; the Aura **is** an enchantment its controller controls, so it counts
/// itself and the Elf comes out a 3/3. Four Forests are on the table and
/// none of them counts, which is what "creature and/or enchantment" excludes
/// — counting every permanent would read 7/7 here.
///
/// And casting a second Elf afterwards is what makes it "for each" rather
/// than a number fixed as the Aura entered: the projection is read again and
/// the enchanted creature goes to 4/4, while the newcomer — which the Aura
/// is not attached to — stays the 1/1 it was printed as.
#[test]
fn the_harvest_aura_swells_only_its_own_creature_and_recounts_the_board_each_time() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[strength_of_the_harvest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // Crosses a turn boundary if the seed puts p0 on the draw, which
    // `reach_main_phase` cannot: the combat on the way asks for attackers.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a Llanowar Elves is a 1/1 before anything enchants it"
    );

    // Four Forests: three pay {2}{G/W} — the hybrid takes green — and the
    // fourth stays floating for the second Elf, which has to be cast in this
    // same main phase because a pool empties when the step ends (CR 500.5).
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, strength_of_the_harvest()).expect("the Aura is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("{2}{G/W} off four Forests");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura spell requires a target defined by its enchant ability \
             (CR 303.4a) — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        [elf],
        "\"enchant creature\" reaches the one creature on the table and none \
         of the four Forests standing beside it"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, strength_of_the_harvest())
        .expect("the Aura resolved onto the battlefield and stayed there");
    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|o| o.attached_to)
            .expect("the Aura is attached to something"),
        elf,
        "an Aura enters the battlefield attached to the object its spell \
         targeted; attached to nothing it would already be in its owner's \
         graveyard (CR 704.5m)"
    );
    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "one creature (the Elf) and one enchantment (the Aura itself) — and \
         not one of the four Forests"
    );

    let second = in_hand(&engine, p0, llanowar_elves()).expect("the second Elf is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .expect("{G} off the Forest left floating");
    pass_until(&mut engine, stack_is_empty);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the second Elf resolved");
    assert_eq!(
        pt(&engine, elf),
        (4, 4),
        "\"for each\" is read on every projection, so a creature arriving \
         after the Aura counts too"
    );
    let newcomer = elves
        .into_iter()
        .find(|id| *id != elf)
        .expect("the one that is not the enchanted Elf");
    assert_eq!(
        pt(&engine, newcomer),
        (1, 1),
        "only the *enchanted* creature gets the bonus — the filter is the \
         Aura's own host, not every creature you control"
    );
}

/// "Haven of the Harvest — Land. This land enters tapped. {T}: Add {G} or
/// {W}."
///
/// A player playing a modal double-faced card as a land chooses one of its
/// faces that's a land before putting it onto the battlefield, and it enters
/// with that face up (CR 712.12). Only the back face of this card is a land,
/// so there is exactly one choice to make and the engine makes it: the card
/// arrives as face 1, a Land with none of the Aura's text on it.
///
/// It has to be a real land drop rather than a seeded battlefield, because
/// "this land enters tapped" is a replacement effect and a permanent placed
/// on the board never enters at all — the tapped assertion would pass on a
/// card that printed nothing of the kind.
///
/// Then the mana ability, which is the rest of the face: the land untaps on
/// its controller's next turn, and what it offers is asked of the offer
/// first — one printed ability and no CR 305.6 shortcut, because the Haven
/// prints no basic land type — and then of the pool, which ends up holding
/// one mana of the colour chosen and none of the other. "{G} **or** {W}" is
/// the claim that a single {T} producing both would pass.
#[test]
fn the_harvest_land_face_enters_tapped_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) = play_land_face(strength_of_the_harvest(), 1)
        .unwrap_or_else(|why| panic!("Haven of the Harvest {why}"));

    let printed = types(&engine, land);
    assert!(
        printed.contains(TypeSet::LAND),
        "the face that was played is the Land: {printed:?}"
    );
    assert!(
        !printed.contains(TypeSet::ENCHANTMENT),
        "and it carries none of the Aura face's types: {printed:?}"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\""
    );

    // Its own untap step is the first moment it can be tapped for mana.
    pass_until(&mut engine, |e| !is_tapped(e, land));
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&land),
        "the Haven prints no basic land type, so nothing offers it the \
         intrinsic tap of CR 305.6"
    );
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == land)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        [0],
        "the land face offers its own printed mana ability and nothing the \
         Aura face wrote"
    );

    activate(&mut engine, p0, strength_of_the_harvest(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a choice of two colours — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        [ManaColor::Green, ManaColor::White],
        "the two the card prints, in the order it prints them, and no third"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white is one of the two offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "one white mana in the pool, which is what the ability produces"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and none of the colour that was not chosen — the card prints \
         \"or\", not both"
    );
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} in the ability's cost spent the land"
    );
}
