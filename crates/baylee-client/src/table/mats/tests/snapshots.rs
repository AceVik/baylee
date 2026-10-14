use super::*;

#[test]
fn same_sequence_replacements_refresh_projected_colors_and_ability_index() {
    let mut view = tower(ManaColor::Red);
    let mut duel = received(view.clone());
    assert_tally(&duel, [0.0, 0.0, 0.0, 1.0, 0.0]);
    // Alternate repeatedly: neither the sequence, object id nor card identity
    // changes, and returning to an earlier value must work too.
    for color in [ManaColor::Blue, ManaColor::Red, ManaColor::Blue] {
        view.battlefield[0].board_mana.as_mut().unwrap().colors = vec![color];
        duel.receive_view(view.clone());
        let mut expected = [0.0; 5];
        expected[color as usize] = 1.0;
        assert_tally(&duel, expected);
        assert_eq!(duel.view.as_ref().unwrap().seq, 1);
    }
    view.battlefield[0].board_mana.as_mut().unwrap().index = 1;
    duel.receive_view(view.clone());
    assert_tally(&duel, [0.0; 5]);
    view.battlefield[0].board_mana = None;
    duel.receive_view(view);
    assert_tally(&duel, [0.0; 5]);
}

#[test]
fn granted_mana_and_projected_basic_types_are_read_on_the_snapshot_edge() {
    let mut land = token(1, 0, "Projected land", 0, 0);
    land.types = TypeSet::LAND;
    land.subtypes = SubtypeSet::from_slice(&[baylee_core::generated::subtypes::land::FOREST]);
    let mut view = ViewBuilder::new(2).with_battlefield(0, [land]).build();
    let mut duel = received(view.clone());
    assert_tally(&duel, [0.0, 0.0, 0.0, 0.0, 1.0]);
    view.battlefield[0].granted_mana = Some(baylee_view::GrantedMana {
        slot: 3,
        colors: vec![ManaColor::Blue],
        amount: 1,
    });
    duel.receive_view(view.clone());
    assert_tally(&duel, [0.0, 0.5, 0.0, 0.0, 0.5]);
    view.battlefield[0].granted_mana = None;
    view.battlefield[0].subtypes =
        SubtypeSet::from_slice(&[baylee_core::generated::subtypes::land::MOUNTAIN]);
    duel.receive_view(view.clone());
    assert_tally(&duel, [0.0, 0.0, 0.0, 1.0, 0.0]);
    view.battlefield[0].status = baylee_view::ObjectStatus::FACE_DOWN;
    duel.receive_view(view);
    assert_tally(&duel, [0.0; 5]);
}

#[test]
fn dynamic_mana_notices_other_permanents_and_their_controller() {
    let mut druid = card(1, 0, "Harabaz Druid");
    // A projected type change removed its own Ally subtype. Only another
    // Ally can make the amount positive; the source itself stays unchanged.
    druid.subtypes = SubtypeSet::EMPTY;
    let mut view = ViewBuilder::new(2).with_battlefield(0, [druid]).build();
    let mut duel = received(view.clone());
    assert_tally(&duel, [0.0; 5]);
    let mut ally = token(2, 0, "Ally", 1, 1);
    ally.subtypes = SubtypeSet::from_slice(&[baylee_core::generated::subtypes::creature::ALLY]);
    view.battlefield.push(ally);
    duel.receive_view(view.clone());
    assert_tally(&duel, [0.2; 5]);
    view.battlefield[1].controller = PlayerId::new(1);
    duel.receive_view(view);
    assert_tally(&duel, [0.0; 5]);
}

#[test]
fn same_sequence_perspective_replacement_and_empty_board_forget_old_mana() {
    let mut duel = received(tower(ManaColor::Green));
    assert_tally(&duel, [0.0, 0.0, 0.0, 0.0, 1.0]);
    let mut other = tower(ManaColor::White);
    other.seat = PlayerId::new(1);
    duel.receive_view(other.clone());
    assert_tally(&duel, [1.0, 0.0, 0.0, 0.0, 0.0]);
    other.battlefield.clear();
    duel.receive_view(other);
    assert_tally(&duel, [0.0; 5]);
    duel.receive_view(tower(ManaColor::Red));
    assert_tally(&duel, [0.0, 0.0, 0.0, 1.0, 0.0]);
}

#[test]
fn rewind_and_new_game_do_not_keep_the_previous_tables_totals() {
    let mut duel = received(tower(ManaColor::Green));
    assert_tally(&duel, [0.0, 0.0, 0.0, 0.0, 1.0]);
    duel.rewind();
    assert!(duel.view.is_none());
    assert_tally(&duel, [0.0; 5]);
    duel.receive_view(tower(ManaColor::Blue));
    assert_tally(&duel, [0.0, 1.0, 0.0, 0.0, 0.0]);
    duel = Duel::default();
    assert_tally(&duel, [0.0; 5]);
    duel.receive_view(tower(ManaColor::Red));
    assert_tally(&duel, [0.0, 0.0, 0.0, 1.0, 0.0]);
}
