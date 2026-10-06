use super::*;

/// A name predicate and a rules-text predicate in one `OR` is the defect
/// this projection exists to undo: no index can satisfy both at once, so
/// Postgres builds a `tsvector` for every row instead — 2742 ms for
/// `稲妻` against the live catalog. It would come back as an
/// *optimisation* ("one pass over one table"), and it would still be
/// correct, which is why nothing else would catch it.
#[test]
fn the_name_and_the_rules_text_are_never_asked_in_one_predicate() {
    let sql = search_sql();
    let named = sql.find("named AS").expect("the name tier");
    let texted = sql.find("texted AS").expect("the text tier");
    let between = &sql[named..texted];
    assert!(
        !between.contains(" OR "),
        "the name tier's WHERE grew an OR:\n{between}"
    );
    assert!(
        sql.contains("UNION ALL"),
        "the two tiers have to be unioned, not joined"
    );
}

/// The names are fenced in separators so one `position()` can ask "at the
/// start of *a* name" across every language a card prints in. If the fill
/// query stops writing the fence, every tier test silently answers 3 —
/// the search still works, and ranks a substring match as highly as an
/// exact one.
#[test]
fn the_fence_the_tiers_read_is_the_fence_the_projection_writes() {
    assert!(
        PROJECT_SQL.contains("'| ' || catalog_norm(") && PROJECT_SQL.contains("|| ' |'"),
        "the projection stopped fencing names_norm"
    );
    let sql = search_sql();
    assert!(
        sql.contains("position('| ' || q.n || ' |' in s.names_norm)"),
        "the exact tier stopped reading the fence"
    );
}

/// A flavor name belongs to a *printing*, and the inner query keeps one
/// printing per language. Command Tower carries six of them, so folding
/// the lookup in there would keep whichever printing `DISTINCT ON` chose
/// and throw the other five away — a search that finds `Cybertron` and
/// not `Croft Manor`, with nothing to show it happened. It is therefore
/// its own join over every printing, and it has to keep writing the
/// fence, or the names it adds rank as substrings instead of as names.
#[test]
fn every_flavor_name_a_card_was_ever_printed_under_is_searchable() {
    let start = PROJECT_SQL
        .find("f.flavor_name")
        .expect("the projection stopped reading flavor names");
    let join = PROJECT_SQL[..start]
        .rfind("LEFT JOIN (")
        .expect("flavor names are not inside a join of their own");
    let branch = &PROJECT_SQL[join..];
    assert!(
        !branch.contains("DISTINCT ON"),
        "the flavor-name join picks one printing per card:\n{branch}"
    );
    assert!(
        branch.contains("string_agg(DISTINCT f.flavor_name"),
        "the flavor-name join stopped collecting every printing's name"
    );
    assert!(
        branch.contains("' ' || catalog_norm(") && branch.contains("|| ' |'"),
        "the flavor names stopped being fenced like every other name"
    );
    assert!(
        PROJECT_SQL.contains("p.names_norm || coalesce(v.extra_norm, '')"),
        "the flavor names are computed and then not appended"
    );
}

/// A kept card has to escape **every** clause, not the last one written.
///
/// The three filters were `AND`ed in a row and the keep-list was wrapped
/// round them afterwards, which is exactly the edit an added fourth clause
/// undoes by accident: append it after the closing bracket and an acorn
/// land is dropped again, silently, by a rule that never mentions it.
/// So this reads the bracket back rather than trusting the shape.
#[test]
fn a_card_the_repo_implements_escapes_the_whole_filter_and_not_part_of_it() {
    let start = CORPUS_SQL
        .find("AND (c.oracle_id::text = ANY(string_to_array($1")
        .map(|at| at + "AND ".len())
        .expect("the corpus stopped reading the keep-list");

    // Walk to the bracket the keep clause opened. Every filter has to be
    // inside it; what follows may only be the ORDER BY that closes the CTE.
    let body = &CORPUS_SQL[start..];
    let mut depth = 0i32;
    let mut end = body.len();
    for (at, ch) in body.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = at;
                    break;
                }
            }
            _ => {}
        }
    }
    let inside = &body[..end];
    assert!(
        inside.contains(") OR ("),
        "the keep-list is not the left branch of an OR:\n{inside}"
    );
    for clause in ["set_type", "layout", "vintage"] {
        assert!(
            inside.contains(clause),
            "the {clause} filter is outside the keep-list's bracket, so a \
             kept card is still dropped by it"
        );
    }
    let after = &body[end..];
    assert!(
        !after.contains("AND"),
        "a filter was added after the keep-list stopped applying:\n{after}"
    );
}

