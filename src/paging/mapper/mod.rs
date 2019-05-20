//! Abstractions for reading and modifying the mapping of pages.

mod mapped_page_table;
mod recursive_page_table;

pub use self::mapped_page_table::MappedPageTable;
pub use self::recursive_page_table::RecursivePageTable;

use crate::paging::{
    frame::PhysFrame,
    frame_alloc::FrameAllocator,
    page::{Page, PageSize, Size1GiB, Size2MiB, Size4KiB},
    page_table::{PageTableAttribute, PageTableEntry, PageTableFlags},
};
use crate::{PhysAddr, VirtAddr};

/// This trait defines page table operations that work for all page sizes of the aarch64
/// architecture.
pub trait MapperAllSizes: Mapper<Size4KiB> + Mapper<Size2MiB> + Mapper<Size1GiB> {
    /// Return the frame that the given virtual address is mapped to and the offset within that
    /// frame.
    ///
    /// If the given address has a valid mapping, the mapped frame and the offset within that
    /// frame is returned. Otherwise an error value is returned.
    ///
    /// This function works with huge pages of all sizes.
    fn translate(&self, addr: VirtAddr) -> TranslateResult;

    /// Translates the given virtual address to the physical address that it maps to.
    ///
    /// Returns `None` if there is no valid mapping for the given address.
    ///
    /// This is a convenience method. For more information about a mapping see the
    /// [`translate`](MapperAllSizes::translate) method.
    fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr> {
        match self.translate(addr) {
            TranslateResult::PageNotMapped | TranslateResult::InvalidFrameAddress(_) => None,
            TranslateResult::Frame4KiB { frame, offset } => Some(frame.start_address() + offset),
            TranslateResult::Frame2MiB { frame, offset } => Some(frame.start_address() + offset),
            TranslateResult::Frame1GiB { frame, offset } => Some(frame.start_address() + offset),
        }
    }
}

/// The return value of the [`MapperAllSizes::translate`] function.
///
/// If the given address has a valid mapping, a `Frame4KiB`, `Frame2MiB`, or `Frame1GiB` variant
/// is returned, depending on the size of the mapped page. The remaining variants indicate errors.
#[derive(Debug)]
pub enum TranslateResult {
    /// The page is mapped to a physical frame of size 4KiB.
    Frame4KiB {
        /// The mapped frame.
        frame: PhysFrame<Size4KiB>,
        /// The offset whithin the mapped frame.
        offset: u64,
    },
    /// The page is mapped to a physical frame of size 2MiB.
    Frame2MiB {
        /// The mapped frame.
        frame: PhysFrame<Size2MiB>,
        /// The offset whithin the mapped frame.
        offset: u64,
    },
    /// The page is mapped to a physical frame of size 2MiB.
    Frame1GiB {
        /// The mapped frame.
        frame: PhysFrame<Size1GiB>,
        /// The offset whithin the mapped frame.
        offset: u64,
    },
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// The page table entry for the given page points to an invalid physical address.
    InvalidFrameAddress(PhysAddr),
}

/// A trait for common page table operations on pages of size `S`.
pub trait Mapper<S: PageSize> {
    /// Creates a new mapping in the page table.
    ///
    /// This function might need additional physical frames to create new page tables. These
    /// frames are allocated from the `allocator` argument. At most three frames are required.
    ///
    /// This function is unsafe because the caller must guarantee that passed `frame` is
    /// unused, i.e. not used for any other mappings.
    unsafe fn map_to<A>(
        &mut self,
        page: Page<S>,
        frame: PhysFrame<S>,
        flags: PageTableFlags,
        attr: PageTableAttribute,
        frame_allocator: &mut A,
    ) -> Result<MapperFlush<S>, MapToError>
    where
        A: FrameAllocator<Size4KiB>;

    /// Get the reference of the specified `page` entry
    fn get_entry(&self, page: Page<S>) -> Result<&PageTableEntry, EntryGetError>;

    /// Get the mutable reference of the specified `page` entry
    fn get_entry_mut(&mut self, page: Page<S>) -> Result<&mut PageTableEntry, EntryGetError> {
        let entry = self.get_entry(page)?;
        let entry_mut = unsafe { &mut *(entry as *const _ as *mut PageTableEntry) };
        Ok(entry_mut)
    }

    /// Removes a mapping from the page table and returns the frame that used to be mapped.
    ///
    /// Note that no page tables or pages are deallocated.
    fn unmap(&mut self, page: Page<S>) -> Result<(PhysFrame<S>, MapperFlush<S>), UnmapError>;

