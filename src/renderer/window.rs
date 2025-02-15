use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use super::VulkanContext;

pub struct WindowLoop<'a> {
    pub window: Option<Window>,
    render_callback: fn(&VulkanContext),
    vulkan_context: Option<&'a VulkanContext>,
}

#[allow(dead_code)]
impl<'a> WindowLoop<'a> {
    pub fn start_loop(mut self) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::new()?;

        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self)?;
        Ok(())
    }

    pub fn set_renderer(&mut self, f: fn(&VulkanContext)) {
        self.render_callback = f;
    }
}

impl<'a> Default for WindowLoop<'a> {
    fn default() -> Self {
        #[allow(unused_variables)]
        fn noop(vulkan_context: &VulkanContext) {}
        Self {
            window: None,
            render_callback: noop,
            vulkan_context: None,
        }
    }
}

impl<'a> ApplicationHandler for WindowLoop<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        )
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
                if let Some(vulkan_context) = self.vulkan_context {
                    (self.render_callback)(vulkan_context);
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
