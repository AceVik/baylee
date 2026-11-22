use crate::deck::{Pool, Registry};

struct Deck {
    versions: [Pool; 4],
    last_original: i32,
    returned: bool,
}
// Synthetic families have disjoint card identities, one per pilot. Exact
// lists live in the common format registry and score from pre-match values.
pub(super) struct Field {
    books: [Registry; 2],
    decks: Vec<Deck>,
    pub matches: u64,
    pub returns: u64,
    pub max_error: i32,
}
impl Field {
    pub(super) fn new(formats: &[usize]) -> Self {
        let mut books = std::array::from_fn(|_| Registry::new(2.0));
        let decks = formats
            .iter()
            .enumerate()
            .map(|(id, &format)| {
                let base = u32::try_from(id).expect("fixture size") * 256;
                let versions = std::array::from_fn(|season| {
                    let mut cards: Vec<_> = (0..75)
                        .map(|i| {
                            base + if (season == 1 && i < 15) || (season == 2 && i < 45) {
                                100 + i
                            } else {
                                i
                            }
                        })
                        .collect();
                    if season == 3 {
                        cards.swap(0, 60);
                    }
                    Pool::registered(&cards[..60], &cards[60..])
                });
                books[format].restore_reference(&versions[0], 1500);
                Deck {
                    versions,
                    last_original: 1500,
                    returned: false,
                }
            })
            .collect();
        Self {
            books,
            decks,
            matches: 0,
            returns: 0,
            max_error: 0,
        }
    }
    pub(super) fn close_season(&mut self) {
        for book in &mut self.books {
            book.close_season();
        }
    }
    pub(super) fn play(
        &mut self,
        season: usize,
        format: usize,
        ids: [usize; 2],
        pilot_gap: i32,
        won: bool,
    ) {
        let book = &mut self.books[format];
        let before = ids.map(|id| book.register(&self.decks[id].versions[season]));
        for (i, id) in ids.into_iter().enumerate() {
            let deck = &mut self.decks[id];
            if season == 3 && !deck.returned {
                self.max_error = self.max_error.max((before[i] - deck.last_original).abs());
                deck.returned = true;
                self.returns += 1;
            }
            let updated = book.record_result(
                &deck.versions[season],
                before[1 - i],
                if i == 0 { pilot_gap } else { -pilot_gap },
                if i == 0 { won } else { !won },
            );
            if season == 0 {
                deck.last_original = updated;
            }
        }
        self.matches += 1;
    }
}
