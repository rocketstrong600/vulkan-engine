use simple_logger::SimpleLogger;
use vulkan_engine::app::App;
use vulkan_engine::utils::GameInfo;
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

    let mut app = App::new(game_info);

    if let Err(error) = app.start(&mut event_loop) {
        panic!("Failed on EventLoop: {error:?}");
    }
}
