mod app;
mod renderer;
mod utils;

use crate::utils::GameInfo;

use winit::event_loop::EventLoop;

fn main() {
    let game_info = GameInfo {
        app_name: c"Test",
        major: 0,
        minor: 0,
        patch: 1,
    };

    /*
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
    */
    let event_loop_result = EventLoop::new();

    let mut event_loop = match event_loop_result {
        Ok(event_loop) => event_loop,
        Err(error) => panic!("Failed to Create Event Loop: {error:?}"),
    };

    let mut app = crate::app::App::new(game_info);

    /*
    println!(
        "{:?}",
        renderer::get_winit_vk_ext(&event_loop)
            .unwrap()
            .iter()
            .map(|c_name| { unsafe { CStr::from_ptr(*c_name).to_str().unwrap() } })
            .collect::<Vec<&str>>()
    );
    */

    if let Err(error) = app.start(&mut event_loop) {
        panic!("Failed on EventLoop: {error:?}");
    }
}
