use crate::renderer::VKInstance;
use ash::{khr::surface, vk};
use std::error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub struct VKSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: surface::Instance,
}

impl VKSurface {
    pub fn new(vk_instance: &VKInstance, window: &Window) -> Result<Self, Box<dyn error::Error>> {
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

    pub fn get_swapchain_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<VKSwapchainCapabilities, vk::Result> {
        Ok(VKSwapchainCapabilities {
            surface_capibilities: unsafe {
                self.surface_loader
                    .get_physical_device_surface_capabilities(physical_device, self.surface)?
            },
            surface_formats: unsafe {
                self.surface_loader
                    .get_physical_device_surface_formats(physical_device, self.surface)?
            },
            present_modes: unsafe {
                self.surface_loader
                    .get_physical_device_surface_present_modes(physical_device, self.surface)?
            },
        })
    }
}

impl Drop for VKSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

pub struct VKSwapchainCapabilities {
    pub surface_capibilities: vk::SurfaceCapabilitiesKHR,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub struct VKSwapchain {
    pub capibilities: VKSwapchainCapabilities,
}

impl VKSwapchain {}
