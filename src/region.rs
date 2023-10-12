use std::collections::HashSet;

use crate::common::*;

pub struct Region {
	l_count: LockId,
	locks: HashSet<LockId>,
}

impl Region {
	pub fn new() -> Self {
		Self {
			l_count: LockId::MIN,
			locks: HashSet::new(),
		}
	}

	fn gen_lock_id(&mut self) -> LockId {
		let result = self.l_count;
		self.l_count += 1;
		return result;
	}
}
