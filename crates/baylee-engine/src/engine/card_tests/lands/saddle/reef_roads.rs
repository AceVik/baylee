//! `cards/lands/saddle/reef_roads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The five single-permanent entry clauses that looked across the table,
/// each played beside the permanent it asks about — once on the opponent's
/// side, where it must not count, and once on its own, where it must.
///
/// `every_entry_clause_is_scoped_to_its_controller` is the rule; this is
/// the board that shows what the rule was about. Every one of the five
/// prints "unless you control …", and every one of them entered untapped
/// because the opponent had a Vehicle, a legend, a Swamp or an Island —
/// and passed its own test, which had one player's board on it. The Roads'
/// tests never had the other half at all: nothing put a Vehicle anywhere.
#[test]
fn a_checkland_asks_only_about_its_controller_s_permanents() {
    let p0 = PlayerId::new(0);
    // (the land, a permanent its clause names)
    let cases: [(CardIndex, CardIndex); 5] = [
        (reef_roads(), smugglers_copter()),
        (rocky_roads(), smugglers_copter()),
        (chocobo_camp(), katara_the_fearless()),
        (spymaster_s_vault(), swamp()),
        (cori_mountain_monastery(), island()),
    ];
    for (land, named) in cases {
        let name = baylee_cards::by_index(land).map_or("?", baylee_cards_dsl::CardDef::name);
        for (seat, untapped) in [(1, false), (0, true)] {
            let mut engine = Duel::new(219, forest())
                .battlefield(seat, &[named])
                .hand(0, &[land])
                .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);
            let card = in_hand(&engine, p0, land).expect("the land is in hand");
            engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                !entered_tapped(&engine, card),
                untapped,
                "{name} with what its clause names on seat {seat}"
            );
        }
    }
}

/// Reef Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {U}." / "{1}{U}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn reef_roads_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(324, forest()).hand(0, &[reef_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, reef_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, reef_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
