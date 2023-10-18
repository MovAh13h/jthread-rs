use std::collections::{HashSet, LinkedList};
use std::cmp::Ordering;

use crate::{common::*, Relation};
use crate::jmutex::JMutex;

pub struct Region {
	l_count: LockId,
	locks: HashSet<LockId>,
	relations: LinkedList<Relation>,
}

impl Region {
	pub fn new() -> Self {
		Self {
			l_count: LockId::MIN,
			locks: HashSet::new(),
			relations: LinkedList::new(),
		}
	}

	pub fn new_lock<D>(&mut self, data: D) -> JMutex<D> {
		let lid = self.gen_lock_id();
		self.locks.insert(lid);
		JMutex::internal_new(lid, data)
	}

	#[allow(dead_code)]
	pub (crate) fn check_lock(&self, lid: LockId) -> bool {
		self.locks.contains(&lid)
	}

	#[allow(dead_code)]
	pub (crate) fn check_relation(&self, acq_relation: &Relation) -> Option<bool> {
		for curr_relation in self.relations.iter() {
			if curr_relation == acq_relation {
				let ord = match curr_relation.partial_cmp(&acq_relation) {
					Some(ord) => ord,
					None => return Some(false),
				};

				match ord {
					Ordering::Greater => return Some(true),
					_ => return Some(false),
				}
			}
		}

		return None;
	}

	#[allow(dead_code)]
	pub (crate) fn add_relation(&mut self, rel: Relation) {
		self.relations.push_back(rel);
	}

	fn gen_lock_id(&mut self) -> LockId {
		let result = self.l_count;
		self.l_count += 1;
		return result;
	}
}