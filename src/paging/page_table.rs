//! Abstractions for page tables and page table entries.

use bitflags::bitflags;
use core::{
    fmt,
    ops::{Index, IndexMut},
};
use tock_registers::{fields::FieldValue, register_bitfields};
use ux::*;

use super::{PageSize, PhysFrame, Size4KiB};
use crate::PhysAddr;

/// Output address mask
pub const ADDR_MASK: u64 = 0x0000_ffff_ffff_f000;
/// Other flags mask
pub const FLAGS_MASK: u64 = !(MEMORY_ATTR_MASK | ADDR_MASK);

/// Memory attribute fields
pub type PageTableAttribute = FieldValue<u64, MEMORY_ATTRIBUTE::Register>;

/// The error returned by the `PageTableEntry::frame` method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameError {
    /// The entry does not have the `PRESENT` flag set, so it isn't currently mapped to a frame.
    FrameNotPresent,
    /// The entry does have the `HUGE_PAGE` flag set. The `frame` method has a standard 4KiB frame
    /// as return type, so a huge frame can't be returned.
    HugeFrame,
}

/// A 64-bit page table entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Creates an unused entry.
    #[inline]
    pub const fn new() -> Self {
        Self { entry: 0 }
    }

    /// Returns whether this entry is zero.
    #[inline]
    pub fn is_unused(&self) -> bool {
        self.entry == 0
    }

    /// Sets this entry to zero.
    #[inline]
    pub fn set_unused(&mut self) {
        self.entry = 0;
    }

    /// Returns the flags of this entry.
    #[inline]
    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.entry)
    }

    /// Returns the physical address mapped by this entry, might be zero.
    #[inline]
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.entry & ADDR_MASK)
    }

    /// Returns the memory attribute fields of this entry.
    #[inline]
    pub fn attr(&self) -> PageTableAttribute {
        PageTableAttribute::new(MEMORY_ATTR_MASK, 0, self.entry)
    }

    /// Returns whether this entry is mapped to a block.
    #[inline]
    pub fn is_block(&self) -> bool {
        !self.flags().contains(PageTableFlags::TABLE_OR_PAGE)
    }

    /// Returns the physical frame mapped by this entry.
    ///
    /// Returns the following errors:
    ///
    /// - `FrameError::FrameNotPresent` if the entry doesn't have the `PRESENT` flag set.
    /// - `FrameError::HugeFrame` if the entry has the `HUGE_PAGE` flag set (for huge pages the
    ///   `addr` function must be used)
    pub fn frame(&self) -> Result<PhysFrame, FrameError> {
        if !self.flags().contains(PageTableFlags::VALID) {
            Err(FrameError::FrameNotPresent)
        } else if self.is_block() {
            // is a huge page (block)
            Err(FrameError::HugeFrame)
        } else {
            Ok(PhysFrame::containing_address(self.addr()))
        }
    }

    /// Map the entry to the specified physical address with the specified flags and memory
    /// attribute.
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags, attr: PageTableAttribute) {
        debug_assert!(addr.is_aligned(Size4KiB::SIZE));
        self.entry = (addr.as_u64()) | flags.bits() | attr.value;
    }

    /// Map the entry to the specified physical frame with the specified flags and memory attribute.
    pub fn set_frame(&mut self, frame: PhysFrame, flags: PageTableFlags, attr: PageTableAttribute) {
        // is not a block
        debug_assert!(flags.contains(PageTableFlags::TABLE_OR_PAGE));
        self.set_addr(frame.start_address(), flags, attr);
    }

    /// The descriptor gives the base address of a block of memory, and the attributes for that
    /// memory region.
    pub fn set_block<S: PageSize>(
        &mut self,
        addr: PhysAddr,
        flags: PageTableFlags,
        attr: PageTableAttribute,
    ) {
        // is a block
        debug_assert!(!flags.contains(PageTableFlags::TABLE_OR_PAGE));
        self.set_addr(addr.align_down(S::SIZE), flags, attr);
    }

    /// Sets the flags of this entry.
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.entry = (self.entry & !FLAGS_MASK) | flags.bits();
    }

    /// Sets the memory attribute of this entry.
    pub fn set_attr(&mut self, attr: PageTableAttribute) {
        self.entry = (self.entry & !MEMORY_ATTR_MASK) | attr.value;
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("value", &self.entry);
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.field("attr", &self.attr().value);
        f.finish()
    }
}

