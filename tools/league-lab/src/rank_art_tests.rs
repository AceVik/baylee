use super::{NAMES, ROMAN, SELECTED, Scale};
use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::Path,
};

#[test]
fn artwork_covers_each_published_rank_in_both_families() {
    let scale = Scale::new(SELECTED.0, SELECTED.1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/baylee/ranks");
    let mut hashes = BTreeSet::new();
    for family in ["player", "deck"] {
        let dir = root.join(family);
        let mut expected = BTreeSet::new();
        if family == "player" {
            for place in 1..=3 {
                expected.insert(format!("champion-{place}.png"));
            }
        }
        for (i, name) in NAMES.iter().enumerate() {
            let slug = name.to_lowercase();
            expected.insert(format!("{slug}.png"));
            for numeral in &ROMAN[..scale.steps[i].len()] {
                expected.insert(format!("{slug}-{}.png", numeral.to_lowercase()));
            }
        }
        let actual: BTreeSet<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(expected, actual, "{family}");
        for file in actual {
            let path = dir.join(&file);
            let img = image::open(&path).unwrap().into_rgba8();
            assert_eq!(img.dimensions(), (384, 384), "{path:?}");
            let mut opaque = 0;
            for (x, y, p) in img.enumerate_pixels() {
                if !(8..376).contains(&x) || !(8..376).contains(&y) {
                    assert_eq!(p[3], 0, "clipped badge {path:?}");
                }
                opaque += usize::from(p[3] >= 128);
            }
            assert!(
                (15_000..110_000).contains(&opaque),
                "coverage {opaque} in {path:?}"
            );
            let mut h = DefaultHasher::new();
            img.as_raw().hash(&mut h);
            assert!(hashes.insert(h.finish()), "duplicate badge {path:?}");
        }
    }
}
