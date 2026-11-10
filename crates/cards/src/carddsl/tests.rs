#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_dsl_encodes_without_debug_strings_and_distinguishes_cards() {
        // Verify that common compiled cards produce bounded, non-zero, distinct
        // summary and ability vectors.
        let mut non_zero = 0_usize;
        for id in 1..500_u32 {
            let Some(def) = crate::by_index(CardIndex::new(id)) else {
                continue;
            };
            if !def.is_fully_read() {
                continue;
            }
            let enc = encode_card_face(def, 0);
            for &v in &enc.summary {
                assert!(
                    (-1.0..=1.0).contains(&v),
                    "card {} summary value {v} out of [-1, 1]",
                    def.name()
                );
            }
            for ab_row in &enc.abilities {
                for &v in ab_row {
                    assert!(
                        (-1.0..=1.0).contains(&v),
                        "card {} ability value {v} out of [-1, 1]",
                        def.name()
                    );
                }
            }
            if enc.summary.iter().any(|&v| v != 0.0) {
                non_zero += 1;
            }
        }
        assert!(non_zero > 20, "expected >20 encoded cards in 1..500");
    }
}
