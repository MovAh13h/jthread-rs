use crate::*;

pub (crate) struct Region(RegionId);

impl Region {
	pub (crate) fn new() -> Self {
		Self(generate_new_region_id())
	}

	pub (crate) fn region_id(&self) -> RegionId {
		self.0
	}
}
