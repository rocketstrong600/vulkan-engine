use ash::{vk, Instance};
use log::{debug, info};
use std::error;

pub struct VulkanDevice {
    pub device: vk::PhysicalDevice,
}

impl VulkanDevice {
    pub fn new(instance: &Instance) -> Result<Self, Box<dyn error::Error>> {
        let device = select_vk_physical_device(instance)?;

        let mut device_properties_two = vk::PhysicalDeviceProperties2::default();

        unsafe { instance.get_physical_device_properties2(device, &mut device_properties_two) };

        let instance_version = device_properties_two.properties.api_version;
        info!(
            "VK Instance Version: {}.{}.{}",
            vk::api_version_major(instance_version),
            vk::api_version_minor(instance_version),
            vk::api_version_patch(instance_version)
        );

        let device_name = device_properties_two.properties.device_name_as_c_str();

        if let Ok(device_name) = device_name {
            info!("VK Device Name: {}", device_name.to_string_lossy());
        }

        info!(
            "VK Device Memory: {}",
            physical_device_memory_size(&device, &instance)
        );

        Ok(Self { device })
    }
}

// we dyn box the error in result to make error inference runtime,
// this is so that the function can support multiple error types.
// these errors are passed on by different functions using ?
fn select_vk_physical_device(
    instance: &Instance,
) -> Result<vk::PhysicalDevice, Box<dyn error::Error>> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    // turn each physical device into tupil containing our score and device
    let mut physical_devices: Vec<(u64, &vk::PhysicalDevice)> = physical_devices
        .iter()
        .map(|physical_device| {
            (
                score_physical_device(physical_device, instance),
                physical_device,
            )
        })
        .collect();

    // sort by the score
    physical_devices.sort_by_key(|device_score| device_score.0);

    // Highest scoring element last in vec
    let physical_device = physical_devices.last().ok_or("No Devices Found")?;
    // return device if score was greater than 0
    if physical_device.0 > 0 {
        Ok(*physical_device.1)
    } else {
        Err("No Suitable Device Found".into())
    }
}

// calculate a capability score for a physical device
// score improvment should go down as importance of property goes down
fn score_physical_device(physical_device: &vk::PhysicalDevice, instance: &Instance) -> u64 {
    let mut score: u64 = 0;
    let device_properties = unsafe { instance.get_physical_device_properties(*physical_device) };

    // llvmpipe virtual gpu can go die in a hole
    if device_properties
        .device_name_as_c_str()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .starts_with("llvmpipe")
    {
        return 0;
    }
    // discrete gpu adds more score than integrated everything else does not improve score
    let device_type = device_properties.device_type;
    match device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => {
            score += 100;
        }
        vk::PhysicalDeviceType::INTEGRATED_GPU => {
            score += 50;
        }
        _ => {}
    }

    let device_features = unsafe { instance.get_physical_device_features(*physical_device) };

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

    // you can't even make a game without a graphics queue
    let graphics_queue = queue_family_properties
        .iter()
        .any(|queue_prop| queue_prop.queue_flags.contains(vk::QueueFlags::GRAPHICS));

    // good cards should be capable of compute
    let compute_queue = queue_family_properties
        .iter()
        .any(|queue_prop| queue_prop.queue_flags.contains(vk::QueueFlags::COMPUTE));

    if !graphics_queue {
        return 0;
    }

    if compute_queue {
        score += 10
    }

    // Cards with Geometry shaders are typically newer
    if device_features.geometry_shader == vk::TRUE {
        score += 10
    }

    // 64 bit floats is not common on low end cards?
    if device_features.shader_float64 == vk::TRUE {
        score += 5
    }

    // add gpu memory to score devices with higer vram tend to be better.
    // capped at 64gb to filter out devices with querks
    score += (physical_device_memory_size(physical_device, instance) / 1024).min(64);
    score
}

// get device memory in MiB
pub fn physical_device_memory_size(
    physical_device: &vk::PhysicalDevice,
    instance: &Instance,
) -> u64 {
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(*physical_device) };

    memory_properties
        .memory_heaps
        .iter()
        .fold(0u64, |acc, heap| {
            if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                acc + heap.size / (1024 * 1024) // Convert to MiB
            } else {
                acc
            }
        })
}