    /// Updates the flags of an existing mapping.
    fn update_flags(
        &mut self,
        page: Page<S>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<S>, FlagUpdateError> {
        let entry = self.get_entry_mut(page)?;
        if entry.is_unused() {
            return Err(FlagUpdateError::PageNotMapped);
        }
        entry.set_flags(flags);
        Ok(MapperFlush::new(page))
    }

    /// Return the frame that the specified page is mapped to.
    ///
    /// This function assumes that the page is mapped to a frame of size `S` and returns an
    /// error otherwise.
    fn translate_page(&self, page: Page<S>) -> Result<PhysFrame<S>, TranslateError> {
        let entry = self.get_entry(page)?;
        if entry.is_unused() {
            return Err(TranslateError::PageNotMapped);
        }
        PhysFrame::from_start_address(entry.addr())
            .map_err(|()| TranslateError::InvalidFrameAddress(entry.addr()))
    }

    /// Maps the given frame to the virtual page with the same address.
    ///
    /// This function is unsafe because the caller must guarantee that the passed `frame` is
    /// unused, i.e. not used for any other mappings.
    unsafe fn identity_map<A>(
        &mut self,
        frame: PhysFrame<S>,
        flags: PageTableFlags,
        attr: PageTableAttribute,
        frame_allocator: &mut A,
    ) -> Result<MapperFlush<S>, MapToError>
    where
        A: FrameAllocator<Size4KiB>,
        S: PageSize,
        Self: Mapper<S>,
    {
        let page = Page::containing_address(VirtAddr::new(frame.start_address().as_u64()));
        self.map_to(page, frame, flags, attr, frame_allocator)
    }
}

/// This type represents a page whose mapping has changed in the page table.
///
/// The old mapping might be still cached in the translation lookaside buffer (TLB), so it needs
/// to be flushed from the TLB before it's accessed. This type is returned from function that
/// change the mapping of a page to ensure that the TLB flush is not forgotten.
#[derive(Debug)]
#[must_use = "Page Table changes must be flushed or ignored."]
pub struct MapperFlush<S: PageSize>(Page<S>);

impl<S: PageSize> MapperFlush<S> {
    /// Create a new flush promise
    fn new(page: Page<S>) -> Self {
        MapperFlush(page)
    }

    /// Flush the page from the TLB to ensure that the newest mapping is used.
    pub fn flush(self) {
        #[cfg(target_arch = "aarch64")]
        crate::asm::tlb_invalidate_all();
    }

    /// Don't flush the TLB and silence the “must be used” warning.
    pub fn ignore(self) {}
}

/// This error is returned from `map_to` and similar methods.
#[derive(Debug)]
pub enum MapToError {
    /// An additional frame was needed for the mapping process, but the frame allocator
    /// returned `None`.
    FrameAllocationFailed,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of an already mapped huge page.
    ParentEntryHugePage,
    /// The given page is already mapped to a physical frame.
    PageAlreadyMapped,
}

/// An error indicating that an `get_entry` or `get_entry_mut` call failed.
#[derive(Debug)]
pub enum EntryGetError {
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of a huge page and can't be freed individually.
    ParentEntryHugePage,
}

/// An error indicating that an `unmap` call failed.
#[derive(Debug)]
pub enum UnmapError {
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of a huge page and can't be freed individually.
    ParentEntryHugePage,
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// The page table entry for the given page points to an invalid physical address.
    InvalidFrameAddress(PhysAddr),
}

/// An error indicating that an `update_flags` call failed.
#[derive(Debug)]
pub enum FlagUpdateError {
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of a huge page and can't be freed individually.
    ParentEntryHugePage,
}

/// An error indicating that an `translate` call failed.
#[derive(Debug)]
pub enum TranslateError {
    /// The given page is not mapped to a physical frame.
    PageNotMapped,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of a huge page and can't be freed individually.
    ParentEntryHugePage,
    /// The page table entry for the given page points to an invalid physical address.
    InvalidFrameAddress(PhysAddr),
}

impl From<EntryGetError> for UnmapError {
    fn from(err: EntryGetError) -> Self {
        match err {
            EntryGetError::ParentEntryHugePage => UnmapError::ParentEntryHugePage,
            EntryGetError::PageNotMapped => UnmapError::PageNotMapped,
        }
    }
}

impl From<EntryGetError> for FlagUpdateError {
    fn from(err: EntryGetError) -> Self {
        match err {
            EntryGetError::ParentEntryHugePage => FlagUpdateError::ParentEntryHugePage,
            EntryGetError::PageNotMapped => FlagUpdateError::PageNotMapped,
        }
    }
}

impl From<EntryGetError> for TranslateError {
    fn from(err: EntryGetError) -> Self {
        match err {
            EntryGetError::ParentEntryHugePage => TranslateError::ParentEntryHugePage,
            EntryGetError::PageNotMapped => TranslateError::PageNotMapped,
        }
    }
}
