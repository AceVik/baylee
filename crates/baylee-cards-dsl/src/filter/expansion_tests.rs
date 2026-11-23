use super::{ANTIQUITIES_NAMES, ARABIAN_NIGHTS_NAMES, OriginalExpansion};

#[test]
fn every_listed_name_is_found_in_its_own_expansion_and_not_in_the_other() {
    // The lookup is a binary search: one name out of order hides others.
    for (expansion, other, names) in [
        (
            OriginalExpansion::ArabianNights,
            OriginalExpansion::Antiquities,
            ARABIAN_NIGHTS_NAMES,
        ),
        (
            OriginalExpansion::Antiquities,
            OriginalExpansion::ArabianNights,
            ANTIQUITIES_NAMES,
        ),
    ] {
        assert!(!names.is_empty());
        for pair in names.windows(2) {
            assert!(pair[0] < pair[1], "{} before {}", pair[0], pair[1]);
        }
        for name in names {
            assert!(expansion.contains_name(name), "{name}");
            assert!(!other.contains_name(name), "{name}");
        }
    }
}

#[test]
fn a_name_is_matched_whole_and_exactly() {
    let antiquities = OriginalExpansion::Antiquities;
    assert!(antiquities.contains_name("Yotian Soldier"));
    for near in [
        "",
        "Yotian",
        "yotian soldier",
        "Yotian Soldier ",
        "Yotian Soldiers",
        "Lightning Bolt",
    ] {
        assert!(!antiquities.contains_name(near), "{near:?}");
    }
}
