use ash::{vk, Device, Entry, Instance};

fn select_vk_physical_device(instance: &Instance) -> vk::PhysicalDevice {
    let physical_devices_result = unsafe { instance.enumerate_physical_devices() };
    let physical_devices = match physical_devices_result {
        Ok(physical_devices) => physical_devices,
        Err(error) => panic!("Error Getting Devices: {error:?}"),
    };

    let p_device_index_score: (u32, u32) = (0, 0);
    for (index, p_device) in physical_devices.iter().enumerate() {
        // todo
    }
}
