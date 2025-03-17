use crate::renderer::VKInstance;
use ash::{
    khr::{surface, swapchain},
    vk::{self, Handle},
};
use std::error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::renderer::{device::VKDevice, VKContext};

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

    /// # Safety
    /// Destroy Before Vulkan Instance
    /// Read VK Docs For Destruction Order
    pub unsafe fn destroy(&mut self) {
        self.surface_loader.destroy_surface(self.surface, None);
    }
}

pub struct VKSwapchainCapabilities {
    pub surface_capibilities: vk::SurfaceCapabilitiesKHR,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl VKSwapchainCapabilities {
    pub fn new(
        vk_surface: &VKSurface,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, vk::Result> {
        Ok(Self {
            surface_capibilities: unsafe {
                vk_surface
                    .surface_loader
                    .get_physical_device_surface_capabilities(physical_device, vk_surface.surface)?
            },
            surface_formats: unsafe {
                vk_surface
                    .surface_loader
                    .get_physical_device_surface_formats(physical_device, vk_surface.surface)?
            },
            present_modes: unsafe {
                vk_surface
                    .surface_loader
                    .get_physical_device_surface_present_modes(
                        physical_device,
                        vk_surface.surface,
                    )?
            },
        })
    }

    // if Mailbox Supporeted Return Mailbox else FIFO
    pub fn ideal_present_mode(&self) -> vk::PresentModeKHR {
        self.present_modes
            .iter()
            .cloned()
            .find(|present_mode| *present_mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO)
    }

    // if 8bit BGRA in SRGB Colour Space pick it Else first Option
    pub fn ideal_surface_format(&self) -> vk::SurfaceFormatKHR {
        self.surface_formats
            .iter()
            .cloned()
            .find(|surface_format| surface_format.format == vk::Format::B8G8R8A8_SRGB)
            .unwrap_or(self.surface_formats[0])
    }

    // Tries to return number of images for tripple buffering if that does not work then tries double buffering else min
    pub fn ideal_n_images(&self) -> u32 {
        let mut image_count = self.surface_capibilities.min_image_count;

        if self.surface_capibilities.min_image_count <= 3 {
            if self.surface_capibilities.max_image_count >= 3
                || self.surface_capibilities.max_image_count == 0
            {
                image_count = 3
            } else if self.surface_capibilities.max_image_count >= 2
                || self.surface_capibilities.max_image_count == 0
            {
                image_count = 2
            }
        }

        image_count
    }

    pub fn get_extent(&self, init_width: u32, init_height: u32) -> vk::Extent2D {
        // window manager can indicate that Size of window will be determined by swapchain
        // return current exent?
        if self.surface_capibilities.current_extent.width != u32::MAX {
            self.surface_capibilities.current_extent
        } else {
            let max_extent = self.surface_capibilities.max_image_extent;
            let min_extent = self.surface_capibilities.min_image_extent;
            vk::Extent2D::default()
                .width(init_width.clamp(min_extent.width, min_extent.height))
                .height(init_height.clamp(min_extent.height, max_extent.height))
        }
    }
}

pub struct VKSwapchain {
    // Swapchain starts of as none, can also be invalidated by setting to None ie window Resize
    pub swapchain: vk::SwapchainKHR,
    pub image_views: Vec<vk::ImageView>,
    pub images: Vec<vk::Image>,
    pub swapchain_loader: swapchain::Device,
    pub capibilities: VKSwapchainCapabilities,
}

impl VKSwapchain {
    pub fn new(
        vk_instance: &VKInstance,
        vk_device: &VKDevice,
        vk_surface: &VKSurface,
    ) -> Result<Self, vk::Result> {
        let physical_device = vk_device.p_device;
        let instance = &vk_instance.instance;
        let device = &vk_device.device;

        let capibilities = VKSwapchainCapabilities::new(vk_surface, physical_device)?;

        let ideal_surface_format = capibilities.ideal_surface_format();

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(vk_surface.surface)
            .min_image_count(capibilities.ideal_n_images())
            .image_format(ideal_surface_format.format)
            .image_color_space(ideal_surface_format.color_space)
            .image_extent(capibilities.get_extent(800, 600))
            .image_array_layers(1) // always 1 for non sterioscopic displays
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // opperations to be used on image can also be transfer
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // single queue can access image
            .pre_transform(capibilities.surface_capibilities.current_transform) // Don't Rotate Image
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // Alpha Blending with other windows = Opaque
            .present_mode(capibilities.ideal_present_mode())
            .clipped(true); // ignore Pixel covered by other windows

        let swapchain_loader = swapchain::Device::new(instance, device);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let image_views =
            Self::create_image_views(&images, ideal_surface_format.format, vk_device)?;

        Ok(Self {
            swapchain,
            image_views,
            images,
            swapchain_loader,
            capibilities,
        })
    }

    fn create_image_views(
        swapchain_images: &[vk::Image],
        image_format: vk::Format,
        vk_device: &VKDevice,
    ) -> Result<Vec<vk::ImageView>, vk::Result> {
        Ok(swapchain_images
            .iter()
            .map(|image| {
                let image_view_create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D) // it is a 2d image
                    .format(image_format) // the colour format matches the swapchain
                    .components(
                        vk::ComponentMapping::default()
                            .r(vk::ComponentSwizzle::IDENTITY)
                            .g(vk::ComponentSwizzle::IDENTITY)
                            .b(vk::ComponentSwizzle::IDENTITY)
                            .a(vk::ComponentSwizzle::IDENTITY),
                    ) // no components are Swizzled aka swapped or changed
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ); // 1 colour resource spanning the whole image
                unsafe {
                    vk_device
                        .device
                        .create_image_view(&image_view_create_info, None)
                }
            })
            .collect::<Result<Vec<vk::ImageView>, vk::Result>>())?
    }

