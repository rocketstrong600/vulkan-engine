mod app;
mod renderer;
mod utils;

use crate::utils::GameInfo;

use simple_logger::SimpleLogger;
use winit::event_loop::EventLoop;

fn main() {
    let logger = SimpleLogger::new().with_level(log::LevelFilter::Info);

    match logger.init() {
        Ok(l) => l,
        Err(e) => {
            println!("Logger Init Error: {}", e);
        }
    }

    let game_info = GameInfo {
        app_name: c"Test",
        major: 0,
        minor: 0,
        patch: 1,
    };

    let event_loop_result = EventLoop::new();

    let mut event_loop = match event_loop_result {
        Ok(event_loop) => event_loop,
        Err(error) => panic!("Failed to Create Event Loop: {error:?}"),
    };

    let mut app = crate::app::App::new(game_info);

    /* // old code to list required extensions for window manager surface
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
