[![crates.io](https://img.shields.io/crates/d/cortex-a.svg)](https://crates.io/crates/cortex-a)
[![crates.io](https://img.shields.io/crates/v/cortex-a.svg)](https://crates.io/crates/cortex-a)

# aarch64

Low level access to Cortex-A 64-bit processors.

Used by [rCore](https://github.com/wangrunji0408/rCore).

## Usage

Example from https://github.com/andre-richter/rust-raspi3-tutorial

```rust
extern crate aarch64;

#[no_mangle]
pub unsafe extern "C" fn _boot_cores() -> ! {
    use aarch64::{asm, regs::*};

    const CORE_MASK: u64 = 0x3;
    const STACK_START: u64 = 0x80_000;

    match MPIDR_EL1.get() & CORE_MASK {
        0 => {
            SP.set(STACK_START);
            reset()
        }
        _ => loop {
            // if not core0, infinitely wait for events
            asm::wfe();
        },
    }
}
```

## Disclaimer

Descriptive comments in the source files are taken from the [ARM Architecture Reference Manual ARMv8, for ARMv8-A architecture profile](https://static.docs.arm.com/ddi0487/ca/DDI0487C_a_armv8_arm.pdf?_ga=2.266626254.1122218691.1534883460-1326731866.1530967873).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
