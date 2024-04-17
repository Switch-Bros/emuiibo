use nx::result::*;
use nx::ipc::sf;
use nx::ipc::sf::nfp;
use nx::ipc::sf::ncm;
use nx::ipc::sf::applet;
use nx::wait;
use nx::sync;
use nx::service::hid;
use nx::input;
use nx::thread;
use crate::area;
use crate::emu;

static mut G_INPUT_CTX: Option<input::Context> = None;

pub fn initialize() -> Result<()> {
    let supported_tags = hid::NpadStyleTag::FullKey() | hid::NpadStyleTag::Handheld() | hid::NpadStyleTag::JoyDual() | hid::NpadStyleTag::JoyLeft() | hid::NpadStyleTag::JoyRight();
    let supported_npad_ids = [hid::NpadIdType::No1, hid::NpadIdType::Handheld];
    unsafe {
        G_INPUT_CTX = Some(input::Context::new(supported_tags, &supported_npad_ids)?);
    }
    Ok(())
}

pub fn get_input_context() -> &'static input::Context {
    unsafe {
        G_INPUT_CTX.as_ref().unwrap()
    }
}

pub struct EmulationHandler {
    application_id: ncm::ProgramId,
    activate_event: wait::SystemEvent,
    deactivate_event: wait::SystemEvent,
    availability_change_event: wait::SystemEvent,
    state: sync::Locked<nfp::State>,
    device_state: sync::Locked<nfp::DeviceState>,
    should_end_thread: sync::Locked<bool>,
    current_opened_area: area::ApplicationArea,
    emu_handler_thread: thread::Thread
}

impl EmulationHandler {
    pub fn new(application_id: ncm::ProgramId) -> Result<Self> {
        log!("\n[{:#X}] New handler!\n", application_id.0);
        Ok(Self { application_id, activate_event: wait::SystemEvent::new()?, deactivate_event: wait::SystemEvent::new()?, availability_change_event: wait::SystemEvent::new()?, state: sync::Locked::new(false, nfp::State::NonInitialized), device_state: sync::Locked::new(false, nfp::DeviceState::Unavailable), should_end_thread: sync::Locked::new(false, false), emu_handler_thread: thread::Thread::empty(), current_opened_area: area::ApplicationArea::new() })
    }

    #[inline]
    pub fn get_application_id(&self) -> ncm::ProgramId {
        self.application_id
    }

    pub fn is_state(&mut self, state: nfp::State) -> bool {
        self.state.get_val() == state
    }

    pub fn is_device_state(&mut self, device_state: nfp::DeviceState) -> bool {
        self.device_state.get_val() == device_state
    }

    fn handle_virtual_amiibo_status(&mut self, status: emu::VirtualAmiiboStatus) {
        match status {
            emu::VirtualAmiiboStatus::Connected => match self.device_state.get_val() {
                nfp::DeviceState::SearchingForTag => {
                    self.device_state.set(nfp::DeviceState::TagFound);
                    self.activate_event.signal().unwrap();
                },
                _ => {}
            },
            emu::VirtualAmiiboStatus::Disconnected => match self.device_state.get_val() {
                nfp::DeviceState::TagFound | nfp::DeviceState::TagMounted => {
                    self.device_state.set(nfp::DeviceState::SearchingForTag);
                    self.deactivate_event.signal().unwrap();
                },
                _ => {}
            },
            _ => {}
        };
    }

    fn emu_handler_thread_fn(handler: &*mut EmulationHandler) {
        unsafe {
            loop {
                if (*(*handler)).should_end_thread.get_val() {
                    break;
                }
                
                let status = emu::get_active_virtual_amiibo_status();
                (*(*handler)).handle_virtual_amiibo_status(status);
                let _ = thread::sleep(100_000_000);
            }
        }
    }

