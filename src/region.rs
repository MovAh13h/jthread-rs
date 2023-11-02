use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use crate::jmutex::LockId;

use lazy_static::lazy_static;
use tord::Tord;

pub (crate) type RegionId = u128;
type RegionManager = RefCell<Vec<(Arc<Region>, Vec<LockId>, LockId)>>;

#[derive(Debug)]
pub struct Region(RegionId);

impl Region {
	pub fn new() -> Arc<Self> {
		// Create a Region and Arcs
		let r = Self(Self::generate_region_id());
		let ar = Arc::new(r);
		let arc = Arc::clone(&ar);

		// Add Region to Region manager
		let regions_cell: RegionManager = REGIONS.take().into();
		let mut regions = regions_cell.borrow_mut();
		regions.push((ar, vec![], LockId::MIN));
		REGIONS.set(regions.to_vec());

		arc
	}

	pub (crate) fn id(&self) -> RegionId {
		self.0
	}

	pub (crate) fn generate_region_id() -> RegionId {
		let mut guard = REGION_ID_COUNTER.lock().unwrap();
		let region_id = *guard;
		*guard += 1;
		drop(guard);

		region_id
	}

	pub (crate) fn generate_lock_id(r: &Arc<Region>) -> LockId {
		let regions_cell: RegionManager = REGIONS.take().into();
		let mut regions = regions_cell.borrow_mut();

		match regions.iter_mut().find(|x| { x.0 == *r }) {
			Some(e) => {
				let lid = e.2;

				e.1.push(lid);
				e.2 += 1;

				lid
			}
			None => panic!("Region Does not Exist")
		}
	}
}

impl PartialEq for Region {
	fn eq(&self, r: &Region) -> bool {
		self.0 == r.0
	}
}

static REGION_ID_COUNTER: Mutex<RegionId> = Mutex::new(0);

thread_local! {
	static REGIONS: RegionManager = RefCell::new(Vec::new());
}

lazy_static! {
	pub (crate) static ref RR: Mutex<Tord<RegionId>> = Mutex::new(Tord::new());
}