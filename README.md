# JThread-rs - deadlock-free mutex lock library

JThread is a Rust library designed to facilitate advanced mutex management in multi-threaded environments while providing *deadlock-freedom*. It provides mutexes with fine-grained control, and custom error handling, tailored for achieving deadlock-freedom.

## Features

- `Region` and `JMutex` for advanced mutex locking strategies.
- Custom error types (`JError`) for specific mutex and region-related errors.
- Convenient `sync` macro for easy and safe locking of one or more mutexes.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
jthread = "0.1.0"
```

## Usage

Lock creation:
```rust
use jthread::{JMutex, Region};

let region = Region::new();
let mutex = JMutex::new(5, region); // Protecting an integer
```

Using the `sync` macro:
```rust
use jthread::{JMutex, sync, Region};

let data_mutex = JMutex::with_default(5); // Protecting an integer
let result = sync!([data_mutex], |data_guard| {
    *data_guard += 1; // Modify the data
    *data_guard
});
```

Our `tests` module has the infamous Dining Philosophers problem setup using JThread. Feel free to refer to the variations provided in the module as advanced example usage.

## Tests

The library includes a comprehensive test suite demonstrating its capabilities, especially in complex multi-threading scenarios. Refer to the tests module for examples and usage patterns.


## Documentation

For more detailed information on each component and method, refer to the inline documentation within the library source code, generated using Rust's standard documentation tool, `cargo doc`.