    pub fn initialize(&mut self, aruid: applet::AppletResourceUserId, process_id: sf::ProcessId, mcu_data: sf::InMapAliasBuffer<nfp::McuVersionData>) -> Result<()> {
        // TODO: make use of aruid or mcu data?
        result_return_unless!(self.is_state(nfp::State::NonInitialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Initialize -- aruid: {}, process_id: {}, mcu_version_data: (count: {})\n", self.application_id.0, aruid, process_id.process_id, mcu_data.get_count());
        let mcu_ver_datas = mcu_data.get_slice();
        for mcu_ver_data in mcu_ver_datas {
            log!("[{:#X}] Initialize -- mcu version: {}\n", self.application_id.0, mcu_ver_data.version);
        }

        self.state.set(nfp::State::Initialized);
        self.device_state.set(nfp::DeviceState::Initialized);

        // TODO: different thread names depending on the app id?
        self.emu_handler_thread = thread::Thread::new(Self::emu_handler_thread_fn, &(self as *mut Self), "emuiibo.AmiiboEmulationHandler", 0x1000)?;
        self.emu_handler_thread.initialize(0x2B, -2)?;
        self.emu_handler_thread.start()?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Finalize -- (...)\n", self.application_id.0);

        self.state.set(nfp::State::NonInitialized);
        self.device_state.set(nfp::DeviceState::Finalized);
        Ok(())
    }

    pub fn list_devices(&mut self, out_devices: sf::OutPointerBuffer<nfp::DeviceHandle>) -> Result<u32> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] ListDevices -- out_devices: (count: {})\n", self.application_id.0, out_devices.get_count());

        // Note: a DeviceHandle's id != npad_id on official nfp, but we treat them as the same thing since we don't care about it
        // Official nfp would store the npad_id somewhere else for the command below which retrieves it
        // Send a single fake device handle

        let fake_device_npad_id = {
            let p1 = get_input_context().get_player(hid::NpadIdType::No1);
            if p1.get_style_tag_attributes(hid::NpadStyleTag::FullKey()).contains(hid::NpadAttribute::IsConnected())
                || p1.get_style_tag_attributes(hid::NpadStyleTag::JoyDual()).contains(hid::NpadAttribute::IsConnected())
                || p1.get_style_tag_attributes(hid::NpadStyleTag::JoyLeft()).contains(hid::NpadAttribute::IsConnected()) 
                || p1.get_style_tag_attributes(hid::NpadStyleTag::JoyRight()).contains(hid::NpadAttribute::IsConnected()) {
                hid::NpadIdType::No1
            }
            else {
                hid::NpadIdType::Handheld
            }
        };

        let devices = out_devices.get_mut_slice();
        devices[0].id = fake_device_npad_id as u32;
        Ok(1)
    }

