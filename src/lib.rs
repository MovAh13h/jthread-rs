#![allow(dead_code, unused)]

// ----- Imports -----

use std::fmt::Debug;
use std::fmt::Formatter;
use std::error::Error;
use std::hash::Hasher;
use std::ops::Index;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use std::{fmt, println};
use std::sync::atomic::{AtomicU64, Ordering};
use std::cell::RefCell;

use lazy_static::lazy_static;
use tord::Tord;

// ----- Types -----

type LockId = u64;
type RegionId = u64;

// ----- Globals -----

// Unique Region ID
static REGION_ID: AtomicU64 = AtomicU64::new(0);

// Unique Lock ID
static LOCK_ID: AtomicU64 = AtomicU64::new(0);

// Global Region Ordering
lazy_static! {
	static ref REGION_ORDERING: Mutex<Tord<RegionId>> = Mutex::new(Tord::new());	
}

// Thread local regions
thread_local! {
	static LOCAL_REGIONS: RefCell<LocalRegions> = RefCell::new(LocalRegions::new());
}

// ----- Errors -----

#[derive(Debug, PartialEq)]
pub enum JError {
	IncorrectRegionOrdering,
	UnequalRegions,
	Prelock,
	MutexNotPrelocked
}

impl std::fmt::Display for JError {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self {
			JError::IncorrectRegionOrdering => w.write_str("Incorrect region order"),
			JError::UnequalRegions => w.write_str("Regions are not equal"),
			JError::Prelock => w.write_str("Could not Prelock the mutex"),
			JError::MutexNotPrelocked => w.write_str("Mutex was not prelocked")
		}
	}
}

// ----- Regions -----

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Region(RegionId);

impl Region {
	pub fn new() -> Self {
		Self(Self::generate_region_id())
	}

	fn generate_region_id() -> RegionId {
		let result = REGION_ID.load(Ordering::Relaxed).into();

		REGION_ID.fetch_add(1, Ordering::Relaxed);

		result
	}

	pub (crate) fn id(&self) -> RegionId {
		self.0
	}
}

// ----- JMutex -----

pub struct JMutex<D> {
	inner: Arc<Mutex<D>>,
	lid: LockId,
	region: Region
}

impl<D> JMutex<D> {
	pub fn new(data: D, region: Region) -> Self {
		Self {
			inner: Arc::new(Mutex::new(data)),
			region,
			lid: Self::generate_lock_id()
		}
	}

	pub fn with_default(data: D) -> Self {
		Self {
			inner: Arc::new(Mutex::new(data)),
			region: Region::new(),
			lid: Self::generate_lock_id()
		}
	}

	pub (crate) fn id(&self) -> LockId {
		self.lid
	}

	pub (crate) fn lock(&mut self) -> LockResult<MutexGuard<D>> {
		self.inner.lock()
	}

	pub (crate) fn region(&self) -> Region {
		self.region.clone()
	}

	fn prelock(&mut self) -> Result<(), JError> {
		match self.inner.lock() {
			Ok(g) => Ok(()),
			Err(e) => Err(JError::Prelock)
		}
		
	}

	fn generate_lock_id() -> LockId {
		let result = LOCK_ID.load(Ordering::Relaxed).into();

		LOCK_ID.fetch_add(1, Ordering::Relaxed);

		result
	}
}

impl<D> Clone for JMutex<D> {
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner),
			region: self.region().clone(),
			lid: self.id()
		}
	}
}

impl<D> Debug for JMutex<D> {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		w.debug_struct("JMutex")
			.field("Region", &self.region().id())
			.field("ID", &self.id())
			.finish()
	}
}

// ----- LocalRegion -----

#[derive(Clone, Debug)]
struct ActiveRegion {
	region: Region,
	prelocks: Vec<LockId>,
	active_locks: Vec<LockId>,
}

impl ActiveRegion {
	fn new(region: &Region) -> Self {
		Self {
			region: region.clone(),
			prelocks: vec![],
			active_locks: vec![]
		}
	}

	fn region(&self) -> &Region {
		&self.region
	}

