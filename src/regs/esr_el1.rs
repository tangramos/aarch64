//! Exception Syndrone Register - EL2

use register::cpu::RegisterReadWrite;

pub struct Reg;

impl RegisterReadWrite<u64, ()> for Reg {
    sys_coproc_read_raw!(u64, "ESR_EL1");
    sys_coproc_write_raw!(u64, "ESR_EL1");
}

pub static ESR_EL1: Reg = Reg {};
