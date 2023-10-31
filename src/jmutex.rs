use std::fmt::{Debug, Error, Formatter};
use std::sync::{Arc, Mutex, LockResult, MutexGuard};

use crate::*;

pub struct JMutex<D> {
	lock: Arc<Mutex<D>>,
	lid: LockId,
	region: ArcRegion,
}

impl<D> JMutex<D> {
	pub fn new(data: D) -> Self {
		let regions_cell: RegionManager = REGIONS.take().into();

		let mut regions = regions_cell.borrow_mut();

		let (arc_rgn, lid) = match regions.last_mut() {
			Some((ar, lids, lockid)) => {
				let arc = Arc::clone(&ar);
				let lid = *lockid;

				lids.push(*lockid);

				*lockid += 1;
				
				(arc, lid)
			}

			None => {
				let ar = Arc::new(Region::new());
				let arc = Arc::clone(&ar);

				regions.push((ar, vec![LockId::MIN], LockId::MIN + 1));

				(arc, LockId::MIN)
			}
		};

		REGIONS.set(regions.to_vec());
		
		Self {
			lock: Arc::new(Mutex::new(data)),
			lid,
			region: arc_rgn,
		}
	}

	pub (crate) fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.lock.lock()
	}

	pub (crate) fn region_id(&self) -> RegionId {
		self.region.region_id()
	}
}

impl<D> Debug for JMutex<D> {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), Error> {
		w.debug_struct("JMutex")
			.field("LockId", &self.lid)
			.field("RegionId", &self.region.as_ref().region_id())
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use std::thread;

	use super::*;

	#[test]
	fn lock() {
		let mut m = JMutex::new(String::from("1"));
		println!("{:?}", m);

		let guard = m.lock().unwrap();
		println!("{}", guard);
	}

	#[test]
	fn region_id() {
		let mut m1 = JMutex::new(String::from("1"));
		println!("{:?}", m1);

		thread::spawn(move || {
			let mut m2 = JMutex::new(String::from("2"));
			println!("{:?}", m2);			
		});
	}
}
