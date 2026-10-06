//! The catalog's SQL that is more than a line: the normaliser, the
//! projection, card text, the corpus, the type-name rounds and the search.

/// The advisory lock a rebuild holds.
///
/// Two gateways starting against one database both find the version stale,
/// and the lock is where the second one stops — but only because the
/// staleness question is asked *inside* the transaction that holds it
/// ([`Catalog::rebuild_if_stale`]). The loser waits, asks again, reads the
/// version the winner stamped, and does nothing. A lock taken after the
/// question would serialise two rebuilds rather than prevent one, which is
/// what `TRUNCATE` does for free and why the lock would then be decoration.
pub(crate) const PROJECT_LOCK: i64 = 0x000b_a11e_eca7_a106;

/// Serialises the DDL in [`Catalog::migrate`].
///
/// **`IF NOT EXISTS` is a check and an insert, and nothing holds them
/// together.** Two sessions migrating at once both find `unaccent` missing,
/// both insert, and the loser gets `duplicate key value violates unique
/// constraint "pg_extension_name_index"` — not a wrong schema, but a failed
/// migration, which is worse than either outcome it was protecting against.
/// It is invisible wherever the extension already exists, so it does not
/// happen on a development machine and does happen on a CI server that
/// starts an empty PostgreSQL and then runs the e2e tests in parallel
/// schemas: three tests died on it there while every local run was green.
/// `CREATE TABLE`/`CREATE INDEX IF NOT EXISTS` race the same way, so the
/// lock is around the whole loop and not around the one statement that was
/// caught.
pub(crate) const SCHEMA_LOCK: i64 = 0x000b_a11e_5c4e_3a01;

/// The body of `catalog_norm`, which a query and a stored name are both
/// folded through.
///
/// Three foldings, and each one is a language this catalog serves. `NFKC`
/// turns the full-width Latin a Japanese keyboard produces (`Ｌｉｇｈｔｎｉｎｇ`)
/// into the ASCII a deck builder stores; `unaccent` makes `Æther` reachable
/// by typing `aether`; `lower` is the rest. `unaccent`'s two-argument form is
/// used rather than the one-argument one because only the two-argument form
/// is `IMMUTABLE` — naming the dictionary is what makes this function honest
/// about being one.
pub(crate) const NORM_BODY: &str = "SELECT lower(unaccent('unaccent', normalize(s, NFKC)))";

/// The body of `catalog_bigrams`, which the name index is built from.
///
/// Bigrams rather than trigrams, and the reason is a two-character card name.
/// `pg_trgm` pads a whole *word*, so `show_trgm('稲妻')` does return three
/// trigrams — but a `%稲妻%` pattern is not padded, so the index it built
/// could not serve the query that needed it, and a search for `稲妻` was a
/// sequential scan over 554 242 faces at 2742 ms. Every non-ASCII character
/// is also indexed on its own, because `島` is a card name and a whole word.
pub(crate) const BIGRAMS_BODY: &str = "SELECT coalesce(array_agg(DISTINCT g), '{}'::text[]) FROM ( \
       SELECT substr(s, i, 2) AS g FROM generate_series(1, greatest(length(s) - 1, 0)) i \
       UNION ALL \
       SELECT substr(s, i, 1) FROM generate_series(1, length(s)) i \
        WHERE substr(s, i, 1) !~ '[[:ascii:]]' \
     ) t";

