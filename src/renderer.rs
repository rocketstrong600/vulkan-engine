pub mod device;
pub mod window;

use ash::{vk, Entry, Instance};
use std::error;
use std::ffi::{c_char, CStr};

pub const ENGINE_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const ENGINE_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const ENGINE_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

#[allow(dead_code)]
pub struct VulkanInstance {
    _entry: Entry,
    pub instance: Instance,
}

#[allow(dead_code)]
impl VulkanInstance {
    pub fn new(
        app_name: &CStr,
        app_major: u32,
        app_minor: u32,
        app_patch: u32,
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
            .application_name(app_name)
            .application_version(vk::make_api_version(0, app_major, app_minor, app_patch))
            .engine_name(c"Alcor")
            .engine_version(engine_version);

        let extension_names: &[*const c_char] = if let Some(ext_names) = extension_names {
            ext_names
        } else {
            &[] as &[*const c_char]
        };

        let instance = Self::create_instance(&entry, &app_info, extension_names)?;

        Ok(Self {
            _entry: entry,
            instance,
        })
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
pub struct GameInfo<'a> {
    pub app_name: &'a CStr,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[allow(dead_code)]
impl<'a> Default for GameInfo<'a> {
    fn default() -> Self {
        Self {
            app_name: c"",
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}

#[allow(dead_code)]
pub struct VulkanContext {
    pub vulkan_instance: VulkanInstance,
}

#[allow(dead_code)]
impl VulkanContext {
    pub fn new(game_info: GameInfo) -> Result<Self, Box<dyn error::Error>> {
        Ok(Self {
            vulkan_instance: VulkanInstance::new(
                game_info.app_name,
                game_info.major,
                game_info.minor,
                game_info.patch,
                None,
            )?,
        })
    }
}

#[allow(dead_code)]
pub struct RenderingCollection<'a> {
    pub vulkan_context: VulkanContext,
    pub window: window::WindowLoop<'a>,
}

#[allow(dead_code)]
impl<'a> RenderingCollection<'a> {
    pub fn new(game_info: GameInfo) -> Result<Self, Box<dyn error::Error>> {
        let mut rendering_collection = Self {
            window: window::WindowLoop::default(),
            vulkan_context: VulkanContext::new(game_info)?,
        };

        rendering_collection.window.set_renderer(render);
        Ok(rendering_collection)
    }
}

pub fn render(vulkan_context: &VulkanContext) {}
