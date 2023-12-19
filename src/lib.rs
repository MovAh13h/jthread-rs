mod directed_graph;

/// # JThread
///
/// This library provides a set of tools for managing directed graphs and mutexes in a multi-threaded environment.
/// It is particularly useful for scenarios requiring complex locking strategies and region-based resource management.
///
/// ## Features
/// - `Region` and `JMutex` for fine-grained control over mutex locks.
/// - Directed graph implementation for advanced resource ordering.
/// - Custom error types for handling common mutex and region-related errors.
///
/// ## Examples
/// Basic usage of the library can be found in the `tests` module. It demonstrates how to create regions, mutexes, and handle errors.
// ----- Imports -----
use std::fmt::Debug;
use std::fmt::Formatter;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use directed_graph::DirectedGraph;
use lazy_static::lazy_static;

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
    static ref REGION_ORDERING: Mutex<DirectedGraph<RegionId>> = Mutex::new(DirectedGraph::new());
}

// Thread local regions
thread_local! {
    static LOCAL_REGIONS: RefCell<LocalRegions> = RefCell::new(LocalRegions::new());
}

// ----- Errors -----

/// The JThread Error type
#[derive(Debug, PartialEq)]
pub enum JError {
    /// Indicates an invalid ordering of regions, which can lead to deadlocks.
    IncorrectRegionOrdering,
    /// Used when attempted operations span across different regions.
    UnequalRegions,
    /// Occurs when a mutex guard panics, poisoning the mutex.
    PoisonedMutex,
    /// Raised when a mutex is expected to be pre-locked but isn't.
    MutexNotPrelocked,
}

impl std::fmt::Display for JError {
    fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            JError::IncorrectRegionOrdering => w.write_str("Incorrect region order"),
            JError::UnequalRegions => w.write_str("Regions are not equal"),
            JError::PoisonedMutex => w.write_str("Mutex is poisoned"),
            JError::MutexNotPrelocked => w.write_str("Mutex was not prelocked"),
        }
    }
}

// ----- Regions -----

/// Represents a region in the context of JThread
///
/// A `Region` is a logical grouping of `JMutex`, intended to manage lock ordering and prevent deadlocks.
///
/// # Examples
/// ```
/// use jthread::Region;
///
/// let region = Region::new();
/// println!("Created a new region with ID: {}", region.id());
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Region(RegionId);

impl Region {
    /// Create a new region
    pub fn new() -> Self {
        Self(Self::generate_region_id())
    }

    fn generate_region_id() -> RegionId {
        let result = REGION_ID.load(Ordering::Relaxed).into();

        REGION_ID.fetch_add(1, Ordering::Relaxed);

        result
    }

    /// Returns the ID of the region
    pub fn id(&self) -> RegionId {
        self.0
    }
}

// ----- JMutex -----

/// A region-aware mutex wrapper.
///
/// `JMutex` wraps a standard mutex, providing region-based locking
///
/// # Generics
/// - `D`: The type of data protected by the mutex.
///
/// # Examples
/// ```
/// use jthread::{JMutex, Region};
///
/// let region = Region::new();
/// let data = 5;
/// let mutex = JMutex::new(data, region);
/// ```
pub struct JMutex<D> {
    inner: Arc<Mutex<D>>,
    lid: LockId,
    region: Region,
}

