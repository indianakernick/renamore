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

[`std::fs::rename`]: https://doc.rust-lang.org/std/fs/fn.rename.html
[TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use

## Example

Renaming a file without the possibility of accidentally overwriting anything
can be done using [`rename_exclusive`]. It should be noted that this feature
is not supported by all combinations of operation system and file system.
The return value will indicate whether the operation was performed
atomically or whether a non-atomic fallback was used.

[`rename_exclusive`]: https://docs.rs/renamore/latest/renamore/fn.rename_exclusive.html

```rust
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let from = PathBuf::from("old.txt");
    let to = PathBuf::from("new.txt");

    if renamore::rename_exclusive(&from, &to)? {
        // `new.txt` will definitely not be overwritten.
        println!("The operation was atomic");
    } else {
        // `new.txt` will probably not be overwritten.
        println!("The operation was not atomic");
    }

    Ok(())
}
```

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
