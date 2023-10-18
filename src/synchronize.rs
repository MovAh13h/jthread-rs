#[macro_export]
macro_rules! synchronize {
    ([$mutex1:expr], $closure:expr) => {
        let guard = REGION_MANAGER.lock().unwrap();

        // Region Checks
        synchronize!(@REGION_CHECK, guard, $mutex1);

        drop(guard);

        // Acquire JMutex
        let guard1 = $mutex1.lock().unwrap();

        // Call closure
        $closure(guard1);
    };


    ([$mutex1:expr, $mutex2:expr], $closure:expr) => {
        let mut guard = REGION_MANAGER.lock().unwrap();
        
        // Region Checks
        synchronize!(@REGION_CHECK, guard, $mutex1);
        synchronize!(@REGION_CHECK, guard, $mutex2);
        
        // Handle relations
        synchronize!(@HANDLE_RELATION, guard, $mutex1, $mutex2);

        drop(guard);

        // Prelocking
        synchronize!(@PRELOCK, $mutex2);

        // Acquire JMutex
        let guard1 = $mutex1.lock().unwrap();

        // Call closure
        $closure(guard1, $mutex2);
    };

    ([$mutex1:expr, $mutex2:expr, $mutex3:expr], $closure:expr) => {
        let mut guard = REGION_MANAGER.lock().unwrap();

        // Region Checks
        synchronize!(@REGION_CHECK, guard, $mutex1);
        synchronize!(@REGION_CHECK, guard, $mutex2);
        synchronize!(@REGION_CHECK, guard, $mutex3);
        
        // Handle relations
        synchronize!(@HANDLE_RELATION, guard, $mutex1, $mutex2);
        synchronize!(@HANDLE_RELATION, guard, $mutex1, $mutex3);

        drop(guard);

        // Prelocking
        synchronize!(@PRELOCK, $mutex2);
        synchronize!(@PRELOCK, $mutex3);

        // Acquire JMutex
        let guard1 = $mutex1.lock().unwrap();

        // Call closure
        $closure(guard1, $mutex2, $mutex3);
    };

    (@PRELOCK, $mutex:expr) => {
        drop($mutex.lock().unwrap());
    };

    (@REGION_CHECK, $guard:expr, $mutex:expr) => {
        let region_check = $guard.check_region($mutex.lock_id());

        if !region_check {
            panic!("Lock does not belong to the region");
        }
    };

    (@HANDLE_RELATION, $guard:expr, $mutex1:expr, $mutex2:expr) => {
        $guard.handle_relation($mutex1.lock_id(), $mutex2.lock_id());
    }
}


#[cfg(test)]
mod synchronize {
    use crate::REGION_MANAGER;
    use crate::JMutex;

    #[test]
    fn one_lock() {
        let mut m1 = JMutex::new(String::from("1"));

        synchronize!([m1], |s| {
            println!("{}", s);
        });
    }

    #[test]
    fn two_lock() {
        let mut m1 = JMutex::new(String::from("1"));
        let mut m2 = JMutex::new(String::from("2"));

        synchronize!([m1, m2], |s, mut m2: JMutex<String>| {
            println!("{}", s);

            let guard2 = m2.lock().unwrap();
            println!("{}", guard2);
        });
    }

    #[test]
    fn three_lock() {
        let mut m1 = JMutex::new(String::from("1"));
        let mut m2 = JMutex::new(String::from("2"));
        let mut m3 = JMutex::new(String::from("3"));

        synchronize!([m1, m2, m3], |s, mut m2: JMutex<String>, mut m3: JMutex<String>| {
            println!("{}", s);

            let guard2 = m2.lock().unwrap();
            println!("{}", guard2);

            let guard3 = m3.lock().unwrap();
            println!("{}", guard3);
        });
    }
}