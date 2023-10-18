use std::sync::PoisonError;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use crate::LockId;
use crate::REGION_MANAGER;

pub struct JMutex<D> {
	lid: LockId,
	inner: Arc<Mutex<D>>
}
	
impl<D> JMutex<D> {
	pub fn new(data: D) -> Self {
		let mut guard = REGION_MANAGER.lock().unwrap();
		guard.new_lock(data)
	}

	pub (crate) fn internal_new(lid: LockId, data: D) -> Self {
		Self {
			lid,
			inner: Arc::new(Mutex::new(data)),
		}
	}

	#[allow(dead_code)]
	pub (crate) fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		let guard = REGION_MANAGER.lock().unwrap();

		let res = guard.check_region(self.lid);

		if !res {
			return Err(PoisonError::new(self.inner.lock().unwrap()))
		}

		self.inner.lock()
	}

	#[allow(dead_code)]
	pub (crate) fn lock_id(&self) -> LockId {
		self.lid
	}
}

impl<D> Clone for JMutex<D> {
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner),
			lid: self.lid,
		}
	}
}

// TODO: To add LockId to the JMutex, connect it to a region
// impl<D> From<Mutex<D>> for JMutex<D> {
// 	fn from(lock: Mutex<D>) -> Self {
// 		let data = lock.into();
// 		JMutex::new(data)
// 	}
// }

#[cfg(test)]
mod jmutex {
	use super::*;

	#[test]
	fn lock() {
		let mut m = JMutex::new(String::from("Lorem Ipsum"));

		let g = m.lock().unwrap();

		println!("{:?}", g);

		drop(g);
	}
}