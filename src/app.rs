use crate::renderer::VKContext;
use crate::renderer::VKRenderer;
use crate::utils::GameInfo;
use crate::utils::ReplaceWith;
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
    pub vulkan_renderer: VKRenderer<'a>,
}

impl AppCTX<'_> {
    fn new(game_info: GameInfo, event_loop: &ActiveEventLoop) -> Self {
        let (width, height) = (800, 600);
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(game_info.app_name.to_string_lossy())
                    .with_inner_size(winit::dpi::PhysicalSize::new(width, height)),
            )
            .unwrap();

        let vulkan_ctx = VKContext::new(&game_info, &window).unwrap();

        let vulkan_renderer = VKRenderer::new(vulkan_ctx, 2).unwrap();

        Self {
            game_info,
            window,
            vulkan_renderer,
        }
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
            WindowEvent::Resized(_size) => {
                if let App::Initialised(app_ctx) = self {
                    // Window Resized
                    //info!("resized window");
                    app_ctx.vulkan_renderer.vulkan_present.invalidate_swap();
                }
            }
            WindowEvent::RedrawRequested => {
                if let App::Initialised(app_ctx) = self {
                    app_ctx.vulkan_renderer.render(&app_ctx.window);
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
