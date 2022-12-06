# Renamore

More ways to rename files.

## Overview

The Rust standard library offers [`std::fs::rename`] for renaming files.
Sometimes, that's not enough. Consider the example of renaming a file but
aborting the operation if something already exists at the destination path.
That can be achieved using the Rust standard library but ensuring that the
operation is atomic can only be achieved using platform-specific APIs.
Without using platform-specific APIs, a [TOCTTOU] bug can be introduced.
This library aims to provide a cross-platform interface to these APIs.

[TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
[`std::fs::rename`]: https://doc.rust-lang.org/std/fs/fn.rename.html

## Example

Renaming a file without the possibility of accidentally overwriting anything
can be done using [`rename_exclusive`].

[`rename_exclusive`]: https://docs.rs/renamore/latest/renamore/fn.rename_exclusive.html

```rust
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let from = PathBuf::from("old.txt");
    let to = PathBuf::from("new.txt");

    renamore::rename_exclusive(&from, &to)
}
```

It should be noted that this feature is not supported by all combinations of
operating system and file system. Support can be checked by calling
[`rename_exclusive_is_supported`]. If the feature is not supported, then
[`rename_exclusive_non_atomic`] can be used. As the same suggests, this is a
non-atomic version of `rename_exclusive`.

[`rename_exclusive_is_supported`]: https://docs.rs/renamore/latest/renamore/fn.rename_exclusive_is_supported.html
[`rename_exclusive_non_atomic`]: https://docs.rs/renamore/latest/renamore/fn.rename_exclusive_non_atomic.html

```rust
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let from = PathBuf::from("old.txt");
    let to = PathBuf::from("new.txt");

    // Checking if rename_exclusive is supported by the current OS version
    // using the file system of the current directory.
    if renamore::rename_exclusive_is_supported(".")? {
        // It's supported!
        // `new.txt` will definitely not be overwritten.
        renamore::rename_exclusive(&from, &to)
    } else {
        // Oh no!
        // `new.txt` will probably not be overwritten.
        renamore::rename_exclusive_non_atomic(&from, &to)
    }
}
```

Doing this check can be a little bit verbose. For this reason, there is
[`rename_exclusive_checked`] which will check for support and switch between
the atomic and non-atomic implementations.

[`rename_exclusive_checked`]: https://docs.rs/renamore/latest/renamore/fn.rename_exclusive_checked.html

```rust
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let from = PathBuf::from("old.txt");
    let to = PathBuf::from("new.txt");

    renamore::rename_exclusive_checked(&from, &to)
}
```

For doing a bulk-rename rather than a one-off, it may be more efficient to
do the check for support manually rather than using
`rename_exclusive_checked`. That would mean doing the check once rather than
for every rename.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
