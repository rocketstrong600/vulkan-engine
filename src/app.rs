use crate::renderer::VulkanContext;
use crate::utils::GameInfo;
use crate::utils::ReplaceWith;
use ash::khr::surface;
use log::info;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::Window;
use winit::window::WindowId;

pub struct AppCTX {
    game_info: GameInfo,
    window: Window,
    vulkan_ctx: VulkanContext,
}

impl AppCTX {
    fn new(game_info: GameInfo, event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(
                Window::default_attributes().with_title(game_info.app_name.to_string_lossy()),
            )
            .unwrap();

        let vulkan_ctx = VulkanContext::new(&game_info, &window).unwrap();

        // create surface for binding vulkan to window surface. TODO move out of appCTX into vulkan stuff REQ figure out structure
        let surface = unsafe {
            ash_window::create_surface(
                &vulkan_ctx.vulkan_instance.entry,
                &vulkan_ctx.vulkan_instance.instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
        }
        .unwrap();

        let surface_loader = surface::Instance::new(
            &vulkan_ctx.vulkan_instance.entry,
            &vulkan_ctx.vulkan_instance.instance,
        );

        Self {
            game_info,
            window,
            vulkan_ctx,
        }
    }
}

pub enum App {
    Initialised(AppCTX),
    Uninitialised { game_info: GameInfo },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let App::Uninitialised { .. } = self {
            self.init(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let App::Initialised(app_ctx) = self {
                    app_ctx.window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

impl<F> ReplaceWith<F> for App {}

impl App {
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