    /// # Safety
    /// Destroy Before Vulkan Device
    /// Read VK Docs For Destruction Order
    pub unsafe fn destroy(&mut self, vk_device: &VKDevice) {
        self.image_views
            .iter()
            .for_each(|iv| vk_device.device.destroy_image_view(*iv, None));
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }

    pub fn rebuild_swapchain(self) {}
}

/// Manages Syncronisation objects and part of algo for presenting to screen
/// when rendering a frame
/// use in this order:
/// aquire_img
/// submit_cmd_buf Submit Your Command Buffers with img_rendered semaphore and reset img_rendered fence
/// Present Frame
// TODO: investigate timeline semaphores for sync arround the swapchain such as render completion
#[derive(Default)]
pub struct VKPresent {
    frame: u32,                           // current frame in flight
    max_frames: u32,                      // max Frames gpu can work on
    img_aquired_gpu: Vec<vk::Semaphore>,  // Image Aquired Semaphore
    img_rendered_gpu: Vec<vk::Semaphore>, // render Finished Semaphore
    img_rendered_cpu: Vec<vk::Fence>,     // render Finshed CPU Fence
    img_aquired_index: u32,
    img_in_flight: Vec<vk::Fence>,
}

pub struct ToRenderInfo {
    pub img_aquired_gpu: vk::Semaphore,
    pub img_aquired_index: u32,
    pub done_rendering_cpu: vk::Fence,
    pub done_rendering_gpu: vk::Semaphore,
}

impl VKPresent {
    pub fn get_max_frames(self) -> u32 {
        self.max_frames
    }

    /// Sets max frames in flight 2 is a good number
    /// Should not be higher than the number of images in the swapchain
    ///# Safety
    /// Recreats Sync Objects by destroying
    /// Don't Destroy Vulkan Device before/while using
    /// Don't Use while frames are in flight
    pub unsafe fn max_frames(
        mut self,
        frames: u32,
        vk_ctx: &VKContext,
    ) -> Result<Self, vk::Result> {
        self.max_frames = frames;
        self.frame %= self.max_frames;
        self.recreate_sync(vk_ctx)
    }

    /// returns aquired image and semaphore
    /// for when image is ready
    // TODO: Handle subobtimal or invalidaed swapchain
    pub fn aquire_img(&mut self, vk_ctx: &VKContext) -> Result<ToRenderInfo, vk::Result> {
        let img_rendered_cpu = *self
            .img_rendered_cpu
            .get(self.frame as usize)
            .ok_or(vk::Result::INCOMPLETE)?;

        let img_rendered_gpu = *self
            .img_rendered_gpu
            .get(self.frame as usize)
            .ok_or(vk::Result::INCOMPLETE)?;

        let img_aquired_gpu = *self
            .img_aquired_gpu
            .get(self.frame as usize)
            .ok_or(vk::Result::INCOMPLETE)?;

        // wait on cpu for currently rendering frame to finish
        unsafe {
            vk_ctx
                .vulkan_device
                .device
                .wait_for_fences(&[img_rendered_cpu], true, u64::MAX)?;
        }

        // request img from swapchain
        // _ is type bool for suboptimal or invalid swapchain
        let (img_index, _) = unsafe {
            vk_ctx
                .vulkan_swapchain
                .swapchain_loader
                .acquire_next_image(
                    vk_ctx.vulkan_swapchain.swapchain,
                    u64::MAX,
                    img_aquired_gpu,
                    vk::Fence::null(),
                )?
        };

        // Waits on Swapchain img in use, usually only occurs if the swapchain hands us a img out of order
        if let Some(img_in_flight) = self.img_in_flight.get(img_index as usize) {
            if !img_in_flight.is_null() {
                unsafe {
                    vk_ctx.vulkan_device.device.wait_for_fences(
                        &[*img_in_flight],
                        true,
                        u64::MAX,
                    )?;
                }
            }
        }

        // grow img_in_flight to value at img_index
        if (img_index as usize) >= self.img_in_flight.len() {
            self.img_in_flight
                .resize((img_index as usize) + 1, vk::Fence::null());
        }

        // associates our in flight fence with an image on the swapchain
        self.img_in_flight[img_index as usize] = img_rendered_cpu;

        // make sure fence is not signaled before command buffer would be submitted
        unsafe {
            vk_ctx
                .vulkan_device
                .device
                .reset_fences(&[img_rendered_cpu])?
        };

        Ok(ToRenderInfo {
            img_aquired_gpu,
            img_aquired_index: img_index,
            done_rendering_cpu: img_rendered_cpu,
            done_rendering_gpu: img_rendered_gpu,
        })
    }

    /// waits on rendered semaphore
    /// and then submits frame
    /// image_index is index of image obtained from aquire_image
    // TODO: Handle subobtimal or invalidaed swapchain
    pub fn present_frame(&mut self, vk_ctx: &VKContext) -> Result<(), vk::Result> {
        let swapchains = &[vk_ctx.vulkan_swapchain.swapchain];
        let semaphores = &[*self
            .img_rendered_gpu
            .get(self.frame as usize)
            .ok_or(vk::Result::INCOMPLETE)?];
        let image_indices = &[self.img_aquired_index];

        let present_info = vk::PresentInfoKHR::default()
            .swapchains(swapchains)
            .wait_semaphores(semaphores)
            .image_indices(image_indices);

        unsafe {
            vk_ctx
                .vulkan_swapchain
                .swapchain_loader
                .queue_present(vk_ctx.vulkan_device.graphics_queue, &present_info)?;
        }
        self.frame = (self.frame + 1) % self.max_frames;
        Ok(())
    }

    // Recreates Sync Objects Such as Semaphores and Fences
    unsafe fn recreate_sync(mut self, vk_ctx: &VKContext) -> Result<Self, vk::Result> {
        let vk_device = &vk_ctx.vulkan_device;
        self.destroy(vk_ctx);

        for _ in 0..self.max_frames {
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();
            let img_semaphore = vk_device
                .device
                .create_semaphore(&semaphore_create_info, None)?;
            self.img_aquired_gpu.push(img_semaphore);

            let renderd_semaphore = vk_device
                .device
                .create_semaphore(&semaphore_create_info, None)?;
            self.img_rendered_gpu.push(renderd_semaphore);

            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            let renderd_fence = vk_device.device.create_fence(&fence_create_info, None)?;
            self.img_rendered_cpu.push(renderd_fence);
        }

        Ok(self)
    }

    /// Destroys Sync Objects
    /// # Safety
    /// Destroy Before Vulkan Device
    /// Read VK Docs For Destruction Order
    /// Don't use any destroyed Sync Handles
    pub unsafe fn destroy(&mut self, vk_ctx: &VKContext) {
        let vk_device = &vk_ctx.vulkan_device;
        vk_device.device.device_wait_idle().unwrap_unchecked();
        self.img_aquired_gpu.iter().for_each(|semaphore| {
            if !semaphore.is_null() {
                vk_device.device.destroy_semaphore(*semaphore, None);
            }
        });

        self.img_rendered_gpu.iter().for_each(|semaphore| {
            if !semaphore.is_null() {
                vk_device.device.destroy_semaphore(*semaphore, None);
            }
        });

        self.img_rendered_cpu.iter().for_each(|fence| {
            if !fence.is_null() {
                vk_device.device.destroy_fence(*fence, None);
            }
        });

        self.img_aquired_gpu.clear();
        self.img_rendered_gpu.clear();
        self.img_rendered_cpu.clear();
        self.img_in_flight.clear();
    }
}
