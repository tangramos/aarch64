use crate::{
    barrier::{dsb, isb, sealed},
    regs::*,
};
use core::marker::PhantomData;

pub use crate::barrier::{ISH, NSH, SY};

pub trait CoherencyPoint {}

/// Point of Coherency, all agents in the systesm see the same copy of memory
pub struct PoC;

/// Point of Unification, IC, DC, TTB of all PEs in the ISH domain see the same
///  copy of memory.
pub struct PoU;

impl CoherencyPoint for PoU {}
impl CoherencyPoint for PoC {}

pub trait Flush {}

/// Flush the data written to the cache into memory
pub struct Clean;

/// Invalidate old data in the cache
pub struct Invalidate;

/// A clean instruction followed by a invalidate instruction
pub struct CleanAndInvalidate;

impl Flush for Clean {}
impl Flush for Invalidate {}
impl Flush for CleanAndInvalidate {}

pub trait Cache {
    /// Flush a cache line by the virtual address.
    fn flush_line_op(vaddr: usize);
    /// Cache line size in bytes
    fn cache_line_size() -> u64;

    /// Flush cache for the VA interval [start, end) in the shareability domain.
    fn flush_range<A: sealed::Dsb>(start: usize, end: usize, domain: A) {
        let line_size = 4 << Self::cache_line_size();
        let mut addr = start & !(line_size - 1);
        while addr < end {
            Self::flush_line_op(addr);
            addr += line_size;
        }
        unsafe { dsb(domain) };
        unsafe { isb() };
    }

    /// Flush cache for the VA interval [start, start + sizeend) in the
    /// shareability domain.
    fn flush_area<A: sealed::Dsb>(start: usize, size: usize, domain: A) {
        Self::flush_range(start, start + size, domain);
    }
}

pub struct ICache<F: Flush = Invalidate, P: CoherencyPoint = PoU> {
    _f: PhantomData<F>,
    _p: PhantomData<P>,
}

pub struct DCache<F: Flush, P: CoherencyPoint> {
    _f: PhantomData<F>,
    _p: PhantomData<P>,
}

impl ICache {
    /// Invalidate all I-Cache to the Point of Unification in all PEs.
    #[inline]
    pub fn flush_all() {
        unsafe { llvm_asm!("ic ialluis; dsb ish; isb":::: "volatile") };
    }
    /// Invalidate all I-Cache to the Point of Unification in the current PE.
    #[inline]
    pub fn local_flush_all() {
        unsafe { llvm_asm!("ic iallu; dsb nsh; isb":::: "volatile") };
    }
}

macro_rules! cache_ins {
    (ICache) => {
        "ic"
    };
    (DCache) => {
        "dc"
    };
}

macro_rules! cache_op {
    (Clean) => {
        "c"
    };
    (Invalidate) => {
        "i"
    };
    (CleanAndInvalidate) => {
        "ci"
    };
}

macro_rules! cache_point {
    (PoC) => {
        "c"
    };
    (PoU) => {
        "u"
    };
}

macro_rules! cache_line_size {
    (ICache) => {
        CTR_EL0::IminLine
    };
    (DCache) => {
        CTR_EL0::DminLine
    };
}

macro_rules! define_cache_op {
    ($cache:ident, $flush:ident, $point:ident) => {
        impl Cache for $cache<$flush, $point> {
            #[inline]
            fn flush_line_op(vaddr: usize) {
                unsafe {
                    llvm_asm!(concat!(
                            cache_ins!($cache),
                            " ",
                            cache_op!($flush),
                            "va",
                            cache_point!($point),
                            ", $0"
                        ) :: "r"(vaddr) : "memory" : "volatile");
                }
            }
            #[inline]
            fn cache_line_size() -> u64 {
                CTR_EL0.read(cache_line_size!($cache))
            }
        }
    };
}

define_cache_op!(ICache, Invalidate, PoU);
define_cache_op!(DCache, Clean, PoU);
define_cache_op!(DCache, Clean, PoC);
define_cache_op!(DCache, Invalidate, PoC);
define_cache_op!(DCache, CleanAndInvalidate, PoC);

/// Level 1 instruction cache policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L1ICachePolicy {
    VIPT = 0b10,
    PIPT = 0b11,
    Unsupport,
}

/// Get the level 1 instruction cache policy (VIPT or PIPT), indicates the
/// indexing and tagging policy for the L1 instruction cache.
#[inline]
pub fn get_l1_icache_policy() -> L1ICachePolicy {
    use self::L1ICachePolicy::*;
    match CTR_EL0.read(CTR_EL0::L1Ip) {
        0b10 => VIPT,
        0b11 => PIPT,
        _ => Unsupport,
    }
}
