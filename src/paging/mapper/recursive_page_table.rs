//! Access the page tables through a recursively mapped level 4 table.

use crate::paging::{
    frame::PhysFrame,
    frame_alloc::FrameAllocator,
    mapper::*,
    memory_attribute::*,
    page::{NotGiantPageSize, Page, PageSize, Size4KiB},
    page_table::{FrameError, PageTable, PageTableAttribute, PageTableEntry, PageTableFlags},
};
use ux::u9;

/// A recursive page table is a last level page table with an entry mapped to the table itself.
///
/// This recursive mapping allows accessing all page tables in the hierarchy:
///
/// - To access the level 4 page table, we “loop“ (i.e. follow the recursively mapped entry) four
///   times.
/// - To access a level 3 page table, we “loop” three times and then use the level 4 index.
/// - To access a level 2 page table, we “loop” two times, then use the level 4 index, then the
///   level 3 index.
/// - To access a level 1 page table, we “loop” once, then use the level 4 index, then the
///   level 3 index, then the level 2 index.
///
/// This struct implements the `Mapper` trait.
#[derive(Debug)]
pub struct RecursivePageTable {
    recursive_index: u9,
}

impl RecursivePageTable {
    /// Creates a new RecursivePageTable without performing any checks.
    ///
    /// The `recursive_index` parameter must be the index of the recursively mapped entry.
    pub fn new(recursive_index: u16) -> Self {
        RecursivePageTable {
            recursive_index: u9::new(recursive_index),
        }
    }

    /// Internal helper function to create the page table of the next level if needed.
    ///
    /// If the passed entry is unused, a new frame is allocated from the given allocator, zeroed,
    /// and the entry is updated to that address. If the passed entry is already mapped, the next
    /// table is returned directly.
    ///
    /// The `next_page_table` page must be the page of the next page table in the hierarchy.
    ///
    /// Returns `MapToError::FrameAllocationFailed` if the entry is unused and the allocator
    /// returned `None`. Returns `MapToError::ParentEntryHugePage` if the `HUGE_PAGE` flag is set
    /// in the passed entry.
    unsafe fn create_next_table<'b, A>(
        entry: &'b mut PageTableEntry,
        next_table_page: Page,
        allocator: &mut A,
    ) -> Result<&'b mut PageTable, MapToError>
    where
        A: FrameAllocator<Size4KiB>,
    {
        /// This inner function is used to limit the scope of `unsafe`.
        ///
        /// This is a safe function, so we need to use `unsafe` blocks when we do something unsafe.
        fn inner<'b, A>(
            entry: &'b mut PageTableEntry,
            next_table_page: Page,
            allocator: &mut A,
        ) -> Result<&'b mut PageTable, MapToError>
        where
            A: FrameAllocator<Size4KiB>,
        {
            let created;

            if entry.is_unused() {
                if let Some(frame) = allocator.allocate_frame() {
                    entry.set_frame(frame, PageTableFlags::default(), MairNormal::attr_value());
                    created = true;
                } else {
                    return Err(MapToError::FrameAllocationFailed);
                }
            } else {
                created = false;
            }
            // is a huge page (block)
            if entry.is_block() {
                return Err(MapToError::ParentEntryHugePage);
            }

            let page_table_ptr = next_table_page.start_address().as_mut_ptr();
            let page_table: &mut PageTable = unsafe { &mut *(page_table_ptr) };
            if created {
                #[cfg(target_arch = "aarch64")]
                unsafe {
                    crate::barrier::dsb(crate::barrier::ISHST);
                }
                page_table.zero();
            }
            Ok(page_table)
        }

        inner(entry, next_table_page, allocator)
    }

    fn p4_ptr<S: PageSize>(&self, page: Page<S>) -> *mut PageTable {
        self.p4_page(page).start_address().as_mut_ptr()
    }

    fn p3_ptr<S: PageSize>(&self, page: Page<S>) -> *mut PageTable {
        self.p3_page(page).start_address().as_mut_ptr()
    }

    fn p2_ptr<S: NotGiantPageSize>(&self, page: Page<S>) -> *mut PageTable {
        self.p2_page(page).start_address().as_mut_ptr()
    }

    fn p1_ptr(&self, page: Page<Size4KiB>) -> *mut PageTable {
        self.p1_page(page).start_address().as_mut_ptr()
    }

    fn p4_page<S: PageSize>(&self, page: Page<S>) -> Page {
        Page::from_page_table_indices(
            page.va_range().unwrap(),
            self.recursive_index,
            self.recursive_index,
            self.recursive_index,
            self.recursive_index,
        )
    }

    fn p3_page<S: PageSize>(&self, page: Page<S>) -> Page {
        Page::from_page_table_indices(
            page.va_range().unwrap(),
            self.recursive_index,
            self.recursive_index,
            self.recursive_index,
            page.p4_index(),
        )
    }

    fn p2_page<S: NotGiantPageSize>(&self, page: Page<S>) -> Page {
        Page::from_page_table_indices(
            page.va_range().unwrap(),
            self.recursive_index,
            self.recursive_index,
            page.p4_index(),
            page.p3_index(),
        )
    }

    fn p1_page(&self, page: Page<Size4KiB>) -> Page {
        Page::from_page_table_indices(
            page.va_range().unwrap(),
            self.recursive_index,
            page.p4_index(),
            page.p3_index(),
            page.p2_index(),
        )
    }
}

