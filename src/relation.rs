use std::cmp::Ordering;

use crate::common::*;

#[derive(Debug, Clone, Copy)]
pub struct Relation {
	x: LockId,
	y: LockId,
}

impl Relation {
	pub fn new(x: LockId, y: LockId) -> Self {
		Self { x, y }
	}
}

impl PartialEq for Relation {
	fn eq(&self, r: &Relation) -> bool {
		(self.x == r.x && self.y == r.y) || (self.x == r.y && self.y == r.x)
	}
}

impl PartialOrd for Relation {
	fn partial_cmp(&self, r: &Self) -> Option<Ordering> {
		if self.x == r.x && self.y == r.y {
			return Some(Ordering::Equal);
		} else if self.x == r.y && self.y == r.x {
			return Some(Ordering::Less);
		}

		None
	}
}

#[cfg(test)]
mod relation {
	use super::*;

	#[test]
	fn eq() {
		let r1 = Relation::new(1, 2);
		let r2 = Relation::new(1, 2);

		assert_eq!(r1, r2);
	}

	#[test]
	fn ne() {
		let r1 = Relation::new(1, 2);
		let r2 = Relation::new(1, 3);

		assert_ne!(r1, r2);
	}

	#[test]
	fn partial_cmp() {
		let r1 = Relation::new(1, 2);
		let r2 = Relation::new(1, 2);

		assert_eq!(r1.partial_cmp(&r2), Some(Ordering::Equal));

		let r3 = Relation::new(1, 3);
		
		assert_eq!(r1.partial_cmp(&r3), None);	
		
		let r4 = Relation::new(2, 1);

		assert_eq!(r1.partial_cmp(&r4), Some(Ordering::Less));	
	}
}