use crate::RegionId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;
use std::sync::{Mutex, LockResult, MutexGuard, Arc};

use crate::Region;


pub (crate) type LockId = u64;

pub struct JMutex<D> {
	inner: Mutex<D>,
	lid: LockId,
	rgn: RegionId
}

impl<D> JMutex<D> {
	pub fn new(data: D, or: Option<Region>) -> Self {
		let (rgn, lid): (i32, i32) = match or {
			Some(r) => {
				
				todo!()
			}

			None => {
				
				todo!()
			}
		};


		todo!()
	}

	pub fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.inner.lock()
	}

	pub (crate) fn generate_lock_id() -> LockId {
		let result = LOCK_ID.load(Ordering::SeqCst).into();

		LOCK_ID.fetch_add(1, Ordering::SeqCst);

		result
	}
}

impl<D> fmt::Debug for JMutex<D> {
	fn fmt(&self, w: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		w.debug_struct("JMutex")
			.field("LockId", &self.lid)
			.field("RegionId", &self.rgn)
			.finish()
	}
}

// Unique Region ID
static LOCK_ID: AtomicU64 = AtomicU64::new(0);