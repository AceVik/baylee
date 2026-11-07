//! A seat's view rebuilt from patches and held to the whole view, byte for byte
//! (`--verify --patches`).
//!
//! The receiver here holds only what patches left it. Each patch goes out as
//! the wire would carry it and is read back before it is applied, so what is
//! proved is the whole way from one seat's finished view to the next, and a
//! failure stops the run at the view it happened on.

use std::collections::BTreeMap;

use baylee_core::ids::PlayerId;
use baylee_gamehost::PlayerView;
use baylee_view::{ViewPatch, apply, diff};

/// What each seat's receiver holds, by what it was told.
#[derive(Default)]
pub struct Rebuild {
    /// The view each seat's receiver built.
    held: BTreeMap<PlayerId, PlayerView>,
    /// Patches applied, and so views compared.
    pub views: u64,
    /// Patches that were the whole of a view's change but for `seq`.
    pub empty: u64,
}

impl Rebuild {
    /// Takes the seat's next view, as a patch on what its receiver holds, and
    /// panics unless the receiver ends up with these bytes.
    pub fn view(&mut self, view: &PlayerView) {
        let Some(held) = self.held.get(&view.seat) else {
            self.held.insert(view.seat, view.clone());
            return;
        };
        let patch = diff(held, view);
        self.empty += u64::from(patch.is_empty());
        let wire = serde_json::to_vec(&patch).expect("a patch serializes");
        let read: ViewPatch = serde_json::from_slice(&wire).expect("a patch reads back");
        let rebuilt = match apply(held, &read) {
            Ok(rebuilt) => rebuilt,
            Err(why) => panic!("seat {:?} at seq {}: {why}", view.seat, view.seq),
        };
        let (built, whole) = (
            serde_json::to_vec(&rebuilt).expect("a view serializes"),
            serde_json::to_vec(view).expect("a view serializes"),
        );
        if built != whole {
            let at = built.iter().zip(&whole).take_while(|(a, b)| a == b).count();
            let around = |bytes: &[u8]| {
                String::from_utf8_lossy(&bytes[at.saturating_sub(60)..(at + 60).min(bytes.len())])
                    .into_owned()
            };
            panic!(
                "seat {:?} at seq {}: a rebuilt view differs at byte {at}:\n built {}\n whole {}",
                view.seat,
                view.seq,
                around(&built),
                around(&whole),
            );
        }
        self.views += 1;
        self.held.insert(view.seat, rebuilt);
    }
}
