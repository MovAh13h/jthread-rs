use std::sync::{Arc, LockResult, Mutex, MutexGuard};

pub struct JMutex<D> {
	inner: Arc<Mutex<D>>
}

impl<D> JMutex<D> {
	pub fn new(data: D) -> Self {
		Self { inner: Arc::new(Mutex::new(data)) }
	}

	pub fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.inner.lock()
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