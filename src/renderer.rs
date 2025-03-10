pub mod device;
pub mod shader;
pub mod surface;

use crate::renderer::device::VKDevice;
use crate::utils::GameInfo;
use ash::vk::ShaderStageFlags;
use ash::{vk, Entry, Instance};
use shader::{VKShader, VKShaderLoader};
use std::error;
use std::ffi::c_char;
use surface::{VKSurface, VKSwapchain};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;

pub const ENGINE_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const ENGINE_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const ENGINE_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

pub struct VKInstance {
    pub instance: Instance,
    pub entry: Entry,
}

impl VKInstance {
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

    pub unsafe fn destroy(&self) {
        self.instance.destroy_instance(None);
    }
}

//Safe Destruction Order structs drop from top to bottom.
pub struct VKContext {
    pub vulkan_shader_loader: VKShaderLoader<&'static str>,
    pub vulkan_swapchain: VKSwapchain,
    pub vulkan_surface: VKSurface,
    pub vulkan_device: VKDevice,
    pub vulkan_instance: VKInstance,
}

impl VKContext {
    pub fn new(game_info: &GameInfo, window: &Window) -> Result<Self, Box<dyn error::Error>> {
        let vk_instance_ext = display_vk_ext(window)?;
        let vulkan_instance = VKInstance::new(game_info, Some(vk_instance_ext))?;
        let vulkan_surface = VKSurface::new(&vulkan_instance, window)?;
        let vulkan_device = VKDevice::new(&vulkan_instance, &vulkan_surface)?;
        let vulkan_swapchain = VKSwapchain::new(&vulkan_instance, &vulkan_device, &vulkan_surface)?;
        let mut vulkan_shader_loader = VKShaderLoader::default();

        let vertex_shader = VKShader::new(
            &vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::VERTEX,
            c"vertexMain",
            &mut vulkan_shader_loader,
        );

        let fragment_shader = VKShader::new(
            &vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::FRAGMENT,
            c"fragMain",
            &mut vulkan_shader_loader,
        );

        Ok(Self {
            vulkan_instance,
            vulkan_device,
            vulkan_surface,
            vulkan_swapchain,
            vulkan_shader_loader,
        })
    }
}

impl Drop for VKContext {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_swapchain.destroy(&self.vulkan_device);
            self.vulkan_surface.destroy();
            self.vulkan_device.destroy();
            self.vulkan_instance.destroy();
        }
    }
}

pub fn display_vk_ext(window: &Window) -> Result<&'static [*const c_char], Box<dyn error::Error>> {
    let display_handle = window.display_handle()?;

    Ok(ash_window::enumerate_required_extensions(
        display_handle.as_raw(),
    )?)
}
