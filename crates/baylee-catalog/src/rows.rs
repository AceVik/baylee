//! Ingest's bulk statements: rows of cards, faces and legalities, as many
//! to a statement as the bind limit allows.

use super::*;

/// The columns one printing binds in `upsert_cards`, in bind order.
///
/// The placeholder run, the value pushes and the table definition are written
/// three places apart; a mismatch makes Postgres reject the whole batch, so
/// the list lives here and the tests hold the other two against it.
pub(crate) const CARD_INSERT_COLUMNS: &str = "scryfall_id, oracle_id, lang, set_code, collector_number, \
     rarity, layout, released_at, set_name, artist, finishes, frame_effects, border_color, promo, set_type, \
     digital, games";

/// How many columns that is.
pub(crate) const CARD_COLUMNS: usize = column_count(CARD_INSERT_COLUMNS);

/// The columns one face binds in `upsert_faces`, in bind order.
pub(crate) const FACE_INSERT_COLUMNS: &str = "scryfall_id, face_index, name, printed_name, flavor_name, \
     type_line, printed_type_line, oracle_text, printed_text, mana_cost, power, toughness, loyalty";

/// How many columns that is.
pub(crate) const FACE_COLUMNS: usize = column_count(FACE_INSERT_COLUMNS);

/// The columns one card binds in `upsert_legalities`, in bind order.
pub(crate) const LEGALITY_INSERT_COLUMNS: &str = "oracle_id, legalities";

/// How many columns that is.
pub(crate) const LEGALITY_COLUMNS: usize = column_count(LEGALITY_INSERT_COLUMNS);

/// The most parameters one statement may bind: the extended protocol's Bind
/// message counts them in 16 bits.
///
/// Every upsert splits its rows at this cap itself rather than trusting a
/// caller's batch size. The guard this replaced multiplied a batch by a
/// per-face column count written as a literal, and the literal stayed at
/// twelve when `flavor_name` made the face insert bind thirteen (#174); it
/// also had to assume at most three faces a card, where Scryfall prints
/// five on one.
pub(crate) const MAX_BIND_PARAMS: usize = 65_535;

/// How many rows of a table with `columns` columns fit in one statement.
pub(crate) const fn rows_per_statement(columns: usize) -> usize {
    MAX_BIND_PARAMS / columns
}

/// How many columns a comma-separated column list names.
///
/// Counted from the list itself, so a column added to the list moves every
/// count derived from it.
pub(crate) const fn column_count(list: &str) -> usize {
    let bytes = list.as_bytes();
    let mut count = 1;
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b',' {
            count += 1;
        }
        at += 1;
    }
    count
}

/// Writes one row's placeholders, `($1::uuid,$2,…)`, for a column list.
///
/// The casts Postgres cannot infer from the column are named here rather
/// than counted to, so adding a column ahead of one of them cannot move it.
pub(crate) fn push_row(sql: &mut String, columns: &str, base: usize) {
    sql.push('(');
    for (k, column) in columns.split(',').map(str::trim).enumerate() {
        if k > 0 {
            sql.push(',');
        }
        let at = base + k + 1;
        let _ = match column {
            "scryfall_id" | "oracle_id" => write!(sql, "${at}::uuid"),
            // An omitted `released_at` binds as null and the cast is what
            // tells Postgres which kind of null; an *empty* one would be
            // `''::date`, which is an error and not a null.
            "released_at" => write!(sql, "nullif(${at}, '')::date"),
            "legalities" => write!(sql, "${at}::jsonb"),
            _ => write!(sql, "${at}"),
        };
    }
    sql.push(')');
}

