#![allow(dead_code, unused)]

// Modules

mod jmutex;
mod region;
mod synchronize;

// Imports

use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::collections::HashMap;

use lazy_static::lazy_static;
use tord::Tord;

use region::Region;

pub use jmutex::*;

// Types

pub (crate) type LockId = u128;
pub (crate) type RegionId = u64;
pub (crate) type ArcRegion = Arc<Region>;
pub (crate) type RegionManager = RefCell<Vec<(ArcRegion, Vec<LockId>, LockId)>>;

// Commons

static REGION_ID_COUNTER: Mutex<RegionId> = Mutex::new(0);

pub (crate) fn generate_new_region_id() -> RegionId {
	let mut guard = REGION_ID_COUNTER.lock().unwrap();

	let result = *guard;

	*guard += 1;

	drop(guard);

	return result;
}

thread_local! {
	pub (crate) static REGIONS: RegionManager = RefCell::new(Vec::new());
}

lazy_static! {
	pub (crate) static ref RR: Mutex<Tord<RegionId>> = Mutex::new(Tord::new());
}
