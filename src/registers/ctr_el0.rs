// Copyright (c) 2018 by the author(s)
//
// =============================================================================
//
// Licensed under either of
//   - Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
//   - MIT License (http://opensource.org/licenses/MIT)
// at your option.
//
// =============================================================================
//
// Author(s):
//   - Yuekai Jia <equation618@gmail.com>

//! Cache Type Register
//!
//! Provides information about the architecture of the caches.

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};

register_bitfields! {u64,
    pub CTR_EL0 [
        /// Log2 of the number of words in the smallest cache line of all the
        /// data caches and unified caches that are controlled by the PE.
        DminLine OFFSET(16) NUMBITS(4) [],

        /// Level 1 instruction cache policy. Indicates the indexing and tagging
        /// policy for the L1 instruction cache. Possible values of this field are:
        ///
        /// 0b00 VMID aware Physical Index, Physical tag (VPIPT)
        /// 0b01 ASID-tagged Virtual Index, Virtual Tag (AIVIVT)
        /// 0b10 Virtual Index, Physical Tag (VIPT)
        /// 0b11 Physical Index, Physical Tag (PIPT)
        ///
        /// The value 0b01 is reserved in ARMv8.
        /// The value 0b00 is permitted only in an implementation that includes
        /// ARMv8.2-VPIPT, otherwise the value is reserved.
        L1Ip OFFSET(14) NUMBITS(2) [
            VPIPT = 0b00,
            AIVIVT = 0b01,
            VIPT = 0b10,
            PIPT = 0b11
        ],

        /// Log2 of the number of words in the smallest cache line of all the
        /// instruction caches that are controlled by the PE.
        IminLine OFFSET(0) NUMBITS(4) []
    ]
}

pub struct Reg;

impl Readable for Reg {
    type T = u64;
    type R = CTR_EL0::Register;

    sys_coproc_read_raw!(u64, "CTR_EL0", "x");
}

impl Writeable for Reg {
    type T = u64;
    type R = CTR_EL0::Register;

    sys_coproc_write_raw!(u64, "CTR_EL0", "x");
}

pub const CTR_EL0: Reg = Reg {};
