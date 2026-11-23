//! Resolve the selected private Central deck exactly once before queue admission.
use super::{Account, Error, Format, LobbyState};
use baylee_account::RankedDeck;
use uuid::Uuid;

pub(super) struct Frozen {
    pub(super) deck: RankedDeck,
    pub(super) digest: [u8; 32],
}

#[derive(Clone, Copy)]
pub(super) struct Source {
    id: Uuid,
    revision: u64,
}

pub(super) struct Snapshot {
    pub(super) revision: u64,
    pub(super) deck: RankedDeck,
    pub(super) registration: baylee_cards::ranked::Registration,
}

impl Source {
    pub(super) const fn at_revision(id: Uuid, revision: u64) -> Self {
        Self { id, revision }
    }

    pub(super) fn selected(state: &LobbyState, id: &str, format: Format) -> Result<Self, Error> {
        if !state
            .lobby
            .decks()
            .iter()
            .any(|d| d.id == id && d.format == format.as_str())
        {
            return Err(Error::Conflict);
        }
        let id = Uuid::parse_str(id).map_err(|_| Error::Rejected)?;
        let revision = state
            .central_handle()
            .filter(super::central::Handle::is_active)
            .ok_or(Error::Rejected)?
            .listed_deck_revision(id)?;
        Ok(Self::at_revision(id, revision))
    }

    pub(super) fn freeze(self, account: &mut Account, format: Format) -> Result<Frozen, Error> {
        let snapshot = self.read(account, format)?;
        Ok(Frozen {
            deck: snapshot.deck,
            digest: snapshot.registration.digest,
        })
    }

    pub(super) const fn id(self) -> Uuid {
        self.id
    }

    pub(super) fn read(self, account: &mut Account, format: Format) -> Result<Snapshot, Error> {
        let snapshot = account.deck_version(self.id, self.revision)?;
        if snapshot.id != self.id
            || snapshot.revision != self.revision
            || snapshot.draft.format != format.as_str()
        {
            return Err(Error::Conflict);
        }
        let mut draft = snapshot.draft;
        draft.cards = crate::lobby::central::decks::play_rows(&draft.cards)?;
        draft.sideboard = crate::lobby::central::decks::play_rows(&draft.sideboard)?;
        draft.commanders = crate::lobby::central::decks::play_commanders(&draft.commanders)?;
        let registration = baylee_cards::ranked::registration(
            format.as_str(),
            &draft.cards,
            &draft.sideboard,
            &draft.commanders,
        )
        .map_err(|_| Error::Rejected)?;
        let [cards, sideboard, commanders] =
            baylee_cards::ranked::playable_rows(&registration.deck).map_err(|_| Error::Rejected)?;
        Ok(Snapshot {
            revision: snapshot.revision,
            deck: RankedDeck {
                format,
                cards,
                sideboard,
                commanders,
            },
            registration,
        })
    }
}