/// A `DROP` is the one statement here that can reach out of the schema it
/// is run in: `CREATE … IF NOT EXISTS` builds in `current_schema()`, but
/// a bare `DROP … IF EXISTS` resolves along the whole search path. A test
/// puts the catalog in a sandbox schema with `public` behind it, so an
/// unqualified drop there deletes the *developer's* index — which is
/// exactly what happened the first time this ran.
#[test]
fn nothing_is_dropped_outside_the_schema_it_was_created_in() {
    for sql in schema_statements() {
        if !sql.contains("DROP ") {
            continue;
        }
        assert!(
            sql.contains("current_schema()"),
            "a drop that can reach past its own schema:\n{sql}"
        );
    }
}

/// Every statement has to be safe to run against an existing database,
/// because the gateway applies them on every start.
///
/// Four spellings say that, not one. `IF NOT EXISTS` creates what is
/// missing, `IF EXISTS` drops what is left over, `CREATE OR REPLACE`
/// writes a function body over whatever is there, and `ON CONFLICT` seeds
/// a row without minding that it is already seeded.
#[test]
fn every_schema_statement_is_idempotent() {
    for sql in schema_statements() {
        assert!(
            [
                "IF NOT EXISTS",
                "IF EXISTS",
                "CREATE OR REPLACE",
                "ON CONFLICT"
            ]
            .iter()
            .any(|guard| sql.contains(guard)),
            "not idempotent: {sql}"
        );
    }
}

/// The projection's plain columns are written by `project()` and its one
/// generated column by the server. Making `names_norm` generated too
/// would be the obvious tidying-up and would be wrong: it is folded
/// through an extension's dictionary that the search path resolves, and a
/// `STORED` column computed from that is recomputed only when the row is
/// written — so a rebuild could not correct it.
#[test]
fn only_the_bigrams_are_generated() {
    let table = schema_statements()
        .into_iter()
        .find(|s| s.contains("CREATE TABLE IF NOT EXISTS card_search"))
        .expect("the projection is part of the schema");
    assert_eq!(
        table.matches("GENERATED").count(),
        1,
        "exactly one column of the projection is the server's to compute:\n{table}"
    );
    assert!(
        table.contains("bg         text[] GENERATED"),
        "and it is the bigram array:\n{table}"
    );
}

/// Every language a card in the catalog is printed in has a name a person
/// can read, and the codes are Scryfall's rather than ISO's — `zhs` and
/// `zht` are language codes nowhere else.
#[test]
fn every_language_the_catalog_stores_is_named_in_itself() {
    let seed = language_seed();
    // Measured against the live catalog: `SELECT DISTINCT lang FROM cards`.
    for code in [
        "en", "ja", "fr", "de", "es", "it", "zhs", "pt", "zht", "ru", "ko", "ph", "qya", "dw",
        "grc", "ar", "la", "sa", "he",
    ] {
        assert!(seed.contains(&format!("('{code}', ")), "no name for {code}");
    }
}

/// The placeholders a statement writes, in order: `$1`, `$2`, ….
fn placeholders(sql: &str) -> Vec<usize> {
    sql.split('$')
        .skip(1)
        .map(|rest| {
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            rest[..digits].parse().expect("a numbered placeholder")
        })
        .collect()
}