/// Fills `card_search` from `cards` and `card_faces`.
///
/// The inner query is one row per *printed text* — `DISTINCT ON (oracle_id,
/// lang, face_index)` over 542 177 printings leaves 296 313 — and the outer
/// one collapses those to 41 991 oracle faces, each carrying every language
/// it is printed in. Which printing speaks for a language is decided here
/// rather than in the search, and the order is the one the search used to
/// carry: a printing that actually has a translated type line first, because
/// 6489 of 59 465 German faces have a `printed_name` and no
/// `printed_type_line`, and taking one of those heads a German card
/// `Instant`.
///
/// The `LEFT JOIN` is the half that reaches cards nobody printed abroad.
/// Merging every printed type line already makes `同盟者`, `Ally` and
/// `Kleriker` reach the right cards — but only cards somebody printed that
/// way, and **3634 cards have no German printing at all**. So `type_names`
/// says what `Ally` is called in ten languages and the words go in whether or
/// not the printing exists. Measured here: 89.5% of the subtype occurrences
/// on those 3634 cards can be named in German, against none before.
///
/// It is a join inside this `INSERT` and not an `UPDATE` afterwards, which is
/// the shape it was written in first. Both produce the same `tsvector` —
/// `tsv || tsv` deduplicates the tokens — but the update writes every row a
/// second time, and the whole cost of the translation turned out to be that
/// second write. Measured on the live catalog, `card_search` in total:
/// **215 MB** before any of this, **413 MB** written twice, **230 MB** as it
/// stands. The stored text grew 76 MB to 85 MB in both, so the translation
/// itself costs 15 MB and the other 183 MB was the rewrite. A `VACUUM` finds
/// nothing to report about it either, because the dead half is reusable space
/// inside the files rather than dead tuples, and only the next `TRUNCATE`
/// gives it back. The join is also the faster of the two: 23.8 s against
/// 32.8 s for a full rebuild.
///
/// The dictionary is joined on **whole words**, not with a `LIKE` over the
/// type line. Padding the line and matching `'% ' || english || ' %'` would
/// also find a *multi-word* subtype, and that shape was measured too: 22.8 s
/// against 1.3 s, a nested loop rejecting 178 million pairs. Magic prints
/// exactly one multi-word subtype, `Time Lord`, which no printing anywhere
/// translates — so the general shape costs seventeen times the time to reach
/// a card that is unsearchable in German either way.
///
/// The left side of the type line is looked up twice: as the whole phrase
/// (`Legendary Creature` is `Legendäre Kreatur`, and a supertype is never
/// decomposed — `Basic Land` is one German word for two) and as its last word
/// alone, the card type. The second is what catches the 28 faces whose exact
/// combination of supertypes was never printed abroad, which would otherwise
/// get no word at all.
///
/// The second `LEFT JOIN` carries **flavor names** — the just-for-fun name a
/// Secret Lair prints instead of the card's own, with the real one in small
/// type beside it. It is joined separately, and not folded into the inner
/// query, because that one keeps one printing per language (`DISTINCT ON`)
/// and a flavor name belongs to the *printing*: Command Tower has six of
/// them, and picking one printing would throw five away. 476 cards in 661
/// printings carry one. They reach `names_norm`, so typing `Cybertron` finds
/// Command Tower at the same tier as typing its own name, and `bg` follows
/// because it is `GENERATED` from that column. They reach neither `names` —
/// which answers "what is this card called in your language", and a flavor
/// name is not a language — nor anything the rules read: `oracle_id` and the
/// Oracle name are unchanged, which is the whole reason this is a search
/// concern and not a card one.
///
/// **A printed English name is not a language either**, and that is the same
/// argument one step in. `names` took `coalesce(printed_name, name)` from the
/// newest printing of each language, which is exactly right abroad — the
/// printed name *is* the German name — and wrong at home, where `name` is the
/// Oracle name and `printed_name` is how one Secret Lair chose to set the
/// type. 32 of 41 991 faces answered `names->>'en'` with a styling:
/// `BIRDS OF PARADISE`, `IMP'S MSCHF`, `GIGANTO-SAURUS` (#35). So English
/// takes `name` and every other language keeps the printed one.
///
/// The styling is not lost, it moves. `pn` is the printed spelling whatever
/// the language, and it joins `names_norm` and `tsv` where it differs from
/// the display name — the same place a flavor name lives, for the same
/// reason: somebody holding the card and typing what is on it has to find it.
pub(crate) const PROJECT_SQL: &str = "\
    WITH face AS ( \
      SELECT DISTINCT ON (c.oracle_id, cf.face_index) \
             c.oracle_id, cf.face_index, cf.type_line \
      FROM cards c JOIN card_faces cf USING (scryfall_id) \
      WHERE cf.type_line IS NOT NULL \
      ORDER BY c.oracle_id, cf.face_index, (c.lang = 'en') DESC, c.scryfall_id \
    ) \
    INSERT INTO card_search (oracle_id, face_index, names, names_norm, tsv) \
    SELECT p.oracle_id, p.face_index, p.names, \
           p.names_norm || coalesce(v.extra_norm, ''), \
           p.tsv || coalesce(t.extra, ''::tsvector) \
                 || coalesce(v.extra_tsv, ''::tsvector) \
    FROM ( \
      SELECT oracle_id, face_index, \
             jsonb_object_agg(lang, nm) AS names, \
             '| ' || catalog_norm(string_agg(DISTINCT nm, ' | ') \
               || coalesce(' | ' || string_agg(DISTINCT pn, ' | ') \
                             FILTER (WHERE pn <> nm), '')) || ' |' AS names_norm, \
             to_tsvector('simple', string_agg(nm || ' ' || tl || ' ' || tx, ' ') \
               || coalesce(' ' || string_agg(pn, ' ') FILTER (WHERE pn <> nm), '')) AS tsv \
      FROM ( \
        SELECT DISTINCT ON (c.oracle_id, c.lang, f.face_index) \
               c.oracle_id, c.lang AS lang, f.face_index, \
               CASE WHEN c.lang = 'en' THEN f.name \
                    ELSE coalesce(f.printed_name, f.name) END AS nm, \
               coalesce(f.printed_name, f.name) AS pn, \
               coalesce(f.printed_type_line, f.type_line, '') AS tl, \
               coalesce(f.printed_text, f.oracle_text, '') AS tx \
        FROM cards c JOIN card_faces f USING (scryfall_id) \
        ORDER BY c.oracle_id, c.lang, f.face_index, \
                 (f.printed_type_line IS NOT NULL) DESC, \
                 c.released_at DESC NULLS LAST, c.scryfall_id \
      ) one_per_language \
      GROUP BY oracle_id, face_index \
    ) p LEFT JOIN ( \
      SELECT w.oracle_id, w.face_index, \
             to_tsvector('simple', string_agg(DISTINCT d.printed, ' ')) AS extra \
      FROM ( \
        SELECT oracle_id, face_index, split_part(type_line, ' — ', 1) AS english FROM face \
        UNION ALL \
        SELECT oracle_id, face_index, regexp_replace(split_part(type_line, ' — ', 1), '^.* ', '') \
        FROM face \
        UNION ALL \
        SELECT oracle_id, face_index, unnest(string_to_array(split_part(type_line, ' — ', 2), ' ')) \
        FROM face WHERE type_line LIKE '% — %' \
      ) w JOIN type_names d USING (english) \
      GROUP BY w.oracle_id, w.face_index \
    ) t USING (oracle_id, face_index) \
    LEFT JOIN ( \
      SELECT c.oracle_id, f.face_index, \
             ' ' || catalog_norm(string_agg(DISTINCT f.flavor_name, ' | ')) || ' |' \
               AS extra_norm, \
             to_tsvector('simple', string_agg(DISTINCT f.flavor_name, ' ')) AS extra_tsv \
      FROM cards c JOIN card_faces f USING (scryfall_id) \
      WHERE f.flavor_name IS NOT NULL AND f.flavor_name <> '' \
      GROUP BY c.oracle_id, f.face_index \
    ) v USING (oracle_id, face_index)";

/// Every printing [`Catalog::text_by_card`] chooses among, with its faces.
///
/// All of a card's printings in the asked language, and its newest English
/// one: that carries the Oracle and the English names, and it is what an
/// English reader is served. English printings beyond the newest are not
/// read — under `en` the Oracle is drawn, never a printing's own wording —
/// which keeps a basic land's several hundred English printings out of the
/// answer. Ordered newest first within a card, faces in printed order, and
/// every sort ends on a unique key.
pub(crate) const TEXT_SQL: &str = "\
    WITH wanted AS (SELECT DISTINCT unnest(string_to_array($1, ','))::uuid AS oracle_id), \
    printing AS ( \
      SELECT c.scryfall_id, c.oracle_id, c.lang, c.released_at, c.collector_number, c.layout \
      FROM wanted w JOIN cards c ON c.oracle_id = w.oracle_id \
      WHERE c.lang = $2 AND $2 <> 'en' \
      UNION ALL \
      SELECT e.scryfall_id, e.oracle_id, e.lang, e.released_at, e.collector_number, e.layout \
      FROM wanted w CROSS JOIN LATERAL ( \
        SELECT c.scryfall_id, c.oracle_id, c.lang, c.released_at, c.collector_number, c.layout \
        FROM cards c WHERE c.oracle_id = w.oracle_id AND c.lang = 'en' \
        ORDER BY c.released_at DESC NULLS LAST, length(c.collector_number), \
                 c.collector_number, c.scryfall_id \
        LIMIT 1 \
      ) e \
    ) \
    SELECT p.scryfall_id::text AS scryfall_id, p.oracle_id::text AS oracle_id, p.lang AS lang, \
           coalesce(to_char(p.released_at, 'YYYY-MM-DD'), '') AS released_at, \
           p.collector_number AS collector_number, coalesce(p.layout, '') AS layout, \
           f.name AS name, f.printed_name AS printed_name, \
           f.type_line AS type_line, f.printed_type_line AS printed_type_line, \
           f.oracle_text AS oracle_text, f.printed_text AS printed_text, \
           f.mana_cost AS mana_cost \
    FROM printing p JOIN card_faces f ON f.scryfall_id = p.scryfall_id \
    ORDER BY p.oracle_id, p.released_at DESC NULLS LAST, length(p.collector_number), \
             p.collector_number, p.scryfall_id, f.face_index";