	fn prelocks(&self) -> &Vec<LockId> {
		&self.prelocks
	}

	fn prelocks_mut(&mut self) -> &mut Vec<LockId> {
		&mut self.prelocks
	}

	fn active_locks(&self) -> &Vec<LockId> {
		&self.active_locks
	}

	fn active_locks_mut(&mut self) -> &mut Vec<LockId> {
		&mut self.active_locks
	}

	fn add_active_lock(&mut self, lid: LockId) {
		self.active_locks.push(lid);
	}

	fn add_prelock(&mut self, lid: LockId) {
		self.prelocks.push(lid);
	}
}

impl PartialEq for ActiveRegion {
	fn eq(&self, r: &ActiveRegion) -> bool {
		self.region() == r.region()
	}
}

#[derive(Default)]
struct LocalRegions(Vec<ActiveRegion>);

impl LocalRegions {
	fn new() -> Self {
		let top = Region{ 0: RegionId::MAX };  
		Self(vec![ActiveRegion::new(&top)])
	}

	fn can_lock(&self, region: &Region) -> bool {
		let current_active_region = self.0.last().unwrap();

		if current_active_region.region().id() == region.id() {
			return true;
		}

		let ro = REGION_ORDERING.lock().unwrap();
		// TODO: Check ordering
		let result = ro.check_relation(region.id(), current_active_region.region().id());
		drop(ro);

		match result {
			Some(b) => b,
			None => true
		}
	}

	fn lock_region(&mut self, r: &Region, lid: LockId) -> Result<(), JError> {
		if !self.can_lock(r) {
			return Err(JError::IncorrectRegionOrdering);
		}

		let top_region_id = self.0.last().unwrap().region().id();
		let opt_ar = self.0.iter_mut().find(|x| { x.region() == r });

		match opt_ar {
			// Region exist
			Some(ar) => {
				if !ar.prelocks().contains(&lid) {
					return Err(JError::MutexNotPrelocked);
				}

				if !ar.active_locks().contains(&lid) {
					ar.active_locks_mut().push(lid);
				}
			},

			// Region does not exist
			None => {
				let mut ar = ActiveRegion::new(r);
				ar.add_active_lock(lid);
				self.0.push(ar);
			}
		}

		let mut ro = REGION_ORDERING.lock().unwrap();
		// TODO: Check ordering	
		ro.insert(r.id(), top_region_id);
		drop(ro);

		Ok(())
	}

	fn unlock_region(&mut self, r: &Region, lid: LockId) -> Result<(), JError> {
		let opt_ar = self.0.iter_mut().find(|x| { x.region() == r });

		let mut removal = false;

		match opt_ar {
			Some(ar) => {
				match ar.active_locks().iter().position(|x| { *x == lid }) {
					Some(i) => {
						ar.active_locks_mut().swap_remove(i);

						if ar.active_locks().len() == 0 {
							removal = true;
						}
					}
					_ => {
					}
				}
			}

			None => {}
		}

		if removal {
			match self.0.iter_mut().position(|x| { x.region().id() == r.id() }) {
				Some(index) => {
					self.0.swap_remove(index);
				}
				_ => {}
			}
			
		}

		Ok(())
	}
}

#[macro_export]
macro_rules! sync {
    ([$mutex1:expr], $closure:expr) => {
    	sync1($mutex1, $closure)
    };

    ([$mutex1:expr, $mutex2:expr], $closure:expr) => {
    	sync2($mutex1, $mutex2, $closure)
    };

    ([$mutex1:expr, $mutex2:expr, $mutex3:expr], $closure:expr) => {
    	sync3($mutex1, $mutex2, $mutex3, $closure)
    };
}

fn sync1<D1, C, R>(mut m1: JMutex<D1>, c: C) -> Result<R, JError>
where
	C: FnOnce(MutexGuard<D1>) -> R
{
	let tid = std::thread::current().id();

	// Region Check
	// Additionally, lock the region ie. push the region on the stack
	let rm1 = m1.region();
	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.lock_region(&rm1, m1.id());
	
	LOCAL_REGIONS.set(local_regions);

	if lock_result.is_err() {
		return Err(lock_result.unwrap_err());
	}

	println!("{:?} locked {:?}", tid, m1);
	// Acquire first lock
	let guard = m1.lock().unwrap();

	// Call the closure
	let result = c(guard);

	// Unlock the region
	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.unlock_region(&rm1, m1.id());
	LOCAL_REGIONS.set(local_regions);

	Ok(result)
}

