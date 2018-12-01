//! Low level access to Cortex-A processors
//!
//! This crate provides:
//!
//! - Safe wrappers around assembly instructions
//!
//! For now, there's not much. I will update it gradually.
//! If you want to contribute, feel free to reach out!

#![no_std]
#![feature(asm)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(try_from)]

#[macro_use]
extern crate register;
#[macro_use]
extern crate bitflags;
extern crate bit_field;
extern crate usize_conversions;

/// Provides the non-standard-width integer types `u2`–`u63`.
///
/// We use these integer types in various APIs, for example `u9` for page tables indices.
pub extern crate ux;

pub use addr::{align_down, align_up, PhysAddr, VirtAddr};

pub mod asm;
pub mod addr;
pub mod paging;
pub mod barrier;
pub mod regs;