register_bitfields! {u64,
    // Memory attribute fields in the VMSAv8-64 translation table format descriptors (Page 2148~2152)
    pub MEMORY_ATTRIBUTE [
        /// Shareability field
        SH       OFFSET(8) NUMBITS(2) [
            NonShareable = 0b00,
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// Memory attributes index into the MAIR_EL1 register
        AttrIndx OFFSET(2) NUMBITS(3) []
    ]
}

/// Memory attribute fields mask
pub const MEMORY_ATTR_MASK: u64 = (0b11 /* MEMORY_ATTRIBUTE::SH.mask */ << MEMORY_ATTRIBUTE::SH.shift)
    | (0b111 /* MEMORY_ATTRIBUTE::AttrIndx.mask */ << MEMORY_ATTRIBUTE::AttrIndx.shift);

bitflags! {
    /// Possible flags for a page table entry.
    pub struct PageTableFlags: u64 {
        /// identifies whether the descriptor is valid
        const VALID =           1 << 0;
        /// the descriptor type
        /// 0, Block
        /// 1, Table/Page
        const TABLE_OR_PAGE =   1 << 1;
        /// Non-secure bit
        const NS =              1 << 5;
        /// Access permission: accessable at EL0
        const AP_EL0 =          1 << 6;
        /// Access permission: read-only
        const AP_RO =           1 << 7;
        /// Access flag
        const AF =              1 << 10;
        /// not global bit
        const nG =              1 << 11;
        /// Dirty Bit Modifier
        const DBM =             1 << 51;

        /// A hint bit indicating that the translation table entry is one of a contiguous set or
        /// entries
        const Contiguous =      1 << 52;
        /// Privileged Execute-never
        const PXN =             1 << 53;
        /// Execute-never/Unprivileged execute-never
        const UXN =             1 << 54;

        /// Software Dirty Bit Modifier
        const WRITE =           1 << 51;
        /// Software dirty bit
        const DIRTY =           1 << 55;
        /// Software swapped bit
        const SWAPPED =         1 << 56;
        /// Software writable shared bit for COW
        const WRITABLE_SHARED = 1 << 57;
        /// Software readonly shared bit for COW
        const READONLY_SHARED = 1 << 58;

        /// Privileged Execute-never for table descriptors
        const PXNTable =        1 << 59;
        /// Execute-never/Unprivileged execute-never for table descriptors
        const XNTable =         1 << 60;
        /// Access permission: access at EL0 not permitted
        const APTable_nEL0 =    1 << 61;
        /// Access permission: read-only
        const APTable_RO =      1 << 62;
        /// Non-secure bit
        const NSTable =         1 << 63;
    }
}

impl PageTableFlags {
    /// default flags for the table entry
    #[inline]
    pub fn default_table() -> Self {
        Self::VALID | Self::TABLE_OR_PAGE
    }

    /// default flags for the block entry
    #[inline]
    pub fn default_block() -> Self {
        Self::VALID | Self::AF
    }

    /// default flags for the page entry
    #[inline]
    pub fn default_page() -> Self {
        Self::VALID | Self::TABLE_OR_PAGE | Self::AF
    }
}

/// The number of entries in a page table.
const ENTRY_COUNT: usize = 512;

/// Represents a page table.
///
/// Always page-sized.
///
/// This struct implements the `Index` and `IndexMut` traits, so the entries can be accessed
/// through index operations. For example, `page_table[15]` returns the 15th page table entry.
#[repr(align(4096))]
#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    /// Creates an empty page table.
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); ENTRY_COUNT],
        }
    }

    /// Clears all entries.
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }

    /// Returns an iterator over the entries of the page table.
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    /// Returns an iterator that allows modifying the entries of the page table.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<u9> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: u9) -> &Self::Output {
        &self.entries[cast::usize(u16::from(index))]
    }
}

impl IndexMut<u9> for PageTable {
    fn index_mut(&mut self, index: u9) -> &mut Self::Output {
        &mut self.entries[cast::usize(u16::from(index))]
    }
}

impl fmt::Debug for PageTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.entries[..].fmt(f)
    }
}