fn sync2<D1, D2, C, R>(mut m1: JMutex<D1>, mut m2: JMutex<D2>, c: C)
	-> Result<R, JError>
where
	C: FnOnce(MutexGuard<D1>, JMutex<D2>) -> R
{
	// Region Check
	// Additionally, lock the region ie. push the region on the stack
	let rm1 = m1.region();
	let rm2 = m2.region();

	if rm1 != rm2 {
		return Err(JError::UnequalRegions);
	}

	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.lock_region(&rm1, rm1.id());

	LOCAL_REGIONS.set(local_regions);

	if lock_result.is_err() {
		return Err(lock_result.unwrap_err());
	}

	// Prelocking
	let pm2 = m2.prelock();
	if pm2.is_err() {
		return Err(pm2.unwrap_err());
	}

	// Acquire first lock
	let guard = m1.lock().unwrap();

	// Call the closure
	let result = c(guard, m2);

	// Unlock the region
	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.unlock_region(&rm1, m1.id());
	LOCAL_REGIONS.set(local_regions);


	Ok(result)
}

fn sync3<D1, D2, D3, C, R>(mut m1: JMutex<D1>, mut m2: JMutex<D2>, mut m3: JMutex<D3>, c: C)
	-> Result<R, JError>
where
	C: FnOnce(MutexGuard<D1>, JMutex<D2>, JMutex<D3>) -> R
{
	// Region Check
	// Additionally, lock the region ie. push the region on the stack
	let rm1 = m1.region();
	let rm2 = m2.region();
	let rm3 = m3.region();

	if rm1 != rm2 || rm1 != rm3 {
		return Err(JError::UnequalRegions);
	}

	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.lock_region(&rm1, rm1.id());

	LOCAL_REGIONS.set(local_regions);

	if lock_result.is_err() {
		return Err(lock_result.unwrap_err());
	}

	// Prelocking
	let pm2 = m2.prelock();
	if pm2.is_err() {
		return Err(pm2.unwrap_err());
	}

	let pm3 = m3.prelock();
	if pm3.is_err() {
		return Err(pm3.unwrap_err());
	}

	// Acquire first lock
	let guard = m1.lock().unwrap();

	// Call the closure
	let result = c(guard, m2, m3);

	// Unlock the region
	let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
	let mut local_regions = lr.take();
	let lock_result = local_regions.unlock_region(&rm1, m1.id());
	LOCAL_REGIONS.set(local_regions);


	Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // Define a Fork as a simple integer
    struct Fork(u32);

    // Define a Philosopher with two forks
    struct Philosopher {
        left_fork: JMutex<Fork>,
        right_fork: JMutex<Fork>,
        name: String,
    }

    impl Philosopher {
        // Create a new philosopher with given forks and name
        fn new(left_fork: JMutex<Fork>, right_fork: JMutex<Fork>, name: String) -> Self {
            Self {
                left_fork,
                right_fork,
                name,
            }
        }

        // Function for philosopher to eat
        fn eat(&self) {
            println!("{} is thinking.", self.name);

            sync!([self.left_fork.clone(), self.right_fork.clone()], |left_fork_guard, r| {
            	sync!([r], |g| {
            		println!("{} is eating.", self.name);

	                // Simulate eating
	                thread::sleep(std::time::Duration::from_secs(1));

	                println!("{} has finished eating.", self.name);
            	});                
            }).expect("Failed to acquire forks");

            println!("{} is thinking again.", self.name);
        }
    }

    // Works single
    #[test]
    fn dining_philosophers() {
    	let r = Region::new();

        // Initialize forks
        let forks = vec![
            JMutex::new(Fork(1), r.clone()),
            JMutex::new(Fork(2), r.clone()),
            JMutex::new(Fork(3), r.clone()),
            JMutex::new(Fork(4), r.clone()),
        ];

        // Initialize dining_philosophers
        let philosophers = vec![
            Philosopher::new(forks[0].clone(), forks[1].clone(), "Philosopher 1".to_string()),
            Philosopher::new(forks[1].clone(), forks[2].clone(), "Philosopher 2".to_string()),
            Philosopher::new(forks[2].clone(), forks[3].clone(), "Philosopher 3".to_string()),
            Philosopher::new(forks[3].clone(), forks[0].clone(), "Philosopher 4".to_string()),
        ];

        // Create threads for each philosopher to eat
        let handles: Vec<_> = philosophers.into_iter().map(|philosopher| {
            thread::spawn(move || {
                philosopher.eat();
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Philosopher thread panicked");
        }
    }

	// Works single
	#[test]
	fn concurrent_access() {
		let r = Region::new();
	    let shared_data = JMutex::new(vec![1, 2, 3], r);
	    let mut handles = vec![];

	    for _ in 0..10 {
	        let data_clone = shared_data.clone();
	        handles.push(thread::spawn(move || {
	            let result = sync!([data_clone], |guard| {
	                guard.iter().sum::<i32>()
	            });
	            assert_eq!(result.expect("Failed to lock"), 6);
	        }));
	    }

	    for handle in handles {
	        handle.join().expect("Thread panicked");
	    }
	}

	// Works single
	#[test]
	fn same_regions() {
		let r = Region::new();
	    let mutex1 = JMutex::new(1, r.clone());
	    let mutex2 = JMutex::new(2, r.clone());

	    let m1_clone = mutex1.clone();
	    let m2_clone = mutex2.clone();

	    let handle1 = thread::spawn(move || {
	        sync!([m1_clone, m2_clone], |guard1, m2| {
	            sync!([m2], |m2g| {
	            	// Use locks
	            });
	        }).expect("Failed to acquire locks");
	    });

	    let handle2 = thread::spawn(move || {
	        sync!([mutex2, mutex1], |guard2, m1| {
	            sync!([m1], |m1g| {
	            	// Use locks
	            });
	        }).expect("Failed to acquire locks");
	    });

	    handle1.join().expect("Thread 1 panicked");
	    handle2.join().expect("Thread 2 panicked");
	}

	#[test]
	fn different_regions_single_threaded() {
		let r1 = Region::new();
		let r2 = Region::new();

		let m1 = JMutex::new(1, r1);
		let m2 = JMutex::new(2, r2);
		let m1c = m1.clone();
		let m2c = m2.clone();

		let a = sync!([m1c], |g1| {
			sync!([m2c], |g2| {
				// Use locks
			});
		});

		let b = sync!([m2], |g1| {
			sync!([m1], |g2| {
				// Use locks
			});
		});

		if a.is_ok() {
			assert_eq!(b.unwrap_err(), JError::IncorrectRegionOrdering);
		} else if b.is_ok() {
			assert_eq!(a.unwrap_err(), JError::IncorrectRegionOrdering);
		}
	}

	#[test]
	fn different_regions_multi_threaded() {
		let r1 = Region::new();
		let r2 = Region::new();

		let m1 = JMutex::new(1, r1);
		let m2 = JMutex::new(2, r2);

		let m1c = m1.clone();
		let m2c = m2.clone();

		
		let h1 = thread::spawn(move || {
			sync!([m1c], |g1| {
				println!("1. {:?}", std::thread::current().id());
				sync!([m2c], |g2| {
					println!("2. {:?}", std::thread::current().id());
					// Use locks
				});
			})
		});

		let h2 = thread::spawn(move || {
			sync!([m2], |g2| {
				println!("2. {:?}", std::thread::current().id());
				sync!([m1], |g1| {
					println!("1. {:?}", std::thread::current().id());
					// Use locks
				});
			})
		});

		let a = h1.join().unwrap();
		let b = h2.join().unwrap();
 

		if a.is_ok() {
			assert_eq!(b.is_err(), true);
		} else if a.is_err() {
			assert_eq!(b.is_ok(), true);
		}
	}
}
