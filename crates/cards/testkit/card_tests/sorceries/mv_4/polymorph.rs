//! `cards/sorceries/mv_4/polymorph.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn polymorph() -> CardIndex {
    card_index("039115f2-6322-4674-bc64-f21883ed375a")
}

/// Polymorph (039115f2-6322-4674-bc64-f21883ed375a) — {3}{U} Sorcery:
/// "Destroy target creature. It can't be regenerated. Its controller reveals
/// cards from the top of their library until they reveal a creature card. The
/// player puts that card onto the battlefield, then shuffles all other cards
/// revealed this way into their library."
#[test]
fn polymorph_destroys_through_regeneration_and_cheats_in_revealed_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            island(),
            island(),
            island(),
            island(),
            lotleth_troll(),
        ],
        &[],
        &[polymorph(), fangren_hunter(), sol_ring(), counterspell()],
        &[],
    );
    let troll = lk::obj(&engine, P0, lotleth_troll());
    let my_swamp = lk::obj(&engine, P0, swamp());
    let hunter = hand_to_library_top(&mut engine, P0, fangren_hunter());
    let ring = hand_to_library_top(&mut engine, P0, sol_ring());
    let spell = hand_to_library_top(&mut engine, P0, counterspell());

    tap_mana_where(&mut engine, P0, |id| id == my_swamp);
    raise_a_shield(&mut engine, P0, troll, 1);

    tap_all_mana(&mut engine, P0);
    let before_entries = engine.journal().entries().len();
    lk::cast_at(&mut engine, P0, polymorph(), &[troll], &[]);

    let entries = &engine.journal().entries()[before_entries..];
    assert_eq!(
        lk::zone_of(&engine, troll),
        Zone::Graveyard,
        "destroyed through its regeneration shield"
    );
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Battlefield);
    assert_eq!(engine.state().object(hunter).unwrap().controller, P0);
    assert_eq!(lk::zone_of(&engine, ring), Zone::Library);
    assert_eq!(lk::zone_of(&engine, spell), Zone::Library);
    for revealed in [spell, ring, hunter] {
        assert!(
            entries
                .iter()
                .any(|e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&revealed))),
            "expected reveal for {revealed:?}"
        );
    }
}

/// Polymorph — "Its controller reveals...": targeting an opponent's creature
/// reveals from the opponent's library and puts that creature under the
/// opponent's control.
#[test]
fn polymorph_on_opponents_creature_reveals_from_opponents_library_under_their_control() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), island(), island()],
        &[llanowar_elves()],
        &[polymorph()],
        &[aurochs(), sol_ring(), island()],
    );
    let their_elf = lk::obj(&engine, P1, llanowar_elves());
    let their_beast = hand_to_library_top(&mut engine, P1, aurochs());
    let their_ring = hand_to_library_top(&mut engine, P1, sol_ring());
    let their_island = hand_to_library_top(&mut engine, P1, island());

    tap_all_mana(&mut engine, P0);
    lk::cast_at(&mut engine, P0, polymorph(), &[their_elf], &[]);

    assert_eq!(lk::zone_of(&engine, their_elf), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_beast), Zone::Battlefield);
    assert_eq!(
        engine.state().object(their_beast).unwrap().controller,
        P1,
        "enters under the destroyed creature's controller's control"
    );
    assert_eq!(lk::zone_of(&engine, their_ring), Zone::Library);
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Library);
}
