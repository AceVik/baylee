//! `/proc/pressure/*`: how much time tasks lost waiting for a resource.
//! "some" means at least one task stalled, "full" means all of them did.

use serde::Serialize;

/// The 10, 60 and 300 second averages, in percent of wall time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Pressure {
    pub some: [f32; 3],
    pub full: [f32; 3],
}

pub(crate) fn parse(text: &str) -> Option<Pressure> {
    let mut pressure = None::<Pressure>;
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let kind = fields.next()?;
        let mut averages = [0.0; 3];
        for field in fields {
            let Some((key, value)) = field.split_once('=') else {
                continue;
            };
            let at = match key {
                "avg10" => 0,
                "avg60" => 1,
                "avg300" => 2,
                _ => continue,
            };
            averages[at] = value.parse().ok()?;
        }
        let entry = pressure.get_or_insert_default();
        match kind {
            "some" => entry.some = averages,
            "full" => entry.full = averages,
            _ => {}
        }
    }
    pressure
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_lines() {
        let text = "some avg10=10.36 avg60=11.58 avg300=6.10 total=90540554\n\
                    full avg10=10.36 avg60=11.57 avg300=6.04 total=86879339\n";
        let pressure = parse(text).unwrap();
        assert!((pressure.some[0] - 10.36).abs() < 0.001);
        assert!((pressure.full[2] - 6.04).abs() < 0.001);
    }

    #[test]
    fn a_kernel_without_a_full_line_leaves_it_zero() {
        let pressure = parse("some avg10=1.00 avg60=2.00 avg300=3.00 total=1\n").unwrap();
        assert!((pressure.some[2] - 3.0).abs() < 0.001);
        assert_eq!(pressure.full, [0.0; 3]);
    }

    #[test]
    fn an_absent_or_broken_file_is_unavailable_not_zero() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("some avg10=x\n"), None);
    }
}
