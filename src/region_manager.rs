use std::thread::{ThreadId, current};
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::{JMutex, LockId};
use crate::Region;
use crate::Relation;


use lazy_static::lazy_static;

pub struct RegionManager {
	regions: HashMap<ThreadId, Region>
}

impl RegionManager {
	pub fn new() -> Self {
		Self {
			regions: HashMap::new(),
		}
	}

	pub fn new_lock<D>(&mut self, data: D) -> JMutex<D> {
		let tid = current().id();

		match self.regions.entry(tid) {
			Entry::Occupied(mut r) => {
				let region = r.get_mut();
				region.new_lock(data)
			}

			Entry::Vacant(e) => {
				let region = e.insert(Region::new());
				region.new_lock(data)
			}
		}
	}

	pub (crate) fn check_region(&self, lid: LockId) -> bool {
		let tid = current().id();

		match self.regions.get(&tid) {
			Some(r) => {
				r.check_lock(lid)
			}

			_ => false
		}
	}

	#[allow(dead_code)]
	pub (crate) fn handle_relation(&mut self, a: LockId, b: LockId) {
		let tid = current().id();

		let rel = Relation::new(a, b);

		match self.regions.entry(tid) {
			Entry::Occupied(mut r) => {
				let region = r.get_mut();
				match region.check_relation(&rel) {
					// Some relation exists
					// check if the relation is valid
					Some(relation) => {
						if !relation {
							panic!("Lock acquisition order is incorrect");
						}
					},

					// no relation exists
					// add it
					None => {
						let region = r.get_mut();
						region.add_relation(rel);
					}
				}
			}
			Entry::Vacant(e) => {
				let region = e.insert(Region::new());
				region.add_relation(rel);
			}
		}
	}
}

// impl Deref for RegionManager {
// 	type Target = Region;

// 	fn deref(&self) -> &<Self as Deref>::Target {
// 		let tid = std::thread::current().id();
// 		&self.regions[&tid]
// 	}
// }

// impl DerefMut for RegionManager {
// 	fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {		
// 		let tid = std::thread::current().id();
		
// 		match self.regions.entry(tid) {
// 			Entry::Occupied(r) => {
// 				r.into_mut()
// 			}

// 			Entry::Vacant(e) => {
// 				e.insert(Region::new())
// 			}
// 		}
// 	}
// }

lazy_static! {
    pub static ref REGION_MANAGER: Mutex<RegionManager> = {
    	Mutex::new(RegionManager::new())
    };
}