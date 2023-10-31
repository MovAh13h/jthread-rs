use crate::RR;

#[macro_export]
macro_rules! sync {
    ([$mutex1:expr], $closure:expr) => {
        // TODO: Figure out regions for this
        
        let guard1 = $mutex1.lock().unwrap();
        $closure(guard1);
    };

    ([$mutex1:expr, $mutex2:expr], $closure:expr) => {

        // Region Checks
        sync!(@REGION_CHECK, $mutex1, $mutex2);

        // Prelocking
        sync!(@PRELOCK, $mutex2);

        // Unlock first lock
        let guard1 = $mutex1.lock().unwrap();

        // Call closure
        $closure(guard1, $mutex2);
    };

    ([$mutex1:expr, $mutex2:expr, $mutex3:expr], $closure:expr) => {
        // Region Checks
        sync!(@REGION_CHECK, $mutex1, $mutex2);
        sync!(@REGION_CHECK, $mutex1, $mutex3);

        // Prelocking
        sync!(@PRELOCK, $mutex2);
        sync!(@PRELOCK, $mutex3);

        // Unlock first lock
        let guard1 = $mutex1.lock().unwrap();

        // Call closure
        $closure(guard1, $mutex2, $mutex3);
    };

    (@REGION_CHECK, $mutex1:expr, $mutex2:expr) => {
        let ra = $mutex1.region_id();
        let rb = $mutex2.region_id();

        let mut guard = RR.lock().unwrap();

        match guard.check_relation((ra, rb)) {
            Some(b) => {
                if !b {
                    panic!("Incorrect lock acquisition ordering");
                }
            }
            None => guard.insert((ra, rb))
        }

        drop(guard);
    };

    (@PRELOCK, $mutex:expr) => {
        drop($mutex.lock().unwrap());
    };
}

#[cfg(test)]
mod sync {
    use crate::RR;
    use crate::JMutex;

    #[test]
    fn one_lock() {
        let mut m1 = JMutex::new(String::from("1"));

        sync!([m1], |s| {
            println!("{}", s);
        });
    }

    #[test]
    fn two_lock() {
        let mut m1 = JMutex::new(String::from("1"));
        let mut m2 = JMutex::new(String::from("2"));

        sync!([m1, m2], |s, mut m2: JMutex<String>| {
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

        sync!([m1, m2, m3], |s, mut m2: JMutex<String>, mut m3: JMutex<String>| {
            println!("{}", s);

            let guard2 = m2.lock().unwrap();
            println!("{}", guard2);
            
            let guard3 = m3.lock().unwrap();
            println!("{}", guard3);
        });
    }
}