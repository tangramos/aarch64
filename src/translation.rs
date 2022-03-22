use crate::{
    addr::{PhysAddr, VirtAddr},
    paging::PhysFrame,
    registers::*,
};

/// Address Translate (Stage 1 EL1 Read).
///
/// For Raspi 3, it always return the result of a translation table walk,
/// regardless of the TLB caching.
#[inline]
pub fn address_translate(vaddr: usize) -> usize {
    let paddr: usize;
    unsafe {
        core::arch::asm!(
            "at S1E1R, {vaddr}",
            "mrs {paddr}, par_el1",
            vaddr = in(reg) vaddr,
            paddr = out(reg) paddr,
            options(pure, readonly)
        )
    }
    paddr
}

/// Read TTBRx_EL1 as PhysFrame
#[inline]
pub fn ttbr_el1_read(which: u8) -> PhysFrame {
    let baddr = match which {
        0 => TTBR0_EL1.get_baddr(),
        1 => TTBR1_EL1.get_baddr(),
        _ => 0,
    };
    PhysFrame::containing_address(PhysAddr::new(baddr))
}

/// Write TTBRx_EL1 from PhysFrame
#[inline]
pub fn ttbr_el1_write(which: u8, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.set_baddr(baddr),
        1 => TTBR1_EL1.set_baddr(baddr),
        _ => {}
    };
}

/// Read TTBRx_EL1 as PhysFrame and ASID
#[inline]
pub fn ttbr_el1_read_asid(which: u8) -> (u16, PhysFrame) {
    let (asid, baddr) = match which {
        0 => (
            TTBR0_EL1.read(TTBR0_EL1::ASID) as u16,
            TTBR0_EL1.get_baddr(),
        ),
        1 => (
            TTBR1_EL1.read(TTBR1_EL1::ASID) as u16,
            TTBR1_EL1.get_baddr(),
        ),
        _ => (0, 0),
    };
    (asid, PhysFrame::containing_address(PhysAddr::new(baddr)))
}

/// write TTBRx_EL1 from PhysFrame and ASID
#[inline]
pub fn ttbr_el1_write_asid(which: u8, asid: u16, frame: PhysFrame) {
    let baddr = frame.start_address().as_u64();
    match which {
        0 => TTBR0_EL1.write(TTBR0_EL1::ASID.val(asid as u64) + TTBR0_EL1::BADDR.val(baddr >> 1)),
        1 => TTBR1_EL1.write(TTBR1_EL1::ASID.val(asid as u64) + TTBR1_EL1::BADDR.val(baddr >> 1)),
        _ => {}
    };
}

/// Invalidate all TLB entries in all PEs.
#[inline]
pub fn invalidate_tlb_all() {
    // All stage 1 translations used at EL1, in the Inner Shareable shareability
    // domain.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack)
        )
    }
}

/// Invalidate all TLB entries in the current PE.
#[inline]
pub fn local_invalidate_tlb_all() {
    // All stage 1 translations used at EL1
    unsafe {
        core::arch::asm!(
            "dsb nshst",
            "tlbi vmalle1",
            "dsb nsh",
            "isb",
            options(nostack)
        )
    }
}

/// Invalidate TLB entries in all PEs by the virtual address.
#[inline]
pub fn invalidate_tlb_vaddr(vaddr: VirtAddr) {
    // Translations used at EL1 for the specified address, for all ASID values,
    // in the Inner Shareable shareability domain.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vaae1is, {vaddr}",
            "dsb ish",
            "isb",
            vaddr = in(reg) vaddr.as_u64() >> 12,
            options(nostack)
        )
    }
}
