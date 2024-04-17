#![no_std]
#![no_main]
#![feature(const_trait_impl)]

#[macro_use]
extern crate nx;

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate alloc;

extern crate paste;

use nx::result::*;
use nx::util;
use nx::thread;
use nx::diag::abort;
use nx::diag::log;
use nx::ipc::server;
use nx::fs;
use core::panic;

#[macro_use]
mod logger;

mod rc;

#[macro_use]
mod fsext;

mod miiext;

mod ipc;

mod emu;

mod amiibo;

mod area;

mod rand;

rrt0_define_default_module_name!();

const CUSTOM_HEAP_SIZE: usize = 0x8000;
static mut CUSTOM_HEAP: [u8; CUSTOM_HEAP_SIZE] = [0; CUSTOM_HEAP_SIZE];

#[no_mangle]
pub fn initialize_heap(_hbl_heap: util::PointerAndSize) -> util::PointerAndSize {
    unsafe {
        util::PointerAndSize::new(CUSTOM_HEAP.as_mut_ptr(), CUSTOM_HEAP.len())
    }
}

#[no_mangle]
pub fn main() -> Result<()> {
    thread::get_current_thread().name.set_str("emuiibo.Main");
    fs::initialize_fspsrv_session()?;
    fs::mount_sd_card("sdmc")?;

    fsext::ensure_directories()?;
    logger::initialize()?;
    log!("Hello world!\n");

    rand::initialize()?;

    miiext::initialize()?;
    miiext::export_miis()?;

    amiibo::compat::convert_deprecated_virtual_amiibos();
    emu::load_emulation_status();

    ipc::nfp::initialize()?;

    const POINTER_BUF_SIZE: usize = 0x1000;
    type Manager = server::ServerManager<POINTER_BUF_SIZE>;

    let mut manager = Manager::new()?;
    manager.register_mitm_service_server::<ipc::nfp::user::UserManager>()?;
    manager.register_mitm_service_server::<ipc::nfp::sys::SystemManager>()?;
    manager.register_service_server::<ipc::emu::EmulationService>()?;
    manager.loop_process()?;

    miiext::finalize();
    rand::finalize();
    fs::finalize_fspsrv_session();
    fs::unmount_all();
    Ok(())
}

#[panic_handler]
fn panic_handler(info: &panic::PanicInfo) -> ! {
    util::simple_panic_handler::<log::lm::LmLogger>(info, abort::AbortLevel::SvcBreak())
}
