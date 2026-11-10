//! `cards/creatures/mv_7/molimo_maro_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn molimo_maro_sorcerer() -> CardIndex {
    card_index("b87a2d20-4e9f-4256-9d24-80c38515119e")
}

/// Molimo, Maro-Sorcerer — a trampler for {4}{G}{G}{G} whose power and
/// toughness are each the number of lands you control: seven Forests, 7/7.
#[test]
fn molimo_is_as_big_as_my_lands() {
    let mut engine = table7(&lots(forest(), 7), &[molimo_maro_sorcerer()], &[]);
    let molimo = cast_and_resolve(&mut engine, molimo_maro_sorcerer());
    assert_body(&engine, molimo, (7, 7), KeywordSet::TRAMPLE);
}

/// It follows the count: an eighth land makes it 8/8, and the opponent's lands
/// never count.
#[test]
fn molimo_grows_with_a_land_played_and_ignores_theirs() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[molimo_maro_sorcerer(), forest(), forest()],
        &[forest()],
        &lots(mountain(), 3),
    );
    let molimo = on_battlefield(&engine, p0, molimo_maro_sorcerer()).expect("out");
    assert_eq!(pt(&engine, molimo), (2, 2));
    let _ = play_land(&mut engine, p0, forest());
    assert_eq!(pt(&engine, molimo), (3, 3));
}
