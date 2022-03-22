//! Memory region attributes (D4.5, page 2174)

use crate::{
    paging::page_table::{PageTableAttribute, MEMORY_ATTRIBUTE},
    registers::*,
};
use tock_registers::fields::FieldValue;

pub trait MairType {
    const INDEX: u64;

    fn config_value() -> FieldValue<u64, MAIR_EL1::Register>;

    fn attr_value() -> PageTableAttribute;
}

pub enum MairDevice {}
pub enum MairNormal {}
pub enum MairNormalNonCacheable {}

impl MairType for MairNormal {
    const INDEX: u64 = 0;

    #[inline]
    fn config_value() -> FieldValue<u64, MAIR_EL1::Register> {
        MAIR_EL1::Attr0_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc
            + MAIR_EL1::Attr0_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::InnerShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}

impl MairType for MairDevice {
    const INDEX: u64 = 1;

    #[inline]
    fn config_value() -> FieldValue<u64, MAIR_EL1::Register> {
        MAIR_EL1::Attr1_Device::nonGathering_nonReordering_EarlyWriteAck
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::OuterShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}

impl MairType for MairNormalNonCacheable {
    const INDEX: u64 = 2;

    #[inline]
    fn config_value() -> FieldValue<u64, MAIR_EL1::Register> {
        MAIR_EL1::Attr2_Normal_Outer::NonCacheable + MAIR_EL1::Attr2_Normal_Inner::NonCacheable
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::OuterShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}
