/*
 * Copyright (c) 2018 by the author(s)
 *
 * =============================================================================
 *
 * Licensed under either of
 *   - Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
 *   - MIT License (http://opensource.org/licenses/MIT)
 * at your option.
 *
 * =============================================================================
 *
 * Author(s):
 *   - Jorge Aparicio
 *   - Andre Richter <andre.o.richter@gmail.com>
 *   - Yuekai Jia <equation618@gmail.com>
 */

//! Miscellaneous assembly instructions and functions

use regs::*;

/// Returns the current stack pointer.
#[inline(always)]
pub fn sp() -> *const u8 {
    let ptr: usize;
    unsafe {
        asm!("mov $0, sp" : "=r"(ptr) ::: "volatile");
    }

    ptr as *const u8
}

/// Returns the current point counter.
#[inline(always)]
pub unsafe fn get_pc() -> usize {
    let pc: usize;
    asm!("adr $0, ." : "=r"(pc) ::: "volatile");
    pc
}

/// The classic no-op
#[inline]
pub fn nop() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("nop" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Wait For Interrupt
#[inline]
pub fn wfi() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("wfi" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Wait For Event
#[inline]
pub fn wfe() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("wfe" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Send Event
#[inline]
pub fn sev() {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe { asm!("sev" :::: "volatile") },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// Exception return
///
/// Will jump to wherever the corresponding link register points to, and
/// therefore never return.
#[inline]
pub fn eret() -> ! {
    match () {
        #[cfg(target_arch = "aarch64")]
        () => unsafe {
            asm!("eret" :::: "volatile");
            unreachable!()
        },

        #[cfg(not(target_arch = "aarch64"))]
        () => unimplemented!(),
    }
}

/// CPU id
#[inline]
pub fn cpuid() -> usize {
    (MPIDR_EL1.get() & 3) as usize
}
