#[macro_export]
macro_rules! synchronize {
    ([$mutex1:expr], $closure:expr) => {
        let guard1 = $mutex1.lock().unwrap();

        $closure(guard1);
    };


    ([$mutex1:expr, $mutex2:expr], $closure:expr) => {
        drop($mutex2.lock());

        let guard1 = $mutex1.lock().unwrap();

        $closure(guard1, $mutex2);
    };

    ([$mutex1:expr, $mutex2:expr, $mutex3:expr], $closure:expr) => {
        drop($mutex2.lock());
        drop($mutex3.lock());

        let guard1 = $mutex1.lock().unwrap();

        $closure(guard1, $mutex2, $mutex3);
    };
}


#[cfg(test)]
mod tests {
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