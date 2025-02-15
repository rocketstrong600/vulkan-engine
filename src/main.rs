mod renderer;
use ash::vk;
use renderer::device::{physical_device_memory_size, select_vk_physical_device};
use renderer::window;
use renderer::VulkanInstance;
use std::ffi::CStr;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let eng_instance = VulkanInstance::new(c"test", 0, 1, 0, None);
    // Handle error option for instance
    let eng_instance = match eng_instance {
        Ok(instance) => instance,
        Err(error) => panic!("Failed to Create Vulkan Instance: {error:?}"),
    };

    let physical_device_result = select_vk_physical_device(&eng_instance.instance);

    let physical_device = match physical_device_result {
        Ok(physical_device) => physical_device,
        Err(error) => panic!("Failed to pick device: {error:?}"),
    };

    let instance_version = unsafe {
        eng_instance
            .instance
            .get_physical_device_properties(physical_device)
            .api_version
    };

    let device_name = unsafe {
        CStr::from_ptr(
            eng_instance
                .instance
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
        physical_device_memory_size(&physical_device, &eng_instance.instance)
    );

    let event_loop_result = EventLoop::new();

    let event_loop = match event_loop_result {
        Ok(event_loop) => event_loop,
        Err(error) => panic!("Failed to Create Vulkan Instance: {error:?}"),
    };

    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = window::WindowLoop::default();

    if let Err(error) = event_loop.run_app(&mut app) {
        panic!("Failed on EventLoop: {error:?}");
    }
}
