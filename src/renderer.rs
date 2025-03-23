pub mod device;
pub mod presentation;
pub mod shader;

use crate::renderer::device::VKDevice;
use crate::renderer::presentation::VKPresent;
use crate::utils::GameInfo;
use ash::vk::{CommandBufferUsageFlags, PolygonMode, ShaderStageFlags};
use ash::{vk, Entry, Instance};
use gpu_allocator::vulkan;
use gpu_allocator::MemoryLocation;
use presser;

use presentation::{VKSurface, VKSwapchain};
use shader::{VKShader, VKShaderLoader};
use std::error;
use std::ffi::c_char;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;

use glam::{Vec2, Vec3};

use log::info;

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
    pub mem_allocator: Option<vulkan::Allocator>,
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
        let vulkan_swapchain =
            VKSwapchain::new(&vulkan_instance, &vulkan_device, &vulkan_surface, &window)?;

        let alloc_desc = vulkan::AllocatorCreateDesc {
            instance: vulkan_instance.instance.clone(),
            device: vulkan_device.device.clone(),
            physical_device: vulkan_device.p_device,
            debug_settings: Default::default(),
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        };

        let mem_allocator = Some(vulkan::Allocator::new(&alloc_desc)?);

        Ok(Self {
            mem_allocator,
            vulkan_instance,
            vulkan_device,
            vulkan_surface,
            vulkan_swapchain,
        })
    }

    /// # Safety
    /// Vulkan CTX should be destroyed after all of your vk objects
    /// Read VK Docs For Destruction Order
    pub unsafe fn destroy(&mut self) {
        drop(std::mem::take(&mut self.mem_allocator));
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

    pub vertex_buffer: vk::Buffer,
    pub vertex_allocation: vulkan::Allocation,

    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,

    pub vertices_len: u32,
}

impl VKRenderer<'_> {
    pub fn new(
        mut vulkan_ctx: VKContext,
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

        // our triangle to render
        static VERTICES: [Vertex; 3] = [
            Vertex::new(Vec2::new(0.0, -0.5), Vec3::new(1.0, 0.0, 0.0)),
            Vertex::new(Vec2::new(0.5, 0.5), Vec3::new(0.0, 1.0, 0.0)),
            Vertex::new(Vec2::new(-0.5, 0.5), Vec3::new(0.0, 0.0, 1.0)),
        ];

        let vertices_len = VERTICES.len() as u32;

        let (vertex_buffer, vertex_allocation) = create_vertex_buffer(
            &vulkan_ctx.vulkan_device,
            vulkan_ctx.mem_allocator.as_mut().unwrap(),
            &vulkan_cmd_pool,
            &VERTICES,
        )?;

        let (pipeline, pipeline_layout) = create_pipeline(
            &vulkan_ctx.vulkan_device,
            &vulkan_ctx.vulkan_swapchain,
            &vertex_shader.shader_info,
            &fragment_shader.shader_info,
        )?;

        Ok(Self {
            vulkan_ctx,
            vulkan_shader_loader,
            vulkan_present,
            vulkan_cmd_pool,
            vulkan_cmd_buffs,
            vertex_shader,
            fragment_shader,

            vertex_buffer,
            vertex_allocation,

            pipeline,
            pipeline_layout,

            vertices_len,
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
                vk_ctx.vulkan_swapchain.image_views[render_info.img_aquired_index as usize],
                vk_ctx.vulkan_swapchain.image_extent,
                self.pipeline,
                self.vertex_buffer,
                self.vertices_len,
            )
            .unwrap();
        }

        let command_buffer_infos = &[vk::CommandBufferSubmitInfo::default()
            .command_buffer(self.vulkan_cmd_buffs[render_info.frame_in_flight as usize])];

        let wait_semaphore_infos = &[vk::SemaphoreSubmitInfo::default()
            .semaphore(render_info.img_aquired_gpu)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

        let signal_semaphore_infos = &[vk::SemaphoreSubmitInfo::default()
            .semaphore(render_info.done_rendering_gpu)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

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
        cmd_buffer: vk::CommandBuffer,
        vk_device: &VKDevice,
        image: vk::Image,
        image_view: vk::ImageView,
        render_area: vk::Extent2D,
        pipeline: vk::Pipeline,
        vertex_buffer: vk::Buffer,
        vertices_len: u32,
    ) -> Result<(), ash::vk::Result> {
        let begin_info = vk::CommandBufferBeginInfo::default();

        let sub_resource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        // memory barriar info for rendering
        // we use memory barriars to transistion the image into the correct layout
        // this is for transitioning the layout to the required layout for screen clear cmd
        let image_memory_barriers = [vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .image(image)
            .subresource_range(sub_resource_range)];

        // memory barriar info for present
        // we use memory barriars to transistion the image into the correct layout
        // this is for the final layout before presenting
        let present_image_memory_barriers = [vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
            .image(image)
            .subresource_range(sub_resource_range)];

        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);

        let present_dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(&present_image_memory_barriers);

        let mut clear_value = vk::ClearValue::default();
        clear_value.color = vk::ClearColorValue::default();
        clear_value.color.float32 = [0.74757, 0.02016, 0.253, 1.0];

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_value)];

        let render_area_extent = vk::Rect2D::default()
            .extent(render_area)
            .offset(vk::Offset2D::default().x(0).y(0));

        let rendering_info = vk::RenderingInfo::default()
            .color_attachments(&color_attachments)
            .layer_count(1)
            .render_area(render_area_extent);

        let viewport = [vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(render_area.width as f32)
            .height(render_area.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];

        vk_device
            .device
            .begin_command_buffer(cmd_buffer, &begin_info)
            .unwrap();

        vk_device
            .device
            .cmd_pipeline_barrier2(cmd_buffer, &dependency_info);

        vk_device
            .device
            .cmd_begin_rendering(cmd_buffer, &rendering_info);

        vk_device
            .device
            .cmd_bind_pipeline(cmd_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);

        vk_device
            .device
            .cmd_bind_vertex_buffers(cmd_buffer, 0, &[vertex_buffer], &[0u64]);

        vk_device.device.cmd_set_viewport(cmd_buffer, 0, &viewport);

        vk_device
            .device
            .cmd_set_scissor(cmd_buffer, 0, &[render_area_extent]);

        vk_device.device.cmd_draw(cmd_buffer, vertices_len, 1, 0, 0);

        vk_device.device.cmd_end_rendering(cmd_buffer);

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

            self.vulkan_ctx
                .vulkan_device
                .device
                .destroy_pipeline(self.pipeline, None);

            self.vulkan_ctx
                .vulkan_device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            // need to move it out of &mut self so it can be freed by memory allocator, achieved by replacing with empty Allocation
            let vertex_allocation = std::mem::take(&mut self.vertex_allocation);

            self.vulkan_ctx
                .mem_allocator
                .as_mut()
                .unwrap()
                .free(vertex_allocation)
                .unwrap_unchecked();

            self.vulkan_ctx
                .vulkan_device
                .device
                .destroy_buffer(self.vertex_buffer, None);

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

// Repr C here so that rust does not change the order on compile and it is what vulkan expects
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    pos: Vec2,
    color: Vec3,
}

impl Vertex {
    const fn new(pos: Vec2, color: Vec3) -> Self {
        Self { pos, color }
    }

    // vulkan information for layout in memory
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    // vulkan information for the sub elements in memory
    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0);
        let color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec2>() as u32);
        [pos, color]
    }
}

// this is just for learning it will be split up and organised and made more universal/generic.
fn create_vertex_buffer(
    vk_device: &VKDevice,
    gpu_allocator: &mut vulkan::Allocator,
    vk_command_pool: &vk::CommandPool,
    vertices: &[Vertex],
) -> Result<(vk::Buffer, vulkan::Allocation), vk::Result> {
    // create a staging buffer

    let vk_info = vk::BufferCreateInfo::default()
        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .size((size_of::<Vertex>() * vertices.len()) as u64)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let staging_buffer = unsafe { vk_device.device.create_buffer(&vk_info, None)? };

    let requirments = unsafe {
        vk_device
            .device
            .get_buffer_memory_requirements(staging_buffer)
    };

    // allocate memory for staging buffer

    let mut staging_allocation = gpu_allocator
        .allocate(&vulkan::AllocationCreateDesc {
            name: "Vertecies Staging",
            requirements: requirments,
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: vulkan::AllocationScheme::DedicatedBuffer(staging_buffer),
        })
        .unwrap();

    // bind staging buffer to memory

    unsafe {
        vk_device.device.bind_buffer_memory(
            staging_buffer,
            staging_allocation.memory(),
            staging_allocation.offset(),
        )?
    };

    // copy vertecies into staging buffer
    // non 0 start offset issue?

    let copy_info = presser::copy_from_slice_to_offset_with_align(
        &vertices,
        &mut staging_allocation,
        0,
        requirments.alignment as usize,
    )
    .unwrap();

    info!("Vertex Memory Offset: {}", copy_info.copy_start_offset);

    // create vertex buffer

    let vk_info = vk::BufferCreateInfo::default()
        .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER)
        .size((size_of::<Vertex>() * vertices.len()) as u64)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let vertex_buffer = unsafe { vk_device.device.create_buffer(&vk_info, None)? };

    let requirments = unsafe {
        vk_device
            .device
            .get_buffer_memory_requirements(vertex_buffer)
    };

    // allocate memory for vertex buffer

    let vertices_allocation = gpu_allocator
        .allocate(&vulkan::AllocationCreateDesc {
            name: "Vertices",
            requirements: requirments,
            location: MemoryLocation::GpuOnly,
            linear: true,
            allocation_scheme: vulkan::AllocationScheme::DedicatedBuffer(vertex_buffer),
        })
        .unwrap();

    // bind vertex buffer to memory

    unsafe {
        vk_device.device.bind_buffer_memory(
            vertex_buffer,
            vertices_allocation.memory(),
            vertices_allocation.offset(),
        )?
    };

    // copy staging buffer memory to vertex buffer memory

    let buff_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(*vk_command_pool)
        .command_buffer_count(1);

    let cmd_buffer = unsafe { vk_device.device.allocate_command_buffers(&buff_info)?[0] };

    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    let copy_region = vk::BufferCopy::default().size((size_of::<Vertex>() * vertices.len()) as u64);

    let cmd_buffer_info = [vk::CommandBufferSubmitInfo::default().command_buffer(cmd_buffer)];
    let submit_info = vk::SubmitInfo2::default().command_buffer_infos(&cmd_buffer_info);
    unsafe {
        vk_device
            .device
            .begin_command_buffer(cmd_buffer, &begin_info)?;

        vk_device
            .device
            .cmd_copy_buffer(cmd_buffer, staging_buffer, vertex_buffer, &[copy_region]);

        vk_device.device.end_command_buffer(cmd_buffer)?;

        vk_device.device.queue_submit2(
            vk_device.graphics_queue,
            &[submit_info],
            vk::Fence::null(),
        )?;

        // fence more flexible than queue wait idle
        vk_device.device.queue_wait_idle(vk_device.graphics_queue)?;

        // free single use command buffers
        vk_device
            .device
            .free_command_buffers(*vk_command_pool, &[cmd_buffer]);
    }

    // clean up staging buffer as we no longer need it
    gpu_allocator.free(staging_allocation).unwrap();

    unsafe {
        vk_device.device.destroy_buffer(staging_buffer, None);
    };

    Ok((vertex_buffer, vertices_allocation))
}

fn create_pipeline(
    vk_device: &VKDevice,
    vk_swapchain: &VKSwapchain,
    vertex_stage: &vk::PipelineShaderStageCreateInfo,
    fragment_stage: &vk::PipelineShaderStageCreateInfo,
) -> Result<(vk::Pipeline, vk::PipelineLayout), vk::Result> {
    // we wan't the viewport and scissor to be dynamic so that we don't have to recreat the pipeline when the window size changes
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let bind_desc = [Vertex::binding_description()];
    let attr_desc = Vertex::attribute_descriptions();

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bind_desc)
        .vertex_attribute_descriptions(&attr_desc);

    //tringle list aka no vertices are shared between triangles
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // only specify count because viewport state is dynamic
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // no depth test code as not needed yet

    //blending disabled Probably need alpha blending later
    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false)];

    let color_blend_state =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachment);

    let color_attachment_formats = [vk_swapchain.capibilities.ideal_surface_format().format];

    let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats);

    let layout_info = vk::PipelineLayoutCreateInfo::default();

    let pipeline_layout = unsafe {
        vk_device
            .device
            .create_pipeline_layout(&layout_info, None)?
    };

    let stages = [*vertex_stage, *fragment_stage];

    let create_infos = &[vk::GraphicsPipelineCreateInfo::default()
        .dynamic_state(&dynamic_state)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info)
        .stages(&stages)];

    unsafe {
        let pipline_result = vk_device.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            create_infos,
            None,
        );

        // the result of create_graphics_pipeline can include the pipeleines that did get sucesfully created.
        // this match statement just ignores that ant returns error if any of them fail
        match pipline_result {
            Ok(pipeline) => Ok((pipeline[0], pipeline_layout)),
            Err(error) => Err(error.1),
        }
    }
}
