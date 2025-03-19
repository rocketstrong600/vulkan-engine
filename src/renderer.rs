pub mod device;
pub mod presentation;
pub mod shader;

use crate::renderer::device::VKDevice;
use crate::renderer::presentation::VKPresent;
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
pub struct VKContext {
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

        Ok(Self {
            vulkan_instance,
            vulkan_device,
            vulkan_surface,
            vulkan_swapchain,
        })
    }

    pub unsafe fn destroy(&mut self) {
        self.vulkan_swapchain.destroy(&self.vulkan_device);
        self.vulkan_surface.destroy();
        self.vulkan_device.destroy();
        self.vulkan_instance.destroy();
    }
}

pub fn display_vk_ext(window: &Window) -> Result<&'static [*const c_char], Box<dyn error::Error>> {
    let display_handle = window.display_handle()?;

    Ok(ash_window::enumerate_required_extensions(
        display_handle.as_raw(),
    )?)
}

pub struct VKRenderer<'a> {
    pub vulkan_ctx: VKContext,
    pub vulkan_shader_loader: VKShaderLoader<&'static str>,
    pub vulkan_present: VKPresent,

    pub vulkan_cmd_pool: vk::CommandPool,
    pub vulkan_cmd_buffs: Vec<vk::CommandBuffer>,
    pub vertex_shader: VKShader<'a>,
    pub fragment_shader: VKShader<'a>,
}

impl VKRenderer<'_> {
    pub fn new(
        vulkan_ctx: VKContext,
        frames_in_flight: u32,
    ) -> Result<Self, Box<dyn error::Error>> {
        let vulkan_present = unsafe {
            VKPresent::default()
                .max_frames(frames_in_flight, &vulkan_ctx)
                .unwrap()
        };

        let cmd_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(vulkan_ctx.vulkan_device.queue_index);

        // Create Command Pool
        let vulkan_cmd_pool = unsafe {
            vulkan_ctx
                .vulkan_device
                .device
                .create_command_pool(&cmd_pool_info, None)?
        };

        // allocate 1 primary command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(vulkan_cmd_pool)
            .command_buffer_count(frames_in_flight)
            .level(vk::CommandBufferLevel::PRIMARY);

        let vulkan_cmd_buffs = unsafe {
            vulkan_ctx
                .vulkan_device
                .device
                .allocate_command_buffers(&alloc_info)
                .unwrap()
        };

        let mut vulkan_shader_loader = VKShaderLoader::default();
        let vertex_shader = VKShader::new(
            &vulkan_ctx.vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::VERTEX,
            c"vertexMain",
            &mut vulkan_shader_loader,
        )?;

        let fragment_shader = VKShader::new(
            &vulkan_ctx.vulkan_device,
            "shaders/triangle.spv",
            ShaderStageFlags::FRAGMENT,
            c"fragMain",
            &mut vulkan_shader_loader,
        )?;

        Ok(Self {
            vulkan_ctx,
            vulkan_shader_loader,
            vulkan_present,
            vulkan_cmd_pool,
            vulkan_cmd_buffs,
            vertex_shader,
            fragment_shader,
        })
    }

    pub fn render(&mut self, window: &Window) {
        let vk_ctx = &self.vulkan_ctx;
        let vk_present = &mut self.vulkan_present;
        let vk_device = &vk_ctx.vulkan_device;

        let render_info = vk_present.aquire_img(vk_ctx).unwrap();

        unsafe {
            Self::record_cmd_buffer(
                self.vulkan_cmd_buffs[render_info.frame_in_flight as usize],
                vk_device,
                vk_ctx.vulkan_swapchain.images[render_info.img_aquired_index as usize],
            )
            .unwrap();
        }

        let command_buffer_infos = &[vk::CommandBufferSubmitInfo::default()
            .command_buffer(self.vulkan_cmd_buffs[render_info.frame_in_flight as usize])];

        let wait_semaphore_infos = &[vk::SemaphoreSubmitInfo::default()
            .semaphore(render_info.img_aquired_gpu)
            .stage_mask(vk::PipelineStageFlags2::TRANSFER)];

        let signal_semaphore_infos = &[vk::SemaphoreSubmitInfo::default()
            .semaphore(render_info.done_rendering_gpu)
            .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)];

        let submits = [vk::SubmitInfo2::default()
            .wait_semaphore_infos(wait_semaphore_infos)
            .signal_semaphore_infos(signal_semaphore_infos)
            .command_buffer_infos(command_buffer_infos)];

        unsafe {
            vk_device
                .device
                .queue_submit2(
                    vk_device.graphics_queue,
                    &submits,
                    render_info.done_rendering_cpu,
                )
                .unwrap()
        };

        // required for wayland
        window.pre_present_notify();

        vk_present.present_frame(vk_ctx).unwrap();
    }

    unsafe fn record_cmd_buffer(
        cmd_buffer: ash::vk::CommandBuffer,
        vk_device: &VKDevice,
        image: ash::vk::Image,
    ) -> Result<(), ash::vk::Result> {
        let begin_info = vk::CommandBufferBeginInfo::default();

        let sub_resource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        // memory barriar info for clear
        // we use memory barriars to transistion the image into the correct layout
        // this is for transitioning the layout to the required layout for screen clear cmd
        let image_memory_barriers = [vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .image(image)
            .subresource_range(sub_resource_range)];

        // memory barriar info for present
        // we use memory barriars to transistion the image into the correct layout
        // this is for the final layout before presenting
        let present_image_memory_barriers = [vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE_KHR)
            .image(image)
            .subresource_range(sub_resource_range)];

        // nice pinky colour
        let mut clear_color_value = vk::ClearColorValue::default();
        clear_color_value.float32 = [0.74757, 0.02016, 0.253, 1.0];

        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);

        let present_dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(&present_image_memory_barriers);

        vk_device
            .device
            .begin_command_buffer(cmd_buffer, &begin_info)
            .unwrap();

        vk_device
            .device
            .cmd_pipeline_barrier2(cmd_buffer, &dependency_info);

        vk_device.device.cmd_clear_color_image(
            cmd_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &clear_color_value,
            &[sub_resource_range],
        );

        vk_device
            .device
            .cmd_pipeline_barrier2(cmd_buffer, &present_dependency_info);

        vk_device.device.end_command_buffer(cmd_buffer)
    }
}

impl Drop for VKRenderer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_ctx
                .vulkan_device
                .device
                .device_wait_idle()
                .unwrap_unchecked();
            self.fragment_shader.destroy(&self.vulkan_ctx.vulkan_device);
            self.vertex_shader.destroy(&self.vulkan_ctx.vulkan_device);

            self.vulkan_present.destroy(&self.vulkan_ctx);

            self.vulkan_ctx
                .vulkan_device
                .device
                .destroy_command_pool(self.vulkan_cmd_pool, None);
            self.vulkan_ctx.destroy();
        }
    }
}