/// Two printings, the second with two faces.
fn two_printings() -> Vec<scryfall::Card> {
    serde_json::from_str(
        r#"[{"id":"00000000-0000-0000-0000-000000000001","oracle_id":"00000000-0000-0000-0000-0000000000a1",
                 "lang":"de","set":"m10","collector_number":"1","name":"One",
                 "legalities":{"modern":"legal"}},
                {"id":"00000000-0000-0000-0000-000000000002","oracle_id":"00000000-0000-0000-0000-0000000000a2",
                 "lang":"en","set":"mid","collector_number":"2","name":"Front // Back",
                 "layout":"transform","legalities":{"modern":"legal"},
                 "card_faces":[{"name":"Front"},{"name":"Back"}]}]"#,
    )
    .expect("decodes")
}

/// Every statement numbers its placeholders `$1` to `$n` once each and
/// binds exactly `n` values, `columns` of them a row. The count used to
/// be written apart from the column list, and the face insert bound
/// thirteen where the batch guard counted twelve (#174). Postgres
/// rejects a mismatch, but only against a live database.
#[test]
fn every_statement_binds_exactly_the_placeholders_it_writes() {
    let cards = two_printings();
    let cards: Vec<&scryfall::Card> = cards.iter().collect();
    let faces = face_rows(&cards);
    assert_eq!(faces.len(), 3, "one face and two");
    let legalities: Vec<(&str, &scryfall::Card)> = cards
        .iter()
        .map(|c| (c.oracle_identity().expect("an oracle id"), *c))
        .collect();

    for (what, (sql, values), rows, columns) in [
        ("cards", cards_statement(&cards), cards.len(), CARD_COLUMNS),
        ("faces", faces_statement(&faces), faces.len(), FACE_COLUMNS),
        (
            "legalities",
            legalities_statement(&legalities),
            legalities.len(),
            LEGALITY_COLUMNS,
        ),
    ] {
        let written = placeholders(&sql);
        assert_eq!(
            written,
            (1..=values.len()).collect::<Vec<_>>(),
            "{what}: placeholders against {} bound values",
            values.len()
        );
        assert_eq!(values.len(), rows * columns, "{what}: values per row");
    }
}

/// A full chunk is as big as a statement may be and one row more would
/// not be, measured on the statement that is built rather than on the
/// arithmetic that sizes it.
#[test]
fn a_full_chunk_binds_at_most_the_protocol_s_cap() {
    let cards = two_printings();
    let face = cards[1].faces().remove(0);
    let id = cards[1].id.as_str();
    let per = rows_per_statement(FACE_COLUMNS);
    let full: Vec<FaceRow<'_>> = (0..per).map(|_| (id, 0, face.clone())).collect();
    let (_, values) = faces_statement(&full);
    assert!(values.len() <= MAX_BIND_PARAMS, "{} bound", values.len());
    assert!(
        values.len() + FACE_COLUMNS > MAX_BIND_PARAMS,
        "the chunk could carry another row"
    );
    let card: &scryfall::Card = &cards[0];
    let full: Vec<&scryfall::Card> = (0..rows_per_statement(CARD_COLUMNS))
        .map(|_| card)
        .collect();
    let (_, values) = cards_statement(&full);
    assert!(values.len() <= MAX_BIND_PARAMS, "{} bound", values.len());
}

/// A column added to the insert but not to the table, or added to the
/// table but never backfilled onto an existing one, both fail only at
/// ingest time against a real server.
#[test]
fn every_inserted_column_exists_and_can_be_added_to_an_older_catalog() {
    let statements = schema_statements();
    let create = statements
        .iter()
        .find(|s| s.contains("CREATE TABLE IF NOT EXISTS cards"))
        .expect("the cards table is part of the schema");
    let alter = statements
        .iter()
        .find(|s| s.starts_with("ALTER TABLE cards"))
        .expect("the upgrade path is part of the schema");
    // `scryfall_id` through `released_at` predate the printing columns and
    // are in every catalog that ever existed; the rest arrived later and
    // have to be reachable by an ALTER too.
    let original = [
        "scryfall_id",
        "oracle_id",
        "lang",
        "set_code",
        "collector_number",
        "rarity",
        "layout",
        "released_at",
    ];
    for column in CARD_INSERT_COLUMNS.split(',').map(str::trim) {
        assert!(
            create.contains(&format!("{column} ")),
            "{column} is inserted but not declared"
        );
        if !original.contains(&column) {
            assert!(
                alter.contains(&format!("IF NOT EXISTS {column} ")),
                "{column} is new, so an existing catalog cannot gain it"
            );
        }
    }
}

