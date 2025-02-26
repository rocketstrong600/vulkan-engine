use ash::vk::QueueFlags;
use ash::{khr, vk, Device, Instance};
use log::info;
use std::error;
use std::ffi::CStr;

use crate::renderer::surface::VulkanSurface;
use crate::renderer::VulkanInstance;
pub struct VulkanDevice {
    pub p_device: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_queue: vk::Queue,
}

impl VulkanDevice {
    pub fn new(
        instance: &VulkanInstance,
        vulkan_surface: &VulkanSurface,
    ) -> Result<Self, Box<dyn error::Error>> {
        let dev_requirments = DeviceRequirments::default()
            .push_queue_flag(vk::QueueFlags::GRAPHICS)
            .push_ext(khr::dynamic_rendering::NAME)
            .push_ext(khr::swapchain::NAME)
            .push_fn(|physical_device, instance| {
                let device_properties =
                    unsafe { instance.get_physical_device_properties(*physical_device) };
                // llvmpipe virtual gpu can go die in a hole
                !device_properties
                    .device_name_as_c_str()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("llvmpipe")
            });

        let (p_device, ideal_graphics_queue) = Self::pick_device(
            &instance.instance,
            score_physical_device,
            &dev_requirments,
            vulkan_surface,
        )?;

        let mut device_properties_two = vk::PhysicalDeviceProperties2::default();

        unsafe {
            instance
                .instance
                .get_physical_device_properties2(p_device, &mut device_properties_two)
        };

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
            physical_device_memory_size(&p_device, &instance.instance)
        );

        let priorities = [1.0f32];

        let queue_create_infos = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(ideal_graphics_queue)
            .queue_priorities(&priorities);

        let features = vk::PhysicalDeviceFeatures::default();

        // array of device enable extension_names c string ptr
        let device_extension_names = dev_requirments.get_requirments_raw();

        //Dynamic Rendering Device Featuers
        let mut dynamic_rendering_feature =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::default().dynamic_rendering(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&device_extension_names)
            .enabled_features(&features)
            .queue_create_infos(std::slice::from_ref(&queue_create_infos))
            .push_next(&mut dynamic_rendering_feature);

        let device = unsafe {
            instance
                .instance
                .create_device(p_device, &device_create_info, None)?
        };

        let graphics_queue = unsafe { device.get_device_queue(ideal_graphics_queue, 0u32) };

        Ok(Self {
            p_device,
            device,
            graphics_queue,
        })
    }

    fn pick_device<F, B>(
        instance: &Instance,
        score_function: F,
        dev_requirments: &DeviceRequirments<B>,
        vulkan_surface: &VulkanSurface,
    ) -> Result<(vk::PhysicalDevice, u32 /* queue_index */), Box<dyn error::Error>>
    where
        F: Fn(&vk::PhysicalDevice, &Instance) -> u64,
        B: Fn(&vk::PhysicalDevice, &Instance) -> bool,
    {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };

        let mut queue_index = 0;

        let physical_devices: Vec<(&vk::PhysicalDevice, u32)> = physical_devices
            .iter()
            .filter_map(|p_device| {
                dev_requirments
                    .device_compat(
                        p_device,
                        instance,
                        Some(vulkan_surface),
                        Some(&mut queue_index),
                    )
                    .then_some((p_device, queue_index))
            })
            .collect();

        // turn each physical device into tupil containing our score and device
        let mut physical_devices: Vec<(u64, &vk::PhysicalDevice, u32)> = physical_devices
            .iter()
            .map(|physical_device| {
                let score = score_function(physical_device.0, instance);
                (score, physical_device.0, physical_device.1)
            })
            .collect();

        // sort by the score
        physical_devices.sort_by_key(|device_score| device_score.0);

        // Highest scoring element last in vec
        let physical_device = physical_devices.last().ok_or("No Suitable Devices Found")?;
        // return device if score was greater than 0
        Ok((*physical_device.1, physical_device.2))
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            //must be dropped before instance
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
        };
    }
}

/// Struct for holding and testing Device Requirments
/// Example Use:
/// ```
/// let physical_device = ...;
/// let DeviceRequirments = DeviceRequirments::default().push_ext(ash::khr::dynamic_rendering::NAME);
/// printf("Compatible {:?}", DeviceRequirments.check_device(physical_device));
/// ```
pub struct DeviceRequirments<F>
where
    F: Fn(&vk::PhysicalDevice, &Instance) -> bool,
{
    required_extentions: Vec<&'static CStr>,
    requirement_functions: Vec<F>,
    required_queue_flags: vk::QueueFlags,
}

