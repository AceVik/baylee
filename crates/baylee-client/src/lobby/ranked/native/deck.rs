//! Resolve the selected private Central deck exactly once before queue admission.
use super::{Account, Error, Format, LobbyState};
use baylee_account::RankedDeck;
use uuid::Uuid;

pub(super) struct Frozen {
    pub(super) deck: RankedDeck,
    pub(super) digest: [u8; 32],
}

#[derive(Clone, Copy)]
pub(super) struct Source(Uuid);

pub(super) const fn name(format: Format) -> &'static str {
    match format {
        Format::Standard => "standard",
        Format::Highlander => "highlander",
        Format::Commander => "commander",
    }
}

impl Source {
    pub(super) fn selected(state: &LobbyState, id: &str, format: Format) -> Result<Self, Error> {
        if !state
            .lobby
            .decks()
            .iter()
            .any(|d| d.id == id && d.format == name(format))
        {
            return Err(Error::Conflict);
        }
        Ok(Self(Uuid::parse_str(id).map_err(|_| Error::Rejected)?))
    }

    pub(super) fn freeze(self, account: &mut Account, format: Format) -> Result<Frozen, Error> {
        let snapshot = account.load_deck(self.0)?;
        if snapshot.id != self.0 || snapshot.draft.format != name(format) {
            return Err(Error::Conflict);
        }
        let mut draft = snapshot.draft;
        draft.cards = crate::lobby::central::decks::play_rows(&draft.cards)?;
        draft.sideboard = crate::lobby::central::decks::play_rows(&draft.sideboard)?;
        draft.commanders = crate::lobby::central::decks::play_commanders(&draft.commanders)?;
        let registration = baylee_cards::ranked::registration(
            name(format),
            &draft.cards,
            &draft.sideboard,
            &draft.commanders,
        )
        .map_err(|_| Error::Rejected)?;
        Ok(Frozen {
            deck: RankedDeck {
                format,
                cards: draft.cards,
                sideboard: draft.sideboard,
                commanders: draft.commanders,
            },
            digest: registration.digest,
        })
    }
}
