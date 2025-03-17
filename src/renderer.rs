pub mod device;
pub mod presentation;
pub mod shader;

use crate::renderer::device::VKDevice;
use crate::utils::GameInfo;
use ash::vk::ShaderStageFlags;
use ash::{vk, Entry, Instance};
use presentation::{VKSurface, VKSwapchain};
use shader::{VKShader, VKShaderLoader};
use std::error;
use std::ffi::c_char;
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

    /// # Safety
    /// Instance should be Destroyed After All Other Vulkan Objects
    /// Read VK Docs For Destruction Order
    pub unsafe fn destroy(&mut self) {
        self.instance.destroy_instance(None);
    }
}

//Safe Destruction Order structs drop from top to bottom.
pub struct VKContext<'a> {
    pub vulkan_cmd_pool: vk::CommandPool, // TODO: probably move out of here to something that magages the actual rendering algo

    pub vertex_shader: VKShader<'a>,
    pub fragment_shader: VKShader<'a>,

    pub vulkan_shader_loader: VKShaderLoader<&'static str>,
    pub vulkan_swapchain: VKSwapchain,
    pub vulkan_surface: VKSurface,
    pub vulkan_device: VKDevice,
    pub vulkan_instance: VKInstance,
}

impl VKContext<'_> {
    pub fn new(game_info: &GameInfo, window: &Window) -> Result<Self, Box<dyn error::Error>> {
        let vk_instance_ext = display_vk_ext(window)?;
        let vulkan_instance = VKInstance::new(game_info, Some(vk_instance_ext))?;
        let vulkan_surface = VKSurface::new(&vulkan_instance, window)?;
        let vulkan_device = VKDevice::new(&vulkan_instance, &vulkan_surface)?;
        let vulkan_swapchain = VKSwapchain::new(&vulkan_instance, &vulkan_device, &vulkan_surface)?;
        let mut vulkan_shader_loader = VKShaderLoader::default();

        let cmd_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(vulkan_device.queue_index);

        // Create Command Pool
        let vulkan_cmd_pool = unsafe {
            vulkan_device
                .device
                .create_command_pool(&cmd_pool_info, None)?
        };

        let vertex_shader = VKShader::new(
            &vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::VERTEX,
            c"vertexMain",
            &mut vulkan_shader_loader,
        )?;

        let fragment_shader = VKShader::new(
            &vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::FRAGMENT,
            c"fragMain",
            &mut vulkan_shader_loader,
        )?;

        Ok(Self {
            vulkan_instance,
            vulkan_device,
            vulkan_surface,
            vulkan_swapchain,
            vulkan_shader_loader,
            vertex_shader,
            fragment_shader,
            vulkan_cmd_pool,
        })
    }
}

impl Drop for VKContext<'_> {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_device
                .device
                .device_wait_idle()
                .unwrap_unchecked();

            self.vulkan_device
                .device
                .destroy_command_pool(self.vulkan_cmd_pool, None);

            self.fragment_shader.destroy(&self.vulkan_device);
            self.vertex_shader.destroy(&self.vulkan_device);

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
