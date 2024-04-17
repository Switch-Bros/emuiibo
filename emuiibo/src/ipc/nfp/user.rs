use nx::ipc::sf::hid;
use nx::result::*;
use nx::mem;
use nx::ipc::sf;
use nx::ipc::server;
use nx::ipc::sf::applet;
use nx::ipc::sf::ncm;
use nx::ipc::sf::nfp;
use nx::ipc::sf::nfp::IUser;
use nx::ipc::sf::nfp::IUserManager;
use nx::ipc::sf::sm;

use crate::emu;
use super::EmulationHandler;

pub struct User {
    handler: EmulationHandler,
    dummy_session: sf::Session
}

impl User {
    pub fn new(application_id: ncm::ProgramId) -> Result<Self> {
        emu::register_intercepted_application_id(application_id);
        
        Ok(Self {
            handler: EmulationHandler::new(application_id)?,
            dummy_session: sf::Session::new()
        })
    }
}

impl Drop for User {
    fn drop(&mut self) {
        emu::unregister_intercepted_application_id(self.handler.get_application_id());
    }
}

impl sf::IObject for User {
    ipc_sf_object_impl_default_command_metadata!();

    fn get_session(&mut self) -> &mut sf::Session {
        &mut self.dummy_session
    }
}

impl IUser for User {
    fn initialize(&mut self, aruid: applet::AppletResourceUserId, process_id: sf::ProcessId, mcu_data: sf::InMapAliasBuffer<nfp::McuVersionData>) -> Result<()> {
        self.handler.initialize(aruid, process_id, mcu_data)
    }

    fn finalize(&mut self) -> Result<()> {
        self.handler.finalize()
    }

    fn list_devices(&mut self, out_devices: sf::OutPointerBuffer<nfp::DeviceHandle>) -> Result<u32> {
        self.handler.list_devices(out_devices)
    }

    fn start_detection(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        self.handler.start_detection(device_handle)
    }

    fn stop_detection(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        self.handler.stop_detection(device_handle)
    }

    fn mount(&mut self, device_handle: nfp::DeviceHandle, model_type: nfp::ModelType, mount_target: nfp::MountTarget) -> Result<()> {
        self.handler.mount(device_handle, model_type, mount_target)
    }

    fn unmount(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        self.handler.unmount(device_handle)
    }

    fn open_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId) -> Result<()> {
        self.handler.open_application_area(device_handle, access_id)
    }

    fn get_application_area(&mut self, device_handle: nfp::DeviceHandle, out_data: sf::OutMapAliasBuffer<u8>) -> Result<u32> {
        self.handler.get_application_area(device_handle, out_data)
    }

    fn set_application_area(&mut self, device_handle: nfp::DeviceHandle, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        self.handler.set_application_area(device_handle, data)
    }

    fn flush(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        self.handler.flush(device_handle)
    }

    fn restore(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        self.handler.restore(device_handle)
    }

    fn create_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        self.handler.create_application_area(device_handle, access_id, data)
    }

    fn get_tag_info(&mut self, device_handle: nfp::DeviceHandle, out_tag_info: sf::OutFixedPointerBuffer<nfp::TagInfo>) -> Result<()> {
        self.handler.get_tag_info(device_handle, out_tag_info)
    }

    fn get_register_info(&mut self, device_handle: nfp::DeviceHandle, out_register_info: sf::OutFixedPointerBuffer<nfp::RegisterInfo>) -> Result<()> {
        self.handler.get_register_info(device_handle, out_register_info)
    }

    fn get_common_info(&mut self, device_handle: nfp::DeviceHandle, out_common_info: sf::OutFixedPointerBuffer<nfp::CommonInfo>) -> Result<()> {
        self.handler.get_common_info(device_handle, out_common_info)
    }

    fn get_model_info(&mut self, device_handle: nfp::DeviceHandle, out_model_info: sf::OutFixedPointerBuffer<nfp::ModelInfo>) -> Result<()> {
        self.handler.get_model_info(device_handle, out_model_info)
    }

    fn attach_activate_event(&mut self, device_handle: nfp::DeviceHandle) -> Result<sf::CopyHandle> {
        self.handler.attach_activate_event(device_handle)
    }

    fn attach_deactivate_event(&mut self, device_handle: nfp::DeviceHandle) -> Result<sf::CopyHandle> {
        self.handler.attach_deactivate_event(device_handle)
    }

    fn get_state(&mut self) -> Result<nfp::State> {
        self.handler.get_state()
    }

    fn get_device_state(&mut self, device_handle: nfp::DeviceHandle) -> Result<nfp::DeviceState> {
        self.handler.get_device_state(device_handle)
    }

    fn get_npad_id(&mut self, device_handle: nfp::DeviceHandle) -> Result<hid::NpadIdType> {
        self.handler.get_npad_id(device_handle)
    }

    fn get_application_area_size(&mut self, device_handle: nfp::DeviceHandle) -> Result<u32> {
        self.handler.get_application_area_size(device_handle)
    }

    fn attach_availability_change_event(&mut self) -> Result<sf::CopyHandle> {
        self.handler.attach_availability_change_event()
    }

    fn recreate_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        self.handler.recreate_application_area(device_handle, access_id, data)
    }
}

impl server::ISessionObject for User {}

pub struct UserManager {
    info: sm::mitm::MitmProcessInfo,
    dummy_session: sf::Session
}

impl sf::IObject for UserManager {
    ipc_sf_object_impl_default_command_metadata!();

    fn get_session(&mut self) -> &mut sf::Session {
        &mut self.dummy_session
    }
}

impl IUserManager for UserManager {
    fn create_user_interface(&mut self) -> Result<mem::Shared<dyn IUser>> {
        let user = User::new(self.info.program_id)?;
        Ok(mem::Shared::new(user))
    }
}

impl server::ISessionObject for UserManager {}

impl server::IMitmServerObject for UserManager {
    fn new(info: sm::mitm::MitmProcessInfo) -> Self {
        Self {
            info,
            dummy_session: sf::Session::new()
        }
    }
}

impl server::IMitmService for UserManager {
    fn get_name() -> sm::ServiceName {
        sm::ServiceName::new("nfp:user")
    }

    fn should_mitm(_info: sm::mitm::MitmProcessInfo) -> bool {
        emu::get_emulation_status() == emu::EmulationStatus::On
    }
}