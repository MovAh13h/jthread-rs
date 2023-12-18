use std::sync::Mutex;

use criterion::{criterion_group, criterion_main, Criterion};
use jthread::*;

pub fn single_lock_benchmark(c: &mut Criterion) {
	c.bench_function("Standard locking (single)", |b| {
		let m = Mutex::new(1);
		b.iter(|| {
			let mut d = m.lock().unwrap();
			*d += 1;
		});
	});

	c.bench_function("JThread locking (single)", |b| {
		let r = Region::new();
		let m = JMutex::new(1, r);
		b.iter(|| {
			let _ = sync!([m.clone()], |mut d| {
				*d += 1;	
			});
		});
	});
}

pub fn double_lock_benchmark(c: &mut Criterion) {
	c.bench_function("Standard locking (double)", |b| {
		let m = Mutex::new(1);
		let n = Mutex::new(2);
		b.iter(|| {
			let mut d1 = m.lock().unwrap();
			let d2 = n.lock().unwrap();
			*d1 += *d2;
		});
	});

	c.bench_function("JThread locking (double)", |b| {
		let r1 = Region::new();
		let m = JMutex::new(1, r1);

		let r2 = Region::new();
		let n = JMutex::new(2, r2);

		b.iter(|| {
			let _ = sync!([m.clone()], |mut d1| {
				let _ = sync!([n.clone()], |d2| {
					*d1 += *d2;
				});
			});
		});
	});
}

criterion_group!(benches, single_lock_benchmark, double_lock_benchmark);
criterion_main!(benches);