/// The card corpus, in the order the cards first appeared.
///
/// This is where a `CardIndex` comes from. An index is assigned by first
/// appearance and never moves again, so the order below is the only thing
/// that decides one, and it has to be **total**: release date, then the
/// English name, then the oracle id, which no two cards share. Two rows that
/// tie on all three are the same card.
///
/// "First appearance" is the earliest printing in *any* language, because
/// that is when the card appeared; English only breaks a tie on the same day,
/// so the set recorded beside it is the one a person would name.
///
/// Three clauses decide what counts as a card at all, and all three are
/// Scryfall's own words rather than a list of set codes this repo would have
/// to keep correct. `set_type` drops the sets that print souvenirs:
/// `memorabilia` is art cards and the challenge decks, `token` is what it
/// says. `layout` drops the things shaped like cards that turn up inside
/// ordinary sets — a Commander Collection's Snake token is `arsenal`, not
/// `token`. What is left keeps every layout the rules have a type for, planes
/// and schemes and Vanguard avatars included, because [`baylee_core`]'s type
/// bits already do.
///
/// The third clause is the interesting one, and it is why a joke set is not
/// dropped wholesale. Unfinity printed 266 tournament-legal cards beside its
/// acorn ones — same set, same black border — so neither `set_type` nor the
/// border colour separates them. Legality does, in its weakest sense:
/// `not_legal` means no format has ever heard of the card, while `banned` is
/// a real card that a format has an opinion about. It is asked **only**
/// inside a joke set, because planes, schemes and Vanguard avatars are
/// `not_legal` too and are perfectly real.
///
/// All three sit inside one `OR`, because a rule this crate owns cannot see
/// the other half of the question. `data/corpus-keep.tsv` names the cards
/// *this repo implements* and the rule drops — eight acorn lands written
/// before it existed — and codegen cannot build a card with no row in the
/// ledger, so dropping one is not a tidier corpus but a card that stops
/// compiling. A kept card is admitted whole rather than appended: its set and
/// its place in the order come out of this same query, which is the only
/// reason the `set` column beside it is worth freezing.
///
/// [`baylee_core`]: https://docs.rs/baylee-core
pub(crate) const CORPUS_SQL: &str = "\
    WITH first_printing AS ( \
      SELECT DISTINCT ON (c.oracle_id) \
             c.oracle_id, c.scryfall_id, c.released_at, c.set_code \
      FROM cards c \
      WHERE c.released_at IS NOT NULL \
        AND (c.oracle_id::text = ANY(string_to_array($1, ',')) OR ( \
          coalesce(c.set_type, '') NOT IN ('memorabilia', 'token') \
          AND (coalesce(c.set_type, '') <> 'funny' OR EXISTS ( \
                SELECT 1 FROM card_legalities g \
                WHERE g.oracle_id = c.oracle_id \
                  AND coalesce(g.legalities ->> 'vintage', 'not_legal') \
                        <> 'not_legal')) \
          AND coalesce(c.layout, '') \
                NOT IN ('token', 'double_faced_token', 'art_series', 'emblem'))) \
      ORDER BY c.oracle_id, c.released_at, (c.lang = 'en') DESC, \
               c.set_code, c.collector_number \
    ) \
    SELECT p.oracle_id::text AS oracle_id, to_char(p.released_at, 'YYYY-MM-DD') AS released_at, \
           p.set_code AS set_code, \
           string_agg(f.name, ' // ' ORDER BY f.face_index) AS name \
    FROM first_printing p JOIN card_faces f USING (scryfall_id) \
    WHERE f.name <> '' \
    GROUP BY p.oracle_id, p.released_at, p.set_code \
    ORDER BY released_at, name, oracle_id";