impl<F> DeviceRequirments<F>
where
    F: Fn(&vk::PhysicalDevice, &Instance) -> bool,
{
    /// Adds a vulkan extention name to the requirments
    pub fn push_ext(mut self, ext_name: &'static CStr) -> Self {
        self.required_extentions.push(ext_name);
        self
    }

    /// Adds a 'fn(vk::PhysicalDevice, &Instance) -> bool' to the device compatability check process
    /// fn must return whether device meats functions requirments.
    pub fn push_fn(mut self, fn_test: F) -> Self {
        self.requirement_functions.push(fn_test);
        self
    }

    // add queue flag requirments
    pub fn push_queue_flag(mut self, queue_flag: vk::QueueFlags) -> Self {
        self.required_queue_flags |= queue_flag;
        self
    }

    /// Checks if Physical Device is Compatible
    pub fn device_compat(
        &self,
        physical_device: &vk::PhysicalDevice,
        instance: &Instance,
        surface_requirment: Option<&VulkanSurface>,
        mut checked_queue: Option<&mut u32>,
    ) -> bool {
        let device_extentions = unsafe {
            instance
                .enumerate_device_extension_properties(*physical_device)
                .unwrap_or_default()
        };

        let device_extentions: Vec<&CStr> = device_extentions
            .iter()
            .map(|ext_prop| ext_prop.extension_name_as_c_str().unwrap_or_default())
            .collect();

        let has_extentions = self
            .required_extentions
            .iter()
            .all(|extention| device_extentions.contains(extention));

        let funcs_passes = self
            .requirement_functions
            .iter()
            .any(|func| func(physical_device, instance));

        let queue_family_prop =
            unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

        // first suported queu_prop
        let queue_passes = queue_family_prop.iter().enumerate().any(|queue_prop| {
            let mut suported = queue_prop.1.queue_flags.contains(self.required_queue_flags);
            if let Some(surface_req) = surface_requirment {
                suported |= surface_req
                    .queue_supports_surface(*physical_device, queue_prop.0 as u32)
                    .unwrap_or(false);
                if suported {
                    if let Some(queue_index) = checked_queue.as_mut() {
                        **queue_index = queue_prop.0 as u32;
                    }
                }
            }
            suported
        });

        has_extentions && funcs_passes && queue_passes
    }

    pub fn get_requirments(&self) -> &[&'static CStr] {
        self.required_extentions.as_slice()
    }

    pub fn get_requirments_raw(&self) -> Vec<*const std::ffi::c_char> {
        self.required_extentions
            .iter()
            .map(|req| req.as_ptr())
            .collect()
    }
}

impl<F> Default for DeviceRequirments<F>
where
    F: Fn(&vk::PhysicalDevice, &Instance) -> bool,
{
    fn default() -> Self {
        Self {
            required_extentions: Vec::new(),
            requirement_functions: Vec::new(),
            required_queue_flags: QueueFlags::empty(),
        }
    }
}

// calculate a capability score for a physical device
// score improvment should go down as importance of property goes down
fn score_physical_device(physical_device: &vk::PhysicalDevice, instance: &Instance) -> u64 {
    let mut score: u64 = 0;
    let device_properties = unsafe { instance.get_physical_device_properties(*physical_device) };

    // llvmpipe virtual gpu can go die in a hole
    // if device_properties
    //     .device_name_as_c_str()
    //     .unwrap_or_default()
    //     .to_string_lossy()
    //     .starts_with("llvmpipe")
    // {
    //     return None;
    // }
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

    let device_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(*physical_device)
            .unwrap_or_default()
    };

    // let dynamic_rendering = device_extensions.iter().any(|extension_prop| {
    //     extension_prop.extension_name_as_c_str().unwrap_or_default()
    //         == ash::khr::dynamic_rendering::NAME
    // });

    let mesh_shading = device_extensions.iter().any(|extension_prop| {
        extension_prop.extension_name_as_c_str().unwrap_or_default() == ash::ext::mesh_shader::NAME
    });

    // Require Dynamic Rendering
    // if !dynamic_rendering {
    //     return None;
    // }

    // Mesh Shading Modern
    if mesh_shading {
        score += 10;
    }

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

    // you can't even make a game without a graphics queue
    // let suitable_queue = IdealGraphicsQueue::find_queue(
    //     *physical_device,
    //     queue_family_properties.clone(),
    //     vulkan_surface,
    // );
    // good cards should be capable of compute
    let compute_queue = queue_family_properties
        .iter()
        .any(|queue_prop| queue_prop.queue_flags.contains(vk::QueueFlags::COMPUTE));

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
