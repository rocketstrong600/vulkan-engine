use crate::renderer::VulkanInstance;
use ash::{khr::surface, vk};
use std::error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub struct VulkanSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: surface::Instance,
}

impl VulkanSurface {
    pub fn new(
        vk_instance: &VulkanInstance,
        window: &Window,
    ) -> Result<Self, Box<dyn error::Error>> {
        let surface = unsafe {
            ash_window::create_surface(
                &vk_instance.entry,
                &vk_instance.instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };

        let surface_loader = surface::Instance::new(&vk_instance.entry, &vk_instance.instance);

        Ok(Self {
            surface_loader,
            surface,
        })
    }

    pub fn queue_supports_surface(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_index: u32,
    ) -> Result<bool, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device,
                queue_index,
                self.surface,
            )
        }
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
