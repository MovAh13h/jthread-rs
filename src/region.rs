// Imports

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread_local;

use crate::LockId;

// Types

pub (crate) type RegionId = u128;
pub (crate) type RegionManager = RefCell<Vec<(Region, Vec<LockId>)>>;

#[derive(Debug, Clone)]
pub struct Region(RegionId);

impl Region {
	pub fn new() -> Self {
		let result = Self(REGION_ID.load(Ordering::SeqCst).into());

		// Increment the REGION_ID
		REGION_ID.fetch_add(1, Ordering::SeqCst);

		result
	}

	pub (crate) fn id(&self) -> RegionId {
		self.0
	}
}

// Unique Region ID
static REGION_ID: AtomicU64 = AtomicU64::new(0);

thread_local! {
	static REGIONS: RegionManager = RefCell::new(Vec::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use lazy_static::lazy_static;

    // Since tests are run in parallel, we use a Mutex to ensure only one test
    // interacts with REGION_ID at a time. This is for testing purposes only.
    lazy_static! {
        static ref REGION_ID_LOCK: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn new_region_has_unique_id() {
        let _guard = REGION_ID_LOCK.lock().unwrap(); // Lock for the duration of the test

        let before_id = REGION_ID.load(Ordering::SeqCst);
        let region = Region::new();
        let after_id = REGION_ID.load(Ordering::SeqCst);

        // Ensure the ID was incremented exactly once
        assert_eq!(before_id + 1, after_id);
        // Ensure the region's ID matches what was loaded
        assert_eq!(region.id(), before_id.into());
    }

    #[test]
    fn id_method_returns_correct_value() {
        let _guard = REGION_ID_LOCK.lock().unwrap(); // Lock for the duration of the test

        let initial_id = REGION_ID.load(Ordering::SeqCst);
        let region = Region::new();
        let expected_id: RegionId = initial_id.into(); // Convert to RegionId type

        // Ensure the region's ID is the expected value
        assert_eq!(region.id(), expected_id);
    }

    #[test]
    fn region_ids_are_incremental() {
        let _guard = REGION_ID_LOCK.lock().unwrap(); // Lock for the duration of the test

        let region1 = Region::new();
        let region2 = Region::new();

        // Ensure each region ID is incrementing correctly
        assert_eq!(region2.id(), region1.id() + 1);
    }
}