/// The same for the face insert: `flavor_name` arrived after the table
/// did, so it is the one an older catalog gains by `ALTER`.
#[test]
fn every_inserted_face_column_exists_and_can_be_added_to_an_older_catalog() {
    let statements = schema_statements();
    let create = statements
        .iter()
        .find(|s| s.contains("CREATE TABLE IF NOT EXISTS card_faces"))
        .expect("the faces table is part of the schema");
    for column in FACE_INSERT_COLUMNS.split(',').map(str::trim) {
        assert!(
            create.contains(&format!("{column} ")),
            "{column} is inserted but not declared"
        );
    }
    assert!(
        statements
            .iter()
            .any(|s| s == "ALTER TABLE card_faces ADD COLUMN IF NOT EXISTS flavor_name text"),
        "an existing catalog cannot gain flavor_name"
    );
}

/// Scryfall omits `finishes` on some records rather than writing the
/// obvious value, and an empty list reaches the picker as a card that
/// cannot be added at all.
#[test]
fn a_printing_that_names_no_finish_is_still_available_plain() {
    let quiet = scryfall::Card::default();
    assert_eq!(quiet.finish_list(), vec!["nonfoil".to_string()]);

    let shiny = scryfall::Card {
        finishes: vec!["nonfoil".to_string(), "foil".to_string()],
        ..scryfall::Card::default()
    };
    assert_eq!(shiny.finish_list(), vec!["nonfoil", "foil"]);
}

/// The tag columns are stored joined and read back split. A trailing
/// comma, an empty column and a single tag all have to survive that.
#[test]
fn a_joined_tag_column_round_trips() {
    assert_eq!(split_list(""), Vec::<String>::new());
    assert_eq!(split_list("foil"), vec!["foil"]);
    assert_eq!(
        split_list("nonfoil,foil,etched"),
        vec!["nonfoil", "foil", "etched"]
    );
    assert_eq!(
        split_list("showcase, extendedart,"),
        vec!["showcase", "extendedart"]
    );
}

/// A printing's wire shape is what the deck builder's picker renders, and
/// the client defines its own struct for it — same reason as the text
/// entry above, same protection.
#[test]
fn the_printing_wire_shape_is_pinned() {
    let printing = Printing {
        scryfall_id: "id".to_string(),
        oracle_id: "oid".to_string(),
        lang: "ja".to_string(),
        set: "neo".to_string(),
        set_name: "Kamigawa: Neon Dynasty".to_string(),
        collector_number: "123".to_string(),
        rarity: "rare".to_string(),
        released_at: "2022-02-18".to_string(),
        artist: "Someone".to_string(),
        finishes: vec!["nonfoil".to_string(), "foil".to_string()],
        frame_effects: vec!["showcase".to_string()],
        border_color: "black".to_string(),
        promo: false,
        name: "御守り".to_string(),
        layout: "transform".to_string(),
    };
    assert_eq!(
        serde_json::to_string(&printing).expect("serializes"),
        r#"{"scryfall_id":"id","oracle_id":"oid","lang":"ja","set":"neo","set_name":"Kamigawa: Neon Dynasty","collector_number":"123","rarity":"rare","released_at":"2022-02-18","artist":"Someone","finishes":["nonfoil","foil"],"frame_effects":["showcase"],"border_color":"black","promo":false,"name":"御守り","layout":"transform"}"#
    );
}
