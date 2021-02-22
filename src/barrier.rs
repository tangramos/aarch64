// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Copyright (c) 2018-2021 by the author(s)
//
// Author(s):
//   - Andre Richter <andre.o.richter@gmail.com>

// Borrow implementations from the pending upstream ACLE implementation until it is merged.
// Afterwards, we'll probably just reexport them, hoping that the API doesn't change.
//
// https://github.com/rust-lang-nursery/stdsimd/pull/557

pub mod sealed {
    pub trait Dmb {
        unsafe fn __dmb(&self);
    }

    pub trait Dsb {
        unsafe fn __dsb(&self);
    }

    pub trait Isb {
        unsafe fn __isb(&self);
    }
}

macro_rules! dmb_dsb {
    ($A:ident) => {
        impl sealed::Dmb for $A {
            #[inline(always)]
            unsafe fn __dmb(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => {
                        asm!(concat!("DMB ", stringify!($A)), options(nostack))
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
        impl sealed::Dsb for $A {
            #[inline(always)]
            unsafe fn __dsb(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => {
                        asm!(concat!("DSB ", stringify!($A)), options(nostack))
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

// Full system
pub struct SY;
pub struct ST;
pub struct LD;

// Inner Shareable
pub struct ISH;
pub struct ISHST;
pub struct ISHLD;

// Non Shareable
pub struct NSH;
pub struct NSHST;
pub struct NSHLD;

dmb_dsb!(SY);
dmb_dsb!(ST);
dmb_dsb!(LD);

dmb_dsb!(ISH);
dmb_dsb!(ISHST);
dmb_dsb!(ISHLD);

dmb_dsb!(NSH);
dmb_dsb!(NSHST);
dmb_dsb!(NSHLD);

impl sealed::Isb for SY {
    #[inline(always)]
    unsafe fn __isb(&self) {
        match () {
            #[cfg(target_arch = "aarch64")]
            () => {
                asm!("ISB SY", options(nostack))
            }

            #[cfg(not(target_arch = "aarch64"))]
            () => unimplemented!(),
        }
    }
}

/// # Safety
///
/// In your own hands, this is hardware land!
#[inline(always)]
pub unsafe fn dmb<A>(arg: A)
where
    A: sealed::Dmb,
{
    arg.__dmb()
}

/// # Safety
///
/// In your own hands, this is hardware land!
#[inline(always)]
pub unsafe fn dsb<A>(arg: A)
where
    A: sealed::Dsb,
{
    arg.__dsb()
}

/// # Safety
///
/// In your own hands, this is hardware land!
#[inline(always)]
pub unsafe fn isb() {
    use self::sealed::Isb;
    SY.__isb()
}

/// Write memory barrier
#[inline(always)]
pub unsafe fn wmb() {
    dsb(ST)
}

/// Read memory barrier
#[inline(always)]
pub unsafe fn rmb() {
    dsb(LD)
}
