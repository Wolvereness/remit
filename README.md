
# Remit

[![Crates.io version](https://img.shields.io/crates/v/remit.svg)](https://crates.io/crates/remit)
[![docs.rs status](https://docs.rs/remit/badge.svg)](https://docs.rs/remit)
[![Crates.io license](https://img.shields.io/crates/l/remit.svg)](https://crates.io/crates/remit)

Rust generators implemented through async/await syntax.

The pinned implementation is stack-based, and the boxed is heap-based.
No fancy macros and a simple API.
Values are remitted/yielded analogous to how they're awaited.
Response values can also be sent back to the awaiting location.
The generator can also use normal async API when the iterator is called correctly.

This crate is inherently no-std, and the default `alloc` feature can be disabled.
This crate also uses no dependencies outside of `core` and `alloc`.

Some behaviors exhibited by the *lack* of `alloc` are not part of the SemVer.
For example, not awaiting each value one at a time, without alloc, is
[unspecified](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html)
behavior.

## Usage

Add to dependencies:

```toml
[dependencies]
remit = "0.2.0"
```

Example code:
```rust
use std::borrow::Cow;
use std::pin::{pin, Pin};
use remit::*;
use std::future::{Future, poll_fn};
use std::task::Poll;
fn main() {

async fn gen(remit: Remit<'_, usize>) {
    remit.value(42).await;
    // Does not need to be limited
    for i in 1.. {
        remit.value(i).await
    }
}
for item in pin!(Generators::new()).of(gen).take(10) {
    println!("{item}");
    // Prints 42, 1, 2, 3, 4, 5, 6, 7, 8, 9
}
assert_eq!(vec![42, 1, 2, 3], pin!(Generators::new()).of(gen).take(4).collect::<Vec<_>>());
/* // Rust has trouble determining the lifetime
assert_eq!(
    vec![1],
    pin!(Generator::new())
        .of(|remit: Remit<'_, usize>| async move { remit.value(1).await; })
        .collect::<Vec<_>>(),
);
*/
assert_eq!(vec![42, 1], Generators::boxed(gen).take(2).collect::<Vec<_>>());
assert_eq!(vec![1], Generators::boxed(|remit| async move { let () = remit.value(1).await; }).collect::<Vec<_>>());

fn iter() -> impl Iterator<Item=usize> {
    Generators::boxed(gen)
}

async fn scream<D: std::fmt::Display>(iter: impl Iterator<Item=D>, remit: Remit<'_, String>) {
    for person in iter {
        remit.value(format!("{person} scream!")).await
    }
    remit.value("... for ice cream!".to_string()).await;
}
let expected: Vec<String> = ["You scream!", "I scream!", "We all scream!", "... for ice cream!"].iter().map(ToString::to_string).collect();
assert_eq!(
    expected,
    pin!(Generators::new()).parameterized(scream, ["You", "I", "We all"].iter()).collect::<Vec<String>>(),
);
assert_eq!(
    expected,
    Generators::boxed(|remit| scream(["You", "I", "We all"].iter(), remit)).collect::<Vec<String>>(),
);
    
    async fn blah<'a>(cell: &Cell<Option<Remit<'a, usize>>>, remit: Remit<'a, usize>) {
        cell.set(remit); // UB for stack-generators!
    }

// Sending/exchanging values back into the generator-function:
async fn health_regen(
    (starting, regen, minimum): (usize, usize, usize),
    remit: Remit<'_, usize, usize>,
) {
    let mut current = starting;
    while current >= minimum {
        let damage = remit.value(current).await;
        current -= damage;
        current += regen;
    }
}
let mut buffer = vec![];
for exchange in pin!(Generators::new()).parameterized_exchange(health_regen, (400, 3, 20)) {
    let (previous, to_damage) = exchange.take();
    buffer.push(previous);
    to_damage.provide(previous * 2 / 7);
}
assert_eq!(
    buffer,
    vec![400, 289, 210, 153, 113, 84, 63, 48, 38, 31, 26, 22],
);

// Working with actual async functions:
async fn gen_async(data: Cow<'_, str>, remit: Remit<'_, usize>) {
    // This can await on any normal future,
    // because callers use a context via .next_item_future() or .poll_next_item()
    remit.value(data.len()).await;
    remit.value(data.len()).await;
}

async fn async_function_stack() {
    let data = String::from("This future is automatic");
    let stack = pin!(Generators::new());
    let mut stream = stack.parameterized(gen_async, Cow::Borrowed(&data));
    while let Some(item) = stream.next_item_future().await {
        dbg!(item);
    }
}

fn async_function_unpin() -> impl Future<Output=()> + Unpin {
    let data = String::from("This future is unpin");
    let mut boxed = Generators::boxed(move |remit| gen_async(Cow::Owned(data), remit));
    poll_fn(move |cx| {
        while let Poll::Ready(next) = Pin::new(&mut boxed).poll_next_item(cx) {
            let Some(item) = next
                else { return Poll::Ready(()); };
            dbg!(item);
        }
        Poll::Pending
    })
}

}
```

## License

MIT or APACHE-2, at your option.

See respective LICENSE files.
