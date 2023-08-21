use nx::sync;
use alloc::vec::Vec;
use crate::amiibo;
use crate::fsext;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
    pub is_dev_build: bool
}

impl Version {
    pub const fn from(major: u8, minor: u8, micro: u8, is_dev_build: bool) -> Self {
        Self { major: major, minor: minor, micro: micro, is_dev_build: is_dev_build }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum EmulationStatus {
    On,
    Off
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
#[allow(dead_code)]
pub enum VirtualAmiiboStatus {
    Invalid,
    Connected,
    Disconnected
}

#[cfg(debug_assertions)]
pub const IS_DEV_BUILD: bool = true;

#[cfg(not(debug_assertions))]
pub const IS_DEV_BUILD: bool = false;

pub const CURRENT_VERSION: Version = Version::from(1, 1, 0, IS_DEV_BUILD);

static mut G_EMULATION_STATUS: sync::Locked<EmulationStatus> = sync::Locked::new(false, EmulationStatus::Off);
static mut G_ACTIVE_VIRTUAL_AMIIBO_STATUS: sync::Locked<VirtualAmiiboStatus> = sync::Locked::new(false, VirtualAmiiboStatus::Invalid);
static mut G_INTERCEPTED_APPLICATION_IDS: sync::Locked<Vec<u64>> = sync::Locked::new(false, Vec::new());
static mut G_ACTIVE_VIRTUAL_AMIIBO: sync::Locked<amiibo::fmt::VirtualAmiibo> = sync::Locked::new(false, amiibo::fmt::VirtualAmiibo::empty());

const STATUS_ON_FLAG: &str = "status_on";

pub fn get_emulation_status() -> EmulationStatus {
    unsafe {
        G_EMULATION_STATUS.get_val()
    }
}

pub fn load_emulation_status() {
    let status = if fsext::has_flag(STATUS_ON_FLAG) {
        EmulationStatus::On
    }
    else {
        EmulationStatus::Off
    };
    
    unsafe {
        G_EMULATION_STATUS.set(status);
    }
}

pub fn set_emulation_status(status: EmulationStatus) {
    unsafe {
        G_EMULATION_STATUS.set(status);
    }

    fsext::set_flag(STATUS_ON_FLAG, status == EmulationStatus::On);
}

pub fn get_active_virtual_amiibo_status() -> VirtualAmiiboStatus {
    unsafe {
        G_ACTIVE_VIRTUAL_AMIIBO_STATUS.get_val()
    }
}

pub fn set_active_virtual_amiibo_status(status: VirtualAmiiboStatus) {
    unsafe {
        G_ACTIVE_VIRTUAL_AMIIBO_STATUS.set(status);
    }
}

pub fn register_intercepted_application_id(application_id: u64) {
    unsafe {
        G_INTERCEPTED_APPLICATION_IDS.get().push(application_id);
    }
}

pub fn unregister_intercepted_application_id(application_id: u64) {
    unsafe {
        G_INTERCEPTED_APPLICATION_IDS.get().retain(|&id| id != application_id);
    }
}

pub fn is_application_id_intercepted(application_id: u64) -> bool {
    unsafe {
        G_INTERCEPTED_APPLICATION_IDS.get().contains(&application_id)
    }
}

pub fn get_active_virtual_amiibo() -> &'static mut amiibo::fmt::VirtualAmiibo {
    unsafe {
        G_ACTIVE_VIRTUAL_AMIIBO.get()
    }
}

pub fn set_active_virtual_amiibo(virtual_amiibo: amiibo::fmt::VirtualAmiibo) {
    unsafe {
        G_ACTIVE_VIRTUAL_AMIIBO.set(virtual_amiibo);
        set_active_virtual_amiibo_status(VirtualAmiiboStatus::Connected);
    }
}