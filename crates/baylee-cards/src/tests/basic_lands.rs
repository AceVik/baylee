//! A union of all basic land types is different from the Basic supertype.

use super::every_card_file;

const SUBTYPES: [&str; 5] = ["PLAINS", "ISLAND", "SWAMP", "MOUNTAIN", "FOREST"];
const ALL_FIVE: u8 = 0b1_1111;

/// Read one subtype predicate or an actual union of subtype predicates.
/// Other filters cannot contribute names merely by occurring nearby.
fn subtype_union_mask(expression: &str) -> Option<u8> {
    if let Some(path) = expression
        .strip_prefix("Filter::HasSubtype(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let (namespace, name) = path.rsplit_once("::")?;
        if !matches!(namespace, "land" | "subtypes::land") {
            return None;
        }
        return SUBTYPES
            .iter()
            .position(|subtype| *subtype == name)
            .map(|i| 1 << i);
    }
    let body = expression
        .strip_prefix("Filter::Or(&[")?
        .strip_suffix("])")?;
    let mut depth = 0_u32;
    body.split(|ch| match ch {
        '(' => {
            depth += 1;
            false
        }
        ')' => {
            depth = depth.saturating_sub(1);
            false
        }
        ',' => depth == 0,
        _ => false,
    })
    .filter(|item| !item.is_empty())
    .try_fold(0, |mask, item| Some(mask | subtype_union_mask(item)?))
}

/// Inspect balanced `Filter::Or` expressions, including nested unions and
/// unions inside a larger filter. Five independent tests are not one union.
fn has_five_type_union(source: &str) -> bool {
    let code: String = source
        .lines()
        .flat_map(|line| line.split_once("//").map_or(line, |(code, _)| code).chars())
        .filter(|ch| !ch.is_whitespace())
        .collect();
    code.match_indices("Filter::Or(").any(|(start, _)| {
        let expression = &code[start..];
        let mut depth = 0_u32;
        for (index, ch) in expression.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return subtype_union_mask(&expression[..=index]) == Some(ALL_FIVE);
                    }
                }
                _ => {}
            }
        }
        false
    })
}

fn prints_basic_land_types(source: &str) -> bool {
    let printed = source
        .lines()
        .filter(|line| line.starts_with("//! Oracle:"))
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase();
    SUBTYPES.iter().all(|subtype| printed.contains(subtype))
        || printed.contains("WITH A BASIC LAND TYPE")
}

fn misstates_basic_land_filter(source: &str) -> bool {
    has_five_type_union(source) && !prints_basic_land_types(source)
}

/// "Basic land card" is the Basic supertype, not a union of the five basic
/// land types. The latter also accepts nonbasic lands with those types.
/// Printed lists of the five types and "with a basic land type" legitimately
/// use that union. Independent type checks elsewhere in the same card do not
/// replace its basic-land filter and must not be combined into one.
#[test]
fn no_card_spells_basic_land_as_the_five_basic_subtypes() {
    let files = every_card_file();
    let offenders: Vec<_> = files
        .iter()
        .filter(|(_, text)| misstates_basic_land_filter(text))
        .map(|(name, _)| name)
        .collect();
    assert!(
        offenders.is_empty(),
        "these files spell \"basic land\" as five subtypes and print no \
         such list — use `Filter::HasSupertype(SupertypeSet::BASIC)`: \
         {offenders:?}"
    );
    assert!(
        files.iter().any(|(_, text)| has_five_type_union(text)),
        "no card in the pool writes the five-type union, so the printing \
         half of this test is unexercised"
    );
}

const FIVE_TYPE_UNION: &str = r"
    Filter::Or(&[
        Filter::HasSubtype(subtypes::land::PLAINS),
        Filter::HasSubtype(subtypes::land::ISLAND),
        Filter::HasSubtype(subtypes::land::SWAMP),
        Filter::HasSubtype(subtypes::land::MOUNTAIN),
        Filter::HasSubtype(subtypes::land::FOREST),
    ])
";

#[test]
fn actual_five_type_union_is_rejected_as_a_basic_land_filter() {
    for filter in [
        FIVE_TYPE_UNION.to_owned(),
        FIVE_TYPE_UNION.replace("subtypes::land", "land"),
        format!("Filter::And(&[Filter::LAND, {FIVE_TYPE_UNION}])"),
    ] {
        let source = format!("//! Oracle: Search your library for a basic land card.\n{filter}");
        assert!(has_five_type_union(&source));
        assert!(misstates_basic_land_filter(&source));
    }
}

#[test]
fn legitimate_printed_five_types_allow_the_same_union() {
    for printed in [
        "Search for a Plains, Island, Swamp, Mountain, or Forest card.",
        "Search for a land card with a basic land type.",
    ] {
        let source = format!("//! Oracle: {printed}\n{FIVE_TYPE_UNION}");
        assert!(has_five_type_union(&source));
        assert!(!misstates_basic_land_filter(&source));
    }
}

#[test]
fn five_independent_type_checks_do_not_replace_a_basic_land_filter() {
    let source = r"
//! Oracle: Sacrifice a basic land: Grant each landwalk of its land types.
        cost!(Sacrifice(&Filter::BASIC_LAND));
        Condition::SacrificedForCost(&Filter::HasSubtype(subtypes::land::PLAINS));
        Condition::SacrificedForCost(&Filter::HasSubtype(subtypes::land::ISLAND));
        Condition::SacrificedForCost(&Filter::HasSubtype(subtypes::land::SWAMP));
        Condition::SacrificedForCost(&Filter::HasSubtype(subtypes::land::MOUNTAIN));
        Condition::SacrificedForCost(&Filter::HasSubtype(subtypes::land::FOREST));
    ";
    assert!(!has_five_type_union(source));
    assert!(!misstates_basic_land_filter(source));
}

#[test]
fn subtype_names_from_separate_unions_are_not_combined() {
    let separate = r"
        Filter::Or(&[
            Filter::HasSubtype(land::PLAINS),
            Filter::HasSubtype(land::ISLAND),
        ]);
        Filter::Or(&[
            Filter::HasSubtype(land::SWAMP),
            Filter::HasSubtype(land::MOUNTAIN),
            Filter::HasSubtype(land::FOREST),
        ]);
    ";
    assert!(!has_five_type_union(separate));
    let nested = format!("Filter::Or(&[Filter::HasSubtype(land::PLAINS), {FIVE_TYPE_UNION}])");
    assert!(has_five_type_union(&nested));
}
