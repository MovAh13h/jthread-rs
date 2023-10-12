use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use crate::region_manager::rm::REGION_MANAGER;

pub struct JMutex<D> {
	inner: Arc<Mutex<D>>
}

impl<D> JMutex<D> {
	pub fn new(data: D) -> Self {
		REGION_MANAGER.new_lock(data)
	}

	pub fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.inner.lock()
	}
}

impl<D> Clone for JMutex<D> {
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner)
		}
	}
}

impl<D> From<Mutex<D>> for JMutex<D> {
	fn from(lock: Mutex<D>) -> Self {
		Self {
			inner: Arc::new(lock)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lock() {
		let mut m = JMutex::new(String::from("Lorem Ipsum"));

		let g = m.lock().unwrap();

		println!("{:?}", g);

		drop(g);
	}
}