use std::thread::{ThreadId};
use std::ops::{Deref, DerefMut};
use std::collections::HashMap;

use crate::JMutex;
use crate::region::Region;

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

	pub fn new_lock<D>(&mut self, _data: D) -> JMutex<D> {
		todo!();
	}
}

impl Deref for RegionManager {
	type Target = Region;

	fn deref(&self) -> &<Self as Deref>::Target {
		todo!();
	}
}

impl DerefMut for RegionManager {
	fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {		
		todo!();
	}
}

lazy_static! {
    pub static ref REGION_MANAGER: RegionManager = {
    	RegionManager::new()
    };
}

pub mod rm {
	pub use crate::region_manager::REGION_MANAGER;
}