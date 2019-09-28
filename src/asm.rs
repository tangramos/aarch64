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

use addr::{PhysAddr, VirtAddr};
use paging::PhysFrame;
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

/// Invalidate all TLB entries.
#[inline(always)]
pub fn tlb_invalidate_all() {
    // All stage 1 translations used at EL1, in the Inner Shareable shareability
    // domain.
    unsafe {
        asm!(
            "dsb ishst
             tlbi vmalle1is
             dsb ish
             isb"
            :::: "volatile"
        );
    }
}

/// Invalidate TLB entries that would be used to translate the specified address.
#[inline(always)]
pub fn tlb_invalidate(vaddr: VirtAddr) {
    // Translations used at EL1 for the specified address, for all ASID values,
    // in the Inner Shareable shareability domain.
    unsafe {
        asm!(
            "dsb ishst
             tlbi vaale1is, $0
             dsb ish
             isb"
            :: "r"(vaddr.as_u64() >> 12)
            :: "volatile"
        );
    }
}

/// Invalidate all instruction caches in Inner Shareable domain to Point of Unification.
#[inline(always)]
pub fn flush_icache_all() {
    unsafe {
        asm!(
            "ic ialluis
             dsb ish
             isb"
            :::: "volatile"
        );
    }
}

/// Clean and Invalidate data cache by address to Point of Coherency.
#[inline(always)]
pub fn flush_dcache_line(vaddr: usize) {
    unsafe {
        asm!("dc civac, $0" :: "r"(vaddr) :: "volatile");
    }
}

/// Clean and Invalidate data cache by address range to Point of Coherency.
pub fn flush_dcache_range(start: usize, end: usize) {
    let line_size = 4 << CTR_EL0.read(CTR_EL0::DminLine);
    let mut addr = start & !(line_size - 1);
    while addr < end {
        flush_dcache_line(addr);
        addr += line_size;
    }
    unsafe {
        asm!("dsb sy; isb" :::: "volatile");
    }
}

/// Address Translate (Stage 1 EL1 Read).
///
/// For Raspi 3, it always return the result of a translation table walk,
/// regardless of the TLB caching.
#[inline(always)]
pub fn address_translate(vaddr: usize) -> usize {
    let paddr: usize;
    unsafe {
        asm!(
            "at S1E1R, $1
             mrs $0, par_el1"
            : "=r"(paddr)
            : "r"(vaddr)
            :: "volatile"
        );
    }
    paddr
}

/// Read TTBRx_EL1 as PhysFrame
#[inline(always)]
pub fn ttbr_el1_read(which: u8) -> PhysFrame {
    let baddr = match which {
        0 => TTBR0_EL1.get_baddr(),
        1 => TTBR1_EL1.get_baddr(),
        _ => 0,
    };
    PhysFrame::containing_address(PhysAddr::new(baddr))
}

/// Write TTBRx_EL1 from PhysFrame
#[inline(always)]
pub fn ttbr_el1_write(which: u8, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.set_baddr(baddr),
        1 => TTBR1_EL1.set_baddr(baddr),
        _ => {}
    };
}

/// Read TTBRx_EL1 as PhysFrame and ASID
#[inline(always)]
pub fn ttbr_el1_read_asid(which: u8) -> (u16, PhysFrame) {
    let (asid, baddr) = match which {
        0 => (TTBR0_EL1.get_asid(), TTBR0_EL1.get_baddr()),
        1 => (TTBR1_EL1.get_asid(), TTBR1_EL1.get_baddr()),
        _ => (0, 0),
    };
    (asid, PhysFrame::containing_address(PhysAddr::new(baddr)))
}

/// write TTBRx_EL1 from PhysFrame and ASID
#[inline(always)]
pub fn ttbr_el1_write_asid(which: u8, asid: u16, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.write(TTBR0_EL1::ASID.val(asid as u64) + TTBR0_EL1::BADDR.val(baddr >> 1)),
        1 => TTBR1_EL1.write(TTBR1_EL1::ASID.val(asid as u64) + TTBR1_EL1::BADDR.val(baddr >> 1)),
        _ => {}
    };
}