impl<D> JMutex<D> {
    /// Constructs a new `JMutex` with the provided data and region.
    ///
    /// This method creates a new `JMutex` instance which includes an atomic lock ID, a region,
    /// and the data to be protected by the mutex.
    ///
    /// # Arguments
    /// * `data` - The data to be protected by the mutex.
    /// * `region` - The region associated with this mutex, used for ordering and deadlock prevention.
    ///
    /// # Examples
    /// ```
    /// use jthread::{JMutex, Region};
    ///
    /// let region = Region::new();
    /// let mutex = JMutex::new(5, region); // Protecting an integer
    /// ```
    pub fn new(data: D, region: Region) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            region,
            lid: Self::generate_lock_id(),
        }
    }

    /// Constructs a new `JMutex` with the provided data and a default region.
    ///
    /// Similar to `new`, but instead of requiring a region to be passed,
    /// this method automatically associates the mutex with a new, unique region.
    ///
    /// # Arguments
    /// * `data` - The data to be protected by the mutex.
    ///
    /// # Examples
    /// ```
    /// use jthread::JMutex;
    ///
    /// let mutex = JMutex::with_default(5); // Protecting an integer with a default region
    /// ```
    pub fn with_default(data: D) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            region: Region::new(),
            lid: Self::generate_lock_id(),
        }
    }

    pub(crate) fn id(&self) -> LockId {
        self.lid
    }

    pub(crate) fn lock(&mut self) -> LockResult<MutexGuard<D>> {
        self.inner.lock()
    }

    pub(crate) fn region(&self) -> Region {
        self.region.clone()
    }

    fn prelock(&mut self) -> Result<(), JError> {
        match self.inner.lock() {
            Ok(_) => Ok(()),
            Err(_) => Err(JError::PoisonedMutex),
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
            lid: self.id().clone(),
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
            active_locks: vec![],
        }
    }

    fn region(&self) -> &Region {
        &self.region
    }

    fn prelocks(&self) -> &Vec<LockId> {
        &self.prelocks
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
        let top = Region { 0: RegionId::MAX };
        Self(vec![ActiveRegion::new(&top)])
    }

    fn lock_region(&mut self, r: &Region, lid: LockId, prelocks: &[LockId]) -> Result<(), JError> {
        let top_region = self.0.last().unwrap();
        let top_region_id = top_region.region().id();

        let mut ro = REGION_ORDERING.lock().unwrap();

        if top_region_id == r.id() {
            return Ok(());
        }

        match self.0.iter_mut().find(|x| x.region() == r) {
            // Region exist
            Some(ar) => {
                if !ar.prelocks().contains(&lid) {
                    return Err(JError::MutexNotPrelocked);
                }

                if !ar.active_locks().contains(&lid) {
                    ar.active_locks_mut().push(lid);
                }
            }

            // Region does not exist
            None => {
                let mut ar = ActiveRegion::new(r);
                ar.add_active_lock(lid);

                for pr in prelocks {
                    ar.add_prelock(pr.clone());
                }

                self.0.push(ar);
            }
        }

        let result = ro.add_edge_with_check(top_region_id, r.id());

        if result.is_ok() {
            return Ok(());
        } else {
            return Err(JError::IncorrectRegionOrdering);
        }
    }

    fn unlock_region(&mut self, r: &Region, lid: LockId) -> Result<(), JError> {
        let opt_ar = self.0.iter_mut().find(|x| x.region() == r);

        let mut removal = false;

        match opt_ar {
            Some(ar) => match ar.active_locks().iter().position(|x| *x == lid) {
                Some(i) => {
                    ar.active_locks_mut().swap_remove(i);

                    if ar.active_locks().len() == 0 {
                        removal = true;
                    }
                }
                _ => {}
            },

            None => {}
        }

        if removal {
            match self.0.iter_mut().position(|x| x.region().id() == r.id()) {
                Some(index) => {
                    self.0.swap_remove(index);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// The `sync` macro is a convenient way to acquire locks on one or more `JMutex` instances within a single closure.
///
/// This macro provides a simplified interface to manage mutex locks, especially useful in multi-threaded contexts.
/// It abstracts the complexity of locking logic, ensuring that locks are acquired in a safe and deadlock-free manner.
///
/// # Variants
/// - `sync!([mutex1], closure)`: Locks a single `JMutex`.
/// - `sync!([mutex1, mutex2], closure)`: Locks two `JMutex` instances.
/// - `sync!([mutex1, mutex2, mutex3], closure)`: Locks three `JMutex` instances.
///
///
/// # Examples
/// Basic usage with a single mutex:
/// ```
/// use jthread::*;
///
/// let data_mutex = JMutex::with_default(5); // Protecting an integer
/// let result = sync!([data_mutex], |data_guard| {
///     *data_guard += 1; // Modify the data
///     *data_guard
/// });
/// // result now holds the modified value
/// ```
///
/// Using three mutexes:
/// ```
/// use jthread::*;
///
/// let mutex1 = JMutex::with_default(5);
/// let mutex2 = JMutex::with_default("Hello");
/// let mutex3 = JMutex::with_default(vec![1, 2, 3]);
///
/// sync!([mutex1, mutex2, mutex3], |data_guard1, data_guard2, data_guard3| {
///     *data_guard1 += 1; // Modify the first data
///     println!("{}, world!", *data_guard2); // Use the second data
///     data_guard3.push(4); // Modify the vector
/// });
/// ```
///
/// # Notes
/// - The macro is designed to ensure that locks are acquired in a consistent order, preventing deadlocks.
/// - Make sure that the closures do not panic, as this might lead to lock poisoning.
/// - The closure should ideally be short-lived to avoid holding the locks for an extended period.
///
/// For more complex examples, refer to the `tests` module in the library.
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

pub fn sync1<D1, C, R>(mut m1: JMutex<D1>, c: C) -> Result<R, JError>
where
    C: FnOnce(MutexGuard<D1>) -> R,
{
    // Region Check
    // Additionally, lock the region ie. push the region on the stack
    let rm1 = m1.region();
    let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
    let mut local_regions = lr.take();

    let lock_region_result = local_regions.lock_region(&rm1, m1.id(), &[]);
    LOCAL_REGIONS.set(local_regions);

    match lock_region_result {
        Err(e) => return Err(e),
        Ok(_) => {}
    }

    // Acquire first lock
    let guard = m1.lock().unwrap();

    // Call the closure
    let result = c(guard);

    // Unlock the region
    let lr: RefCell<LocalRegions> = LOCAL_REGIONS.take().into();
    let mut local_regions = lr.take();
    let _ = local_regions.unlock_region(&rm1, m1.id());
    LOCAL_REGIONS.set(local_regions);

    Ok(result)
}

pub fn sync2<D1, D2, C, R>(mut m1: JMutex<D1>, mut m2: JMutex<D2>, c: C) -> Result<R, JError>
where
    C: FnOnce(MutexGuard<D1>, JMutex<D2>) -> R,
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
    let lock_result = local_regions.lock_region(&rm1, m1.id(), &[m2.id()]);
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
    let _ = local_regions.unlock_region(&rm1, m1.id());
    LOCAL_REGIONS.set(local_regions);

    Ok(result)
}

pub fn sync3<D1, D2, D3, C, R>(
    mut m1: JMutex<D1>,
    mut m2: JMutex<D2>,
    mut m3: JMutex<D3>,
    c: C,
) -> Result<R, JError>
where
    C: FnOnce(MutexGuard<D1>, JMutex<D2>, JMutex<D3>) -> R,
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
    let lock_result = local_regions.lock_region(&rm1, m1.id(), &[m2.id(), m3.id()]);

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
    let _ = local_regions.unlock_region(&rm1, m1.id());
    LOCAL_REGIONS.set(local_regions);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        assert_eq,
        thread::{self, ThreadId},
        time::Duration,
    };

    fn tid() -> ThreadId {
        std::thread::current().id()
    }

    // Define a Fork as a simple integer
    #[derive(Debug)]
    struct Fork(u32);

    // Define a Philosopher with two forks
    struct Philosopher {
        left_fork: JMutex<Fork>,
        right_fork: JMutex<Fork>,
    }

    impl Philosopher {
        // Create a new philosopher with given forks and name
        fn new(left_fork: JMutex<Fork>, right_fork: JMutex<Fork>) -> Self {
            Self {
                left_fork,
                right_fork,
            }
        }

        // Function for philosopher to eat
        fn eat_same(&self) -> Result<(), JError> {
            println!(
                "{:?} is thinking with locks {} and {}",
                tid(),
                self.left_fork.id(),
                self.right_fork.id()
            );

            let l = sync!(
                [self.left_fork.clone(), self.right_fork.clone()],
                |_left_guard, r| {
                    thread::sleep(std::time::Duration::from_millis(100));
                    let r = sync!([r], |g| {
                        println!("{:?} is eating with {:?}", tid(), g);

                        // Simulate eating
                        thread::sleep(std::time::Duration::from_millis(100));

                        println!("{:?} has finished eating.", tid());
                    });

                    match r {
                        Ok(o) => {
                            println!("{:?} Inner OK", tid());
                            Ok(o)
                        }
                        Err(e) => {
                            println!("{:?} Inner Err: {:?}", tid(), e);
                            Err(e)
                        }
                    }
                }
            );

            match l {
                Ok(ll) => match ll {
                    Ok(()) => {
                        println!("{:?} is thinking again.", tid());
                        return Ok(());
                    }
                    Err(e) => {
                        println!("{:?} faced error: {:?}", tid(), e);
                        return Err(e);
                    }
                },

                Err(e) => {
                    println!("{:?} faced error: {:?}", tid(), e);
                    return Err(e);
                }
            }
        }

        // Function for philosopher to eat
        fn eat_different(&self) -> Result<(), JError> {
            let l = sync!([self.left_fork.clone()], |_gl| {
                thread::sleep(std::time::Duration::from_millis(100));
                let r = sync!([self.right_fork.clone()], |_gr| {
                    println!("{:?} is eating.", tid());
                    // Simulate eating
                    thread::sleep(std::time::Duration::from_millis(100));

                    println!("{:?} has finished eating.", tid());
                });

                match r {
                    Ok(o) => return Ok(o),
                    Err(e) => return Err(e),
                }
            });

            match l {
                Ok(ll) => match ll {
                    Ok(r) => {
                        println!("{:?} is thinking again.", tid());
                        return Ok(r);
                    }
                    Err(e) => {
                        println!("{:?} faced error: {:?}", tid(), e);
                        return Err(e);
                    }
                },

                Err(e) => {
                    println!("{:?} faced error: {:?}", tid(), e);
                    return Err(e);
                }
            }
        }
    }

    #[test]
    fn dining_philosophers_same_region() {
        let r = Region::new();

        // Initialize forks
        let forks = vec![
            JMutex::new(Fork(0), r.clone()),
            JMutex::new(Fork(1), r.clone()),
            JMutex::new(Fork(2), r.clone()),
            JMutex::new(Fork(3), r.clone()),
        ];

        // Initialize dining_philosophers
        let philosophers = vec![
            Philosopher::new(forks[0].clone(), forks[1].clone()),
            Philosopher::new(forks[1].clone(), forks[2].clone()),
            Philosopher::new(forks[2].clone(), forks[3].clone()),
            Philosopher::new(forks[3].clone(), forks[0].clone()),
        ];

        // Create threads for each philosopher to eat
        let handles: Vec<_> = philosophers
            .into_iter()
            .map(|philosopher| thread::spawn(move || philosopher.eat_same()))
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| (handle.thread().id(), handle.join()))
            .collect();

        let mut one_fail = false;
        results.iter().for_each(|(tid, result)| match result {
            Ok(rr) => match rr {
                Ok(_) => {
                    println!("{:?} existed successfully", tid);
                }

                Err(e) => {
                    println!("{:?} exited with error: {:?}", tid, e);
                    one_fail = true;
                }
            },
            Err(e) => {
                println!("{:?} exited with error: {:?}", tid, e);
            }
        });

        assert_eq!(one_fail, false);
    }

    #[test]
    fn dining_philosophers_different_region() {
        let r1 = Region::new();
        let r2 = Region::new();
        let r3 = Region::new();
        let r4 = Region::new();

        // Initialize forks
        let forks = vec![
            JMutex::new(Fork(0), r1),
            JMutex::new(Fork(1), r2),
            JMutex::new(Fork(2), r3),
            JMutex::new(Fork(3), r4),
        ];

        // Initialize dining_philosophers
        let philosophers = vec![
            Philosopher::new(forks[0].clone(), forks[1].clone()),
            Philosopher::new(forks[1].clone(), forks[2].clone()),
            Philosopher::new(forks[2].clone(), forks[3].clone()),
            Philosopher::new(forks[3].clone(), forks[0].clone()),
        ];

        // Create threads for each philosopher to eat
        let handles: Vec<_> = philosophers
            .into_iter()
            .map(|philosopher| thread::spawn(move || philosopher.eat_different()))
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| (handle.thread().id(), handle.join()))
            .collect();

        let mut one_fail = false;
        results.iter().for_each(|(tid, result)| match result {
            Ok(rr) => match rr {
                Ok(_) => {
                    println!("{:?} existed successfully", tid);
                }

                Err(e) => {
                    println!("{:?} exited with error: {:?}", tid, e);
                    one_fail = true;
                }
            },
            Err(e) => {
                println!("{:?} exited with error: {:?}", tid, e);
            }
        });

        assert_eq!(one_fail, true);
    }

    #[test]
    fn concurrent_access() {
        let r = Region::new();
        let shared_data = JMutex::new(vec![1, 2, 3], r);
        let mut handles = vec![];

        for _ in 0..10 {
            let data_clone = shared_data.clone();
            handles.push(thread::spawn(move || {
                let result = sync!([data_clone], |guard| { guard.iter().sum::<i32>() });
                assert_eq!(result.expect("Failed to lock"), 6);
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn same_regions() {
        let r = Region::new();
        let mutex1 = JMutex::new(1, r.clone());
        let mutex2 = JMutex::new(2, r.clone());

        let m1_clone = mutex1.clone();
        let m2_clone = mutex2.clone();

        let handle1 = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            let o = sync!([m1_clone, m2_clone], |_guard1, m2| {
                thread::sleep(Duration::from_millis(100));
                let i = sync!([m2], |_m2g| {
                    thread::sleep(Duration::from_millis(100));
                    // Use locks
                });

                return i;
            });

            match o {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        });

        let handle2 = thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let o = sync!([mutex2, mutex1], |_guard2, m1| {
                thread::sleep(Duration::from_millis(500));
                let i = sync!([m1], |_m1g| {
                    thread::sleep(Duration::from_millis(500));
                    // Use locks
                });

                return i;
            });

            match o {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        });

        let h1 = handle1.join();
        let h2 = handle2.join();

        assert_eq!(h1.is_ok(), true);
        assert_eq!(h2.is_ok(), true);
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
            thread::sleep(Duration::from_millis(100));
            let l = sync!([m1c], |_g1| {
                thread::sleep(Duration::from_millis(100));
                let r = sync!([m2c], |_g2| {
                    thread::sleep(Duration::from_millis(100));
                    // Use locks
                });

                return r;
            });

            match l {
                Ok(rr) => match rr {
                    Ok(f) => return Ok(f),
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        });

        let h2 = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));

            let l = sync!([m2], |_g2| {
                thread::sleep(Duration::from_millis(100));
                let r = sync!([m1], |_g1| {
                    thread::sleep(Duration::from_millis(100));
                    // Use locks
                });

                return r;
            });

            match l {
                Ok(rr) => match rr {
                    Ok(f) => return Ok(f),
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        });

        let a = h1.join().unwrap();
        let b = h2.join().unwrap();

        if a.is_ok() {
            assert_eq!(b.is_err(), true);
        } else if b.is_ok() {
            assert_eq!(a.is_err(), true);
        }
    }
}
