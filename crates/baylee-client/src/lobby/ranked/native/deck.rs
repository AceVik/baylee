//! Resolve the selected private Central deck exactly once before queue admission.
use super::{Account, Error, Format, LobbyState};
use baylee_account::RankedDeck;
use baylee_account::decks::DeckSnapshot;
use uuid::Uuid;

pub(super) struct Frozen {
    pub(super) look: baylee_account::advent::Equipment,
    pub(super) deck: RankedDeck,
    pub(super) digest: [u8; 32],
}

#[derive(Clone, Copy)]
pub(super) struct Source {
    id: Uuid,
    revision: u64,
}

pub(super) struct Snapshot {
    pub(super) look: baylee_account::advent::Equipment,
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
            look: snapshot.look,
            deck: snapshot.deck,
            digest: snapshot.registration.digest,
        })
    }

    pub(super) const fn id(self) -> Uuid {
        self.id
    }

    pub(super) fn read(self, account: &mut Account, format: Format) -> Result<Snapshot, Error> {
        let snapshot = account.deck_version(self.id, self.revision)?;
        self.project(snapshot, format)
    }

    fn project(self, snapshot: DeckSnapshot, format: Format) -> Result<Snapshot, Error> {
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
            look: draft.look,
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_account::{advent::Equipment, decks::DeckDraft};

    #[test]
    fn registered_rows_and_ranked_elo_stay_identical_when_only_private_equipment_changes() {
        let id = Uuid::now_v7();
        let source = Source::at_revision(id, 3);
        let project = |look| {
            let draft = DeckDraft {
                format: "standard".into(),
                cards: vec!["1 Cryptic Spires circles=WU # private plan".into()],
                look,
                ..DeckDraft::default()
            };
            source
                .project(
                    DeckSnapshot {
                        id,
                        revision: 3,
                        draft,
                    },
                    Format::Standard,
                )
                .unwrap()
        };
        let design = Equipment {
            sleeve: Some(*Uuid::now_v7().as_bytes()),
            playmat: None,
        };
        let a = project(Equipment::default());
        let b = project(design);
        assert_eq!(b.look, design);
        assert_ne!(a.look, b.look);
        assert_eq!(a.deck.cards, b.deck.cards);
        let row = baylee_core::deckrow::parse(&a.deck.cards[0]).unwrap();
        assert_eq!(row.name, "Cryptic Spires");
        assert!(row.note.is_none());
        assert!(row.print.scryfall_id.is_some());
        assert_eq!(a.registration.digest, b.registration.digest);
        assert_eq!(
            baylee_cards::ranked::rating_pool(&a.registration),
            baylee_cards::ranked::rating_pool(&b.registration)
        );
    }
}