/// The `INSERT` for one chunk of printings.
pub(crate) fn cards_statement(cards: &[&scryfall::Card]) -> (String, Vec<Value>) {
    let mut sql = format!("INSERT INTO cards ({CARD_INSERT_COLUMNS}) VALUES ");
    let mut values: Vec<Value> = Vec::with_capacity(cards.len() * CARD_COLUMNS);
    for (i, card) in cards.iter().enumerate() {
        if i > 0 {
            sql.push(',');
        }
        push_row(&mut sql, CARD_INSERT_COLUMNS, i * CARD_COLUMNS);
        values.push(Value::from(card.id.clone()));
        values.push(Value::from(card.oracle_identity().map(str::to_string)));
        values.push(Value::from(card.lang.clone()));
        values.push(Value::from(card.set.clone()));
        values.push(Value::from(card.collector_number.clone()));
        values.push(Value::from(card.rarity.clone()));
        values.push(Value::from(card.layout.clone()));
        values.push(Value::from(card.released_at.clone()));
        values.push(Value::from(card.set_name.clone()));
        values.push(Value::from(card.artist.clone()));
        values.push(Value::from(card.finish_list().join(",")));
        values.push(Value::from(card.frame_effects.join(",")));
        values.push(Value::from(card.border_color.clone()));
        values.push(Value::from(card.promo));
        values.push(Value::from(card.set_type.clone()));
        values.push(Value::from(card.digital));
        values.push(Value::from(card.games.join(",")));
    }
    sql.push_str(
        " ON CONFLICT (scryfall_id) DO UPDATE SET \
         oracle_id = EXCLUDED.oracle_id, lang = EXCLUDED.lang, \
         set_code = EXCLUDED.set_code, collector_number = EXCLUDED.collector_number, \
         rarity = EXCLUDED.rarity, layout = EXCLUDED.layout, \
         released_at = EXCLUDED.released_at, set_name = EXCLUDED.set_name, \
         artist = EXCLUDED.artist, finishes = EXCLUDED.finishes, \
         frame_effects = EXCLUDED.frame_effects, border_color = EXCLUDED.border_color, \
         promo = EXCLUDED.promo, set_type = EXCLUDED.set_type, \
         digital = EXCLUDED.digital, games = EXCLUDED.games, \
         updated_at = now()",
    );
    (sql, values)
}

/// One `card_faces` row: the printing it belongs to, its index, the face.
pub(crate) type FaceRow<'c> = (&'c str, i16, scryfall::Face);

/// Every face row a batch of printings writes, in printing then face order.
pub(crate) fn face_rows<'c>(cards: &[&'c scryfall::Card]) -> Vec<FaceRow<'c>> {
    cards
        .iter()
        .flat_map(|card| {
            card.faces()
                .into_iter()
                .enumerate()
                .map(move |(index, face)| (card.id.as_str(), index as i16, face))
        })
        .collect()
}

/// The `INSERT` for one chunk of face rows.
pub(crate) fn faces_statement(rows: &[FaceRow<'_>]) -> (String, Vec<Value>) {
    let mut sql = format!("INSERT INTO card_faces ({FACE_INSERT_COLUMNS}) VALUES ");
    let mut values: Vec<Value> = Vec::with_capacity(rows.len() * FACE_COLUMNS);
    for (n, (id, index, face)) in rows.iter().enumerate() {
        if n > 0 {
            sql.push(',');
        }
        push_row(&mut sql, FACE_INSERT_COLUMNS, n * FACE_COLUMNS);
        values.push(Value::from((*id).to_string()));
        values.push(Value::from(*index));
        values.push(Value::from(face.name.clone()));
        values.push(Value::from(face.printed_name.clone()));
        values.push(Value::from(face.flavor_name.clone()));
        values.push(Value::from(face.type_line.clone()));
        values.push(Value::from(face.printed_type_line.clone()));
        values.push(Value::from(face.oracle_text.clone()));
        values.push(Value::from(face.printed_text.clone()));
        values.push(Value::from(face.mana_cost.clone()));
        values.push(Value::from(face.power.clone()));
        values.push(Value::from(face.toughness.clone()));
        values.push(Value::from(face.loyalty.clone()));
    }
    sql.push_str(
        " ON CONFLICT (scryfall_id, face_index) DO UPDATE SET \
         name = EXCLUDED.name, printed_name = EXCLUDED.printed_name, \
         flavor_name = EXCLUDED.flavor_name, \
         type_line = EXCLUDED.type_line, printed_type_line = EXCLUDED.printed_type_line, \
         oracle_text = EXCLUDED.oracle_text, printed_text = EXCLUDED.printed_text, \
         mana_cost = EXCLUDED.mana_cost, power = EXCLUDED.power, \
         toughness = EXCLUDED.toughness, loyalty = EXCLUDED.loyalty",
    );
    (sql, values)
}

/// The `INSERT` for one chunk of `(oracle_id, card)` legality rows.
pub(crate) fn legalities_statement(rows: &[(&str, &scryfall::Card)]) -> (String, Vec<Value>) {
    let mut sql = format!("INSERT INTO card_legalities ({LEGALITY_INSERT_COLUMNS}) VALUES ");
    let mut values: Vec<Value> = Vec::with_capacity(rows.len() * LEGALITY_COLUMNS);
    for (n, (oracle_id, card)) in rows.iter().enumerate() {
        if n > 0 {
            sql.push(',');
        }
        push_row(&mut sql, LEGALITY_INSERT_COLUMNS, n * LEGALITY_COLUMNS);
        values.push(Value::from((*oracle_id).to_string()));
        values.push(Value::from(
            serde_json::to_string(&card.legalities).unwrap_or_else(|_| "{}".to_string()),
        ));
    }
    sql.push_str(" ON CONFLICT (oracle_id) DO UPDATE SET legalities = EXCLUDED.legalities");
    (sql, values)
}
