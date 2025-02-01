mod device;
mod gpu;
use ash::{
    vk::{self, AccelerationStructureTrianglesDisplacementMicromapNV},
    Entry,
};
use device::{physical_device_memory_size, select_vk_physical_device};
use std::ffi::CStr;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
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
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    // Load Vulkan Library
    let entry_result = unsafe { Entry::load() };

    // Handle error option for load
    let entry = match entry_result {
        Ok(entry) => entry,
        Err(error) => panic!("Failed to load Vulkan: {error:?}"),
    };

    // Create Default Struct to get written into by entry. create instance
    let app_info = vk::ApplicationInfo {
        api_version: vk::make_api_version(0, 1, 0, 0),
        ..Default::default()
    };

    let create_info = vk::InstanceCreateInfo {
        p_application_info: &app_info,
        ..Default::default()
    };

    let instance_result = unsafe { entry.create_instance(&create_info, None) };

    // Handle error option for instance
    let instance = match instance_result {
        Ok(instance) => instance,
        Err(error) => panic!("Failed to Create Vulkan Instance: {error:?}"),
    };

    let physical_device_result = select_vk_physical_device(&instance);

    let physical_device = match physical_device_result {
        Ok(physical_device) => physical_device,
        Err(error) => panic!("Failed to pick device: {error:?}"),
    };

    let instance_version = unsafe {
        instance
            .get_physical_device_properties(physical_device)
            .api_version
    };

    let device_name = unsafe {
        CStr::from_ptr(
            instance
                .get_physical_device_properties(physical_device)
                .device_name
                .as_ptr(),
        )
        .to_string_lossy()
    };

    let major = vk::api_version_major(instance_version);
    let minor = vk::api_version_minor(instance_version);
    let patch = vk::api_version_patch(instance_version);

    println!(
        "Device Name: {}\nVulkan Instance Version: {}.{}.{}",
        device_name, major, minor, patch
    );
    println!(
        "Device Memory: {}MiB",
        physical_device_memory_size(&physical_device, &instance)
    );

    let event_loop_result = EventLoop::new();

    let event_loop = match event_loop_result {
        Ok(event_loop) => event_loop,
        Err(error) => panic!("Failed to Create Vulkan Instance: {error:?}"),
    };

    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app);
    //Cleanup vulkan instance
    unsafe { instance.destroy_instance(None) };
}