    pub fn start_detection(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::Initialized) || self.is_device_state(nfp::DeviceState::TagRemoved), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] StartDetection -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        self.device_state.set(nfp::DeviceState::SearchingForTag);
        Ok(())
    }

    pub fn stop_detection(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] StopDetection -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        self.device_state.set(nfp::DeviceState::Initialized);
        Ok(())
    }

    pub fn mount(&mut self, device_handle: nfp::DeviceHandle, model_type: nfp::ModelType, mount_target: nfp::MountTarget) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Mount -- device_handle: (fake id: {}), model_type: {:?}, mount_target: {:?}\n", self.application_id.0, device_handle.id, model_type, mount_target);
        
        self.device_state.set(nfp::DeviceState::TagMounted);
        Ok(())
    }

    pub fn unmount(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Unmount -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);
        
        self.device_state.set(nfp::DeviceState::TagFound);
        Ok(())
    }

    pub fn open_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] OpenApplicationArea -- device_handle: (fake id: {}), access_id: {:#X}\n", self.application_id.0, device_handle.id, access_id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let application_area = area::ApplicationArea::from_id(&amiibo, self.application_id.0, access_id);
        result_return_unless!(application_area.exists(), nfp::rc::ResultAreaNeedsToBeCreated);

        amiibo.update_area_program_id(access_id, self.application_id)?;
        self.current_opened_area = application_area;
        Ok(())
    }

    pub fn get_application_area(&mut self, device_handle: nfp::DeviceHandle, out_data: sf::OutMapAliasBuffer<u8>) -> Result<u32> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.current_opened_area.exists(), nfp::rc::ResultAreaNeedsToBeCreated);
        log!("[{:#X}] GetApplicationArea -- device_handle: (fake id: {}), out_data: (buf_size: {:#X})\n", self.application_id.0, device_handle.id, out_data.get_size());

        let area_size = self.current_opened_area.get_size()?;
        let size = core::cmp::min(area_size, out_data.get_size());
        
        self.current_opened_area.read(out_data.get_address(), size)?;
        Ok(size as u32)
    }

    pub fn set_application_area(&mut self, device_handle: nfp::DeviceHandle, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.current_opened_area.exists(), nfp::rc::ResultAreaNeedsToBeCreated);
        log!("[{:#X}] SetApplicationArea -- device_handle: (fake id: {}), data: (buf_size: {:#X})\n", self.application_id.0, device_handle.id, data.get_size());

        let area_size = self.current_opened_area.get_size()?;
        let size = core::cmp::min(area_size, data.get_size());

        self.current_opened_area.write(data.get_address(), size)?;
        let amiibo = emu::get_active_virtual_amiibo();
        amiibo.notify_written()?;
        Ok(())
    }

    pub fn flush(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Flush -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        Ok(())
    }

    pub fn restore(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] Restore -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        Ok(())
    }

    pub fn create_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] CreateApplicationArea -- device_handle: (fake id: {}), access_id: {:#X}, data: (buf_size: {:#X})\n", self.application_id.0, device_handle.id, access_id, data.get_size());

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let application_area = area::ApplicationArea::from_id(&amiibo, self.application_id.0, access_id);
        result_return_if!(application_area.exists(), nfp::rc::ResultAreaNeedsToBeCreated);

        application_area.create(data.get_address(), data.get_size(), false)?;
        amiibo.notify_written()?;
        Ok(())
    }

    pub fn get_tag_info(&mut self, device_handle: nfp::DeviceHandle, mut out_tag_info: sf::OutFixedPointerBuffer<nfp::TagInfo>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagFound) || self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetTagInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let tag_info = amiibo.produce_tag_info()?;
        out_tag_info.set_var(tag_info);
        Ok(())
    }

    pub fn get_register_info(&mut self, device_handle: nfp::DeviceHandle, mut out_register_info: sf::OutFixedPointerBuffer<nfp::RegisterInfo>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetRegisterInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let register_info = amiibo.produce_register_info()?;
        out_register_info.set_var(register_info);
        Ok(())
    }

    pub fn get_common_info(&mut self, device_handle: nfp::DeviceHandle, mut out_common_info: sf::OutFixedPointerBuffer<nfp::CommonInfo>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetCommonInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let common_info = amiibo.produce_common_info()?;
        out_common_info.set_var(common_info);
        Ok(())
    }

    pub fn get_model_info(&mut self, device_handle: nfp::DeviceHandle, mut out_model_info: sf::OutFixedPointerBuffer<nfp::ModelInfo>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetModelInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let model_info = amiibo.produce_model_info()?;
        out_model_info.set_var(model_info);
        Ok(())
    }

    pub fn attach_activate_event(&mut self, device_handle: nfp::DeviceHandle) -> Result<sf::CopyHandle> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] AttachActivateEvent -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        Ok(sf::Handle::from(self.activate_event.client_handle))
    }

    pub fn attach_deactivate_event(&mut self, device_handle: nfp::DeviceHandle) -> Result<sf::CopyHandle> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] AttachDeactivateEvent -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        Ok(sf::Handle::from(self.deactivate_event.client_handle))
    }

    pub fn get_state(&mut self) -> Result<nfp::State> {
        log!("[{:#X}] GetState -- (...)\n", self.application_id.0);
        Ok(self.state.get_val())
    }

    pub fn get_device_state(&mut self, device_handle: nfp::DeviceHandle) -> Result<nfp::DeviceState> {
        log!("[{:#X}] GetDeviceState -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);
        Ok(self.device_state.get_val())
    }

    pub fn get_npad_id(&mut self, device_handle: nfp::DeviceHandle) -> Result<hid::NpadIdType> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetNpadId -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);
        
        Ok(unsafe { core::mem::transmute(device_handle.id) })
    }

    pub fn get_application_area_size(&mut self, device_handle: nfp::DeviceHandle) -> Result<u32> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.current_opened_area.exists(), nfp::rc::ResultAreaNeedsToBeCreated);
        log!("[{:#X}] GetApplicationAreaSize -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let area_size = self.current_opened_area.get_size()?;
        Ok(area_size as u32)
    }

    pub fn attach_availability_change_event(&mut self) -> Result<sf::CopyHandle> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] AttachAvailabilityChangeEvent -- (...)\n", self.application_id.0);

        Ok(sf::Handle::from(self.availability_change_event.client_handle))
    }

    pub fn recreate_application_area(&mut self, device_handle: nfp::DeviceHandle, access_id: nfp::AccessId, data: sf::InMapAliasBuffer<u8>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] RecreateApplicationArea -- device_handle: (fake id: {}), access_id: {:#X}, data: (buf_size: {:#X})\n", self.application_id.0, device_handle.id, access_id, data.get_size());

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let application_area = area::ApplicationArea::from_id(&amiibo, self.application_id.0, access_id);
        application_area.create(data.get_address(), data.get_size(), true)?;
        amiibo.notify_written()?;
        Ok(())
    }

    pub fn format(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        log!("[{:#X}] Format -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        Ok(())
    }

    pub fn get_admin_info(&mut self, device_handle: nfp::DeviceHandle, mut out_admin_info: sf::OutFixedPointerBuffer<nfp::AdminInfo>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetAdminInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);
        
        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let admin_info = amiibo.produce_admin_info()?;
        out_admin_info.set_var(admin_info);
        Ok(())
    }

    pub fn get_register_info_private(&mut self, device_handle: nfp::DeviceHandle, mut out_register_info_private: sf::OutFixedPointerBuffer<nfp::RegisterInfoPrivate>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] GetRegisterInfoPrivate -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        let register_info_private = amiibo.produce_register_info_private()?;
        out_register_info_private.set_var(register_info_private);
        Ok(())
    }

    pub fn set_register_info_private(&mut self, device_handle: nfp::DeviceHandle, register_info_private: sf::InFixedPointerBuffer<nfp::RegisterInfoPrivate>) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] SetRegisterInfoPrivate -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        amiibo.update_from_register_info_private(register_info_private.get_var())?;
        Ok(())
    }

    pub fn delete_register_info(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] DeleteRegisterInfo -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        amiibo.delete_all_areas()?;
        Ok(())
    }

    pub fn delete_application_area(&mut self, device_handle: nfp::DeviceHandle) -> Result<()> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] DeleteApplicationArea -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);

        amiibo.delete_current_area()?;
        Ok(())
    }

    pub fn exists_application_area(&mut self, device_handle: nfp::DeviceHandle) -> Result<bool> {
        result_return_unless!(self.is_state(nfp::State::Initialized), nfp::rc::ResultDeviceNotFound);
        result_return_unless!(self.is_device_state(nfp::DeviceState::TagMounted), nfp::rc::ResultDeviceNotFound);
        log!("[{:#X}] ExistsApplicationArea -- device_handle: (fake id: {})\n", self.application_id.0, device_handle.id);

        let amiibo = emu::get_active_virtual_amiibo();
        result_return_unless!(amiibo.is_valid(), nfp::rc::ResultDeviceNotFound);
        Ok(amiibo.has_any_application_areas())
    }
}

impl Drop for EmulationHandler {
    fn drop(&mut self) {
        log!("[{:#X}] Dropping handler...\n", self.application_id.0);
        self.should_end_thread.set(true);
        self.emu_handler_thread.join().unwrap();
    }
}

pub mod user;

pub mod sys;