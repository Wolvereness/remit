
# Remit

[![Crates.io version](https://img.shields.io/crates/v/remit.svg)](https://crates.io/crates/remit)
[![docs.rs status](https://docs.rs/remit/badge.svg)](https://docs.rs/remit)
[![Crates.io license](https://img.shields.io/crates/l/remit.svg)](https://crates.io/crates/remit)

Rust generators implemented through async/await syntax.

The pinned implementation is stack-based, and the boxed is heap-based.
No fancy macros and a simple API. Values can be lazily or eagerly yielded.

This crate is inherently no-std, and the default `alloc` feature can be disabled.

Some behaviors exhibited by the *lack* of `alloc` are not part of the SemVer.
For example, not awaiting before another remit, without alloc, is
[unspecified](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html)
behavior.

## Usage

Add to dependencies:

```toml
[dependencies]
remit = "0.1.1"
```

Example code:
```rust
use std::pin::pin;
use remit::{Generator, Remit};

async fn gen(remit: Remit<'_, usize>) {
    remit.value(42).await;
    // Does not need to be limited
    for i in 1.. {
        remit.value(i).await
    }
}
for item in pin!(Generator::new()).of(gen).take(10) {
    println!("{item}");
    // Prints 42, 1, 2, 3, 4, 5, 6, 7, 8, 9
}
assert_eq!(vec![42, 1, 2, 3], pin!(Generator::new()).of(gen).take(4).collect::<Vec<_>>());
/* // Rust has trouble determining the lifetime
assert_eq!(
    vec![1],
    pin!(Generator::new())
        .of(|remit: Remit<'_, usize>| async move { remit.value(1).await; })
        .collect::<Vec<_>>(),
);
*/
assert_eq!(vec![42, 1], Generator::boxed(gen).take(2).collect::<Vec<_>>());
assert_eq!(vec![1], Generator::boxed(|remit| async move { remit.value(1).await; }).collect::<Vec<_>>());

fn iter() -> impl Iterator<Item=usize> {
    Generator::boxed(gen)
}

async fn scream<D: std::fmt::Display>(iter: impl Iterator<Item=D>, remit: Remit<'_, String>) {
    for person in iter {
        remit.value(format!("{person} scream!")).await
    }
    remit.value("... for ice cream!".to_string());
}
let expected: Vec<String> = ["You scream!", "I scream!", "We all scream!", "... for ice cream!"].iter().map(ToString::to_string).collect();
assert_eq!(
    expected,
    pin!(Generator::new()).parameterized(scream, ["You", "I", "We all"].iter()).collect::<Vec<String>>(),
);
assert_eq!(
    expected,
    Generator::boxed(|remit| scream(["You", "I", "We all"].iter(), remit)).collect::<Vec<String>>(),
);
```

## License

MIT or APACHE-2, at your option.

See respective LICENSE files.