/// The three readings [`Catalog::mine_type_names`] makes, in order.
///
/// Each one may only write a pair it understood in full, which is the same
/// rule the card transcoder obeys and for the same reason: a wrong row here
/// is a German word attached to the wrong cards in everybody's search, and
/// nothing downstream can tell it from a right one.
///
/// **Round 0, the type line's left side**, as one phrase. Supertypes are not
/// decomposed — German prints `Basic Land` as `Standardland`, one word for
/// two, and Spanish lowercases and reorders its adjectives.
///
/// **Round 1, cards with exactly one subtype.** The clean signal: the whole
/// printed segment is the whole translation, with nothing to split. It
/// reaches 316 of the 506 subtypes in German.
///
/// **Round 2, subtraction.** A card with several subtypes whose printed
/// segment tokenises into forms round 0 and 1 already know, plus exactly one
/// leftover, and whose English segment has exactly one unknown — then the two
/// leftovers are each other. This is what `Druid`, `Warlock`, `Advisor`,
/// `Ninja` and `Ally` come from: 62 more subtypes in German, and 79.7% to
/// 89.4% of the subtype occurrences on cards with no German printing. There
/// is no round 3; the remainder does not fall to more passes, it falls to
/// Wizards printing those cards in German.
///
/// Where the rounds disagree, the **most frequent** form wins and the
/// **newest** printing breaks a tie. That is not a style choice: 48 of 317
/// German cells hold more than one form, and reading them showed the extra
/// forms are old type lines and errata — `Löwe` and `Tiger` on cards Scryfall
/// now calls `Cat`, `Engellegende` from when Legend was a card type. Both
/// rules agree everywhere but one cell (`Orgg`, a 2–2 tie the newest printing
/// settles), so the tiebreak is doing exactly the work it claims to.
pub(crate) const MINE_ROUNDS: [&str; 4] = [
    "CREATE TEMP TABLE mined (english text, lang text, printed text, round int) ON COMMIT DROP",
    // Round 0: the left side.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 0 FROM ( \
       SELECT c.lang, c.released_at, \
              CASE WHEN f.type_line LIKE '% — %' \
                   THEN split_part(f.type_line, ' — ', 1) ELSE f.type_line END AS english, \
              btrim(CASE WHEN f.printed_type_line ~ '( — | : |～| - )' \
                         THEN regexp_replace(f.printed_type_line, '( — | : |～| - ).*$', '') \
                         ELSE f.printed_type_line END) AS printed \
       FROM cards c JOIN card_faces f USING (scryfall_id) \
       WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL AND f.type_line IS NOT NULL \
     ) p WHERE printed <> '' \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
    // Round 1: one subtype, one translation.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 1 FROM ( \
       SELECT lang, released_at, subs[1] AS english, printed FROM ( \
         SELECT c.lang, c.released_at, \
                string_to_array(split_part(f.type_line, ' — ', 2), ' ') AS subs, \
                btrim(regexp_replace(f.printed_type_line, '^.*?( — | : |～| - )', '')) AS printed \
         FROM cards c JOIN card_faces f USING (scryfall_id) \
         WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL \
           AND f.type_line LIKE '% — %' AND f.printed_type_line ~ '( — | : |～| - )' \
       ) seg WHERE array_length(subs, 1) = 1 AND subs[1] <> '' \
     ) s WHERE printed <> '' \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
    // Round 2: strike out what is already known and see what is left.
    //
    // The tokeniser has to serve every language at once: German separates
    // subtypes with a comma, Japanese with `・`, Simplified Chinese with `／`,
    // and the Romance languages with a space and sometimes a conjunction
    // (`humain et clerc`), which is not a subtype and would otherwise be
    // learned as one the first time both its neighbours were known.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 2 FROM ( \
       SELECT lang, released_at, unknown[1] AS english, leftover[1] AS printed FROM ( \
         SELECT s.lang, s.released_at, \
                (SELECT array_agg(x) FROM unnest(s.subs) x WHERE x <> '' \
                  AND NOT EXISTS (SELECT 1 FROM mined d \
                                   WHERE d.lang = s.lang AND d.english = x)) AS unknown, \
                (SELECT array_agg(t) FROM unnest( \
                   regexp_split_to_array(s.printed, '[,、，／/・]|\\s+')) t \
                  WHERE btrim(t) <> '' \
                    AND lower(t) NOT IN ('et', 'y', 'e', 'and', 'und', 'i', 'ed', '和') \
                    AND NOT EXISTS (SELECT 1 FROM mined d \
                                     WHERE d.lang = s.lang \
                                       AND lower(d.printed) = lower(t))) AS leftover \
         FROM ( \
           SELECT c.lang, c.released_at, \
                  string_to_array(split_part(f.type_line, ' — ', 2), ' ') AS subs, \
                  btrim(regexp_replace(f.printed_type_line, '^.*?( — | : |～| - )', '')) AS printed \
           FROM cards c JOIN card_faces f USING (scryfall_id) \
           WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL \
             AND f.type_line LIKE '% — %' AND f.printed_type_line ~ '( — | : |～| - )' \
         ) s WHERE array_length(s.subs, 1) > 1 \
       ) c WHERE array_length(unknown, 1) = 1 AND array_length(leftover, 1) = 1 \
     ) s \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
];

/// The search, as one statement, so a test can read the shape of it.
///
/// The representative printing is picked per language, and **English asks the
/// Oracle fields**. `printed_name` and `printed_type_line` are a translation
/// abroad and a styling or an obsolete wording at home, so the preference for
/// a printing that carries a printed type line — written for the 6489 German
/// faces that have a name and no type — was heading English searches with the
/// oldest wording it could find: `birds of paradise` answered
/// `BIRDS OF PARADISE — Summon Bird` against the live catalog, a styling from
/// one Secret Lair over a type line Magic stopped printing in 1994 (#35).
///
/// It is a free function rather than a `const` because the tests below assert
/// about its *structure* — that the two tiers are unioned rather than `OR`ed,
/// and that the fence the tiers read is the one the projection writes — and a
/// constant would say nothing about where either half is used.
///
/// The `LIMIT` sits at the end of `ranked`, so the rows it counts have to be
/// rows `picked` can still resolve. `card_search` holds one row per card in
/// *every* language, and `picked` keeps only a printing in the asked-for
/// language or in English — so a card with neither would be counted against
/// the limit and then vanish, and a caller asking for twenty would be handed
/// nineteen. Eight cards in this catalog have no English printing at all
/// (the Japanese Dreamcast promos, `psdg`), which is few enough that the
/// symptom would have been read as a search that simply found less.
pub(crate) fn search_sql() -> &'static str {
    "\
        WITH q AS (SELECT catalog_norm($1) AS n, $1 AS raw, $2 AS lang), \
        named AS ( \
          SELECT s.oracle_id, s.face_index, \
                 CASE WHEN position('| ' || q.n || ' |' in s.names_norm) > 0 THEN 0 \
                      WHEN position('| ' || q.n in s.names_norm) > 0 THEN 1 \
                      WHEN position(' ' || q.n in s.names_norm) > 0 THEN 2 \
                      ELSE 3 END AS tier \
          FROM card_search s CROSS JOIN q \
          WHERE s.bg @> catalog_bigrams(q.n) \
            AND position(q.n in s.names_norm) > 0 \
        ), \
        texted AS ( \
          SELECT s.oracle_id, s.face_index, 4 AS tier \
          FROM card_search s CROSS JOIN q \
          WHERE s.tsv @@ plainto_tsquery('simple', q.raw) \
        ), \
        hit AS ( \
          SELECT DISTINCT ON (oracle_id) oracle_id, face_index, tier \
          FROM (SELECT * FROM named UNION ALL SELECT * FROM texted) tiers \
          ORDER BY oracle_id, tier, face_index \
        ), \
        ranked AS ( \
          SELECT h.oracle_id, h.face_index, \
                 row_number() OVER ( \
                   ORDER BY h.tier, jsonb_exists(s.names, q.lang) DESC, \
                            length(coalesce(s.names ->> q.lang, s.names ->> 'en', '')), \
                            coalesce(s.names ->> q.lang, s.names ->> 'en') \
                 ) AS nth \
          FROM hit h JOIN card_search s USING (oracle_id, face_index) CROSS JOIN q \
          WHERE jsonb_exists(s.names, q.lang) OR jsonb_exists(s.names, 'en') \
          ORDER BY nth \
          LIMIT $3 \
        ), \
        picked AS ( \
          SELECT DISTINCT ON (r.oracle_id) r.nth, \
                 c.scryfall_id::text AS scryfall_id, c.lang AS lang, \
                 CASE WHEN c.lang = 'en' THEN f.name \
                      ELSE coalesce(f.printed_name, f.name) END AS display_name, \
                 f.name AS english_name, \
                 CASE WHEN c.lang = 'en' THEN coalesce(f.type_line, '') \
                      ELSE coalesce(f.printed_type_line, f.type_line, '') \
                      END AS display_type \
          FROM ranked r \
          JOIN cards c ON c.oracle_id = r.oracle_id \
          JOIN card_faces f ON f.scryfall_id = c.scryfall_id \
                           AND f.face_index = r.face_index \
          CROSS JOIN q \
          WHERE c.lang IN (q.lang, 'en') \
          ORDER BY r.oracle_id, (c.lang = q.lang) DESC, \
                   (c.lang <> 'en' AND f.printed_type_line IS NOT NULL) DESC, \
                   c.released_at DESC NULLS LAST, c.scryfall_id \
        ) \
        SELECT scryfall_id, lang, display_name, english_name, display_type \
        FROM picked ORDER BY nth"
}