impl Mapper<Size4KiB> for RecursivePageTable {
    unsafe fn map_to<A>(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
        attr: PageTableAttribute,
        allocator: &mut A,
    ) -> Result<MapperFlush<Size4KiB>, MapToError>
    where
        A: FrameAllocator<Size4KiB>,
    {
        let p4 = &mut *(self.p4_ptr(page));

        let p3_page = self.p3_page(page);
        let p3 = Self::create_next_table(&mut p4[page.p4_index()], p3_page, allocator)?;

        let p2_page = self.p2_page(page);
        let p2 = Self::create_next_table(&mut p3[page.p3_index()], p2_page, allocator)?;

        let p1_page = self.p1_page(page);
        let p1 = Self::create_next_table(&mut p2[page.p2_index()], p1_page, allocator)?;

        if !p1[page.p1_index()].is_unused() {
            return Err(MapToError::PageAlreadyMapped);
        }
        p1[page.p1_index()].set_frame(frame, flags, attr);

        Ok(MapperFlush::new(page))
    }

    fn get_entry(&self, page: Page<Size4KiB>) -> Result<&PageTableEntry, EntryGetError> {
        let p4 = unsafe { &mut *(self.p4_ptr(page)) };

        if p4[page.p4_index()].is_unused() {
            return Err(EntryGetError::PageNotMapped);
        }

        let p3 = unsafe { &mut *(self.p3_ptr(page)) };

        if p3[page.p3_index()].is_unused() {
            return Err(EntryGetError::PageNotMapped);
        }

        let p2 = unsafe { &mut *(self.p2_ptr(page)) };

        if p2[page.p2_index()].is_unused() {
            return Err(EntryGetError::PageNotMapped);
        }

        let p1 = unsafe { &mut *(self.p1_ptr(page)) };

        Ok(&p1[page.p1_index()])
    }

    fn unmap(
        &mut self,
        page: Page<Size4KiB>,
    ) -> Result<(PhysFrame<Size4KiB>, MapperFlush<Size4KiB>), UnmapError> {
        let p4 = unsafe { &mut *(self.p4_ptr(page)) };

        let p4_entry = &p4[page.p4_index()];
        p4_entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugeFrame => UnmapError::ParentEntryHugePage,
        })?;

        let p3 = unsafe { &mut *(self.p3_ptr(page)) };
        let p3_entry = &p3[page.p3_index()];
        p3_entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugeFrame => UnmapError::ParentEntryHugePage,
        })?;

        let p2 = unsafe { &mut *(self.p2_ptr(page)) };
        let p2_entry = &p2[page.p2_index()];
        p2_entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugeFrame => UnmapError::ParentEntryHugePage,
        })?;

        let p1 = unsafe { &mut *(self.p1_ptr(page)) };
        let p1_entry = &mut p1[page.p1_index()];

        let frame = p1_entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugeFrame => UnmapError::ParentEntryHugePage,
        })?;

        p1_entry.set_unused();
        Ok((frame, MapperFlush::new(page)))
    }
}
