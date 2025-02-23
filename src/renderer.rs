pub mod device;

use crate::renderer::device::VulkanDevice;
use crate::utils::GameInfo;
use ash::{vk, Entry, Instance};
use std::error;
use std::ffi::c_char;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;

pub const ENGINE_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const ENGINE_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const ENGINE_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

#[allow(dead_code)]
pub struct VulkanInstance {
    pub entry: Entry,
    pub instance: Instance,
}

#[allow(dead_code)]
impl VulkanInstance {
    pub fn new(
        game_info: &GameInfo,
        extension_names: Option<&[*const c_char]>,
    ) -> Result<Self, Box<dyn error::Error>> {
        // Load Vulkan Library
        let entry = unsafe { Entry::load()? };

        let engine_version = vk::make_api_version(
            0,
            ENGINE_MAJOR.parse()?,
            ENGINE_MINOR.parse()?,
            ENGINE_PATCH.parse()?,
        );

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::make_api_version(0, 1, 3, 0))
            .application_name(game_info.app_name)
            .application_version(vk::make_api_version(
                0,
                game_info.major,
                game_info.minor,
                game_info.patch,
            ))
            .engine_name(c"Alcor")
            .engine_version(engine_version);

        let extension_names: &[*const c_char] = if let Some(ext_names) = extension_names {
            ext_names
        } else {
            &[] as &[*const c_char]
        };

        let instance = Self::create_instance(&entry, &app_info, extension_names)?;

        Ok(Self { entry, instance })
    }

    fn create_instance(
        entry: &Entry,
        app_info: &vk::ApplicationInfo,
        extension_names: &[*const c_char],
    ) -> Result<Instance, Box<dyn error::Error>> {
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(app_info)
            .enabled_extension_names(extension_names);
        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(instance)
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

#[allow(dead_code)]
pub struct VulkanContext {
    pub vulkan_instance: VulkanInstance,
    pub vulkan_device: VulkanDevice,
}

#[allow(dead_code)]
impl VulkanContext {
    pub fn new(game_info: &GameInfo, window: &Window) -> Result<Self, Box<dyn error::Error>> {
        let vk_instance_ext = display_vk_ext(window)?;
        let vulkan_instance = VulkanInstance::new(game_info, Some(vk_instance_ext))?;
        let vulkan_device = VulkanDevice::new(&vulkan_instance.instance)?;
        Ok(Self {
            vulkan_instance,
            vulkan_device,
        })
    }
}

pub fn display_vk_ext(window: &Window) -> Result<&'static [*const c_char], Box<dyn error::Error>> {
    let display_handle = window.display_handle()?.clone();

    Ok(ash_window::enumerate_required_extensions(
        display_handle.as_raw(),
    )?)
}
