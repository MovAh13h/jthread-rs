use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;
use std::sync::{Arc, Mutex, MutexGuard, LockResult};

use crate::Region;
use crate::region::RegionId;

pub (crate) type LockId = u128;

pub struct JMutex<D> {
	inner: Arc<Mutex<D>>,
	region: Arc<Region>,
	lid: LockId
}

impl<D> JMutex<D> {
	pub fn new(data: D, or: Option<Arc<Region>>) -> Self {
		let (arc_rgn, lid) = match or {
			Some(r) => {
				let lid = Region::generate_lock_id(&r);
				(r, lid)
			},

			None => (Region::new(), LockId::MIN)
		};

		Self {
			inner: Arc::new(Mutex::new(data)),
			lid,
			region: arc_rgn,
		}
	}

	pub fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.inner.lock()
	}

	#[allow(dead_code)]
	pub (crate) fn region_id(&self) -> RegionId {
		self.region.id()
	}
}

impl<D> Debug for JMutex<D> {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), Error> {
		w.debug_struct("JMutex")
			.field("LockId", &self.lid)
			.field("RegionId", &self.region)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use std::thread;

	use super::*;

	#[test]
	fn lock() {
		let mut m = JMutex::new(String::from("1"), None);
		println!("{:?}", m);

		let guard = m.lock().unwrap();
		println!("{}", guard);
	}

	#[test]
	fn debug() {
		let m1 = JMutex::new(String::from("1"), None);
		println!("{:?}", m1);

		thread::spawn(move || {
			let m2 = JMutex::new(String::from("2"), None);
			println!("{:?}", m2);			
		});
	}
}
