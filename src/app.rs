use crate::renderer::device::VKDevice;
use crate::renderer::presentation::VKPresent;
use crate::renderer::VKContext;
use crate::utils::GameInfo;
use crate::utils::ReplaceWith;
use ash::vk;
use log::info;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::Window;
use winit::window::WindowId;

pub struct AppCTX<'a> {
    pub game_info: GameInfo,
    pub window: Window,
    pub vulkan_ctx: VKContext<'a>,
    pub vulkan_present: VKPresent,
    pub vulkan_cmd_buffs: Vec<vk::CommandBuffer>,
}

impl AppCTX<'_> {
    fn new(game_info: GameInfo, event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(
                Window::default_attributes().with_title(game_info.app_name.to_string_lossy()),
            )
            .unwrap();

        let frames_in_flight = 2;

        let vulkan_ctx = VKContext::new(&game_info, &window).unwrap();
        let vulkan_present = unsafe {
            VKPresent::default()
                .max_frames(frames_in_flight, &vulkan_ctx)
                .unwrap()
        };

        // allocate 1 primary command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(vulkan_ctx.vulkan_cmd_pool)
            .command_buffer_count(frames_in_flight)
            .level(vk::CommandBufferLevel::PRIMARY);

        let vulkan_cmd_buffs = unsafe {
            vulkan_ctx
                .vulkan_device
                .device
                .allocate_command_buffers(&alloc_info)
                .unwrap()
        };

        Self {
            game_info,
            window,
            vulkan_ctx,
            vulkan_present,
            vulkan_cmd_buffs,
        }
    }
}

impl Drop for AppCTX<'_> {
    fn drop(&mut self) {
        unsafe { self.vulkan_present.destroy(&self.vulkan_ctx) };
    }
}

pub enum App<'a> {
    Initialised(AppCTX<'a>),
    Uninitialised { game_info: GameInfo },
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let App::Uninitialised { .. } = self {
            self.init(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let App::Initialised(app_ctx) = self {

                    // Window Resized
                }
            }
            WindowEvent::RedrawRequested => {
                if let App::Initialised(app_ctx) = self {
                    render(app_ctx);
                    app_ctx.window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

impl<F> ReplaceWith<F> for App<'_> {}

impl App<'_> {
    pub fn new(game_info: GameInfo) -> Self {
        App::Uninitialised { game_info }
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) {
        self.replace_with(|state| match state {
            Self::Initialised(_) => panic!(),
            Self::Uninitialised { game_info } => {
                info!(
                    "Initialising Game: {}",
                    game_info.app_name.to_string_lossy()
                );
                Self::Initialised(AppCTX::new(game_info, event_loop))
            }
        });
    }

    pub fn start<T>(&mut self, event_loop: &mut EventLoop<T>) -> Result<(), EventLoopError>
    where
        Self: ApplicationHandler<T>,
    {
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app_on_demand(self)
    }
}

fn render(app_ctx: &mut AppCTX) {
    let vk_ctx = &app_ctx.vulkan_ctx;
    let vk_present = &mut app_ctx.vulkan_present;
    let vk_device = &vk_ctx.vulkan_device;

    let render_info = vk_present.aquire_img(vk_ctx).unwrap();

    unsafe {
        record_cmd_buffer(
            app_ctx.vulkan_cmd_buffs[render_info.frame_in_flight as usize],
            vk_device,
            vk_ctx.vulkan_swapchain.images[render_info.img_aquired_index as usize],
        )
        .unwrap();
    }

    let command_buffer_infos = &[vk::CommandBufferSubmitInfo::default()
        .command_buffer(app_ctx.vulkan_cmd_buffs[render_info.frame_in_flight as usize])];

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
    app_ctx.window.pre_present_notify();

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
