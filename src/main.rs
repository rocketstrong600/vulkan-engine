use ash::{vk, Entry};
mod device;

fn main() {
    // Load Vulkan Library
    let entry_result = unsafe { Entry::load() };

    // Handle error option for load
    let entry = match entry_result {
        Ok(result) => result,
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

    //Cleanup vulkan instance
    unsafe { instance.destroy_instance(None) };
}
