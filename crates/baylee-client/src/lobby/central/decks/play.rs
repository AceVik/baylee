//! The gateway receives only the registered cards it must execute. Private
//! deck notes and library metadata have no representation in this projection.
use baylee_account::Error;

fn card(name: &str) -> Result<&str, Error> {
    baylee_cards::decks::by_name(name)
        .map(|_| name)
        .ok_or(Error::Rejected)
}

pub(crate) fn play_rows(rows: &[String]) -> Result<Vec<String>, Error> {
    rows.iter()
        .map(|line| {
            let mut row = baylee_core::deckrow::parse(line).map_err(|_| Error::Rejected)?;
            card(&row.name)?;
            row.note = None;
            Ok(row.to_string())
        })
        .collect()
}

pub(crate) fn play_commanders(names: &[String]) -> Result<Vec<String>, Error> {
    names
        .iter()
        .map(|name| card(name).map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_notes_stay_home_and_the_engine_receives_the_committed_print_and_choices() {
        let private = vec!["1 Cryptic Spires circles=WU # sideboard plan".into()];
        let rows = play_rows(&private).unwrap();
        assert_eq!(rows, ["1 Cryptic Spires circles=WU"]);
        let before = baylee_cards::ranked::registration("standard", &private, &[], &[]).unwrap();
        let played = baylee_cards::ranked::registration("standard", &rows, &[], &[]).unwrap();
        assert_eq!(before.digest, played.digest);
        assert_eq!(before.pool, played.pool);
    }

    #[test]
    fn an_unknown_row_is_never_forwarded_as_a_card_or_commander_name() {
        assert!(play_rows(&["1 arbitrary private group label".into()]).is_err());
        assert!(play_commanders(&["arbitrary private group label".into()]).is_err());
    }
}
