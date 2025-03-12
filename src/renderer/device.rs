use ash::vk::QueueFlags;
use ash::{khr, vk, Device, Instance};
use log::info;
use std::error;
use std::ffi::CStr;

use crate::renderer::surface::{VKSurface, VKSwapchainCapabilities};
use crate::renderer::VKInstance;
pub struct VKDevice {
    pub p_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub device: Device,
}

impl VKDevice {
    pub fn new(
        instance: &VKInstance,
        vulkan_surface: &VKSurface,
    ) -> Result<Self, Box<dyn error::Error>> {
        // Device Requirments should probably be initialised in the Vulkan CTX.
        // With the possibility for the Engine user to append their own-
        // requirments, Possibly by requesting a mutable reference to-
        // base extentions before device setup.
        let mut dev_requirments = VKDeviceRequirments::default()
            .add_queue_flag(vk::QueueFlags::GRAPHICS)
            .push_ext(khr::swapchain::NAME)
            .push_ext(khr::dynamic_rendering::NAME)
            .push_ext(khr::synchronization2::NAME)
            .push_info(
                vk::PhysicalDeviceDynamicRenderingFeaturesKHR::default().dynamic_rendering(true),
            )
            .push_info(
                vk::PhysicalDeviceSynchronization2FeaturesKHR::default().synchronization2(true),
            )
            .push_fn(|physical_device, instance, _| {
                let device_properties =
                    unsafe { instance.get_physical_device_properties(*physical_device) };
                // Declare llvmpipe virtual gpu as incompatible
                !device_properties
                    .device_name_as_c_str()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("llvmpipe")
            })
            .push_fn(|physical_device, _, vk_surface: Option<&VKSurface>| {
                if let Some(vk_surface) = vk_surface {
                    let swap_capabilities =
                        VKSwapchainCapabilities::new(vk_surface, *physical_device).unwrap();

                    swap_capabilities.surface_capibilities.min_image_count > 0
                        || !swap_capabilities.present_modes.is_empty()
                } else {
                    true
                }
            });
        // there is no way for the scoring function to be changed by the user then why have it passed as an argument.
        // possibly make device picking a struct with changable defaults.
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

        // Setup Logical Device (Set Features, Enable Extentions, Configure Extentions)

        let priorities = [1.0f32];

        let queue_create_infos = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(ideal_graphics_queue)
            .queue_priorities(&priorities);

        let features = vk::PhysicalDeviceFeatures::default();

        // array of Requested Device extension_names as c string ptr
        let device_extension_names = dev_requirments.get_requirments_raw();

        // Dynamic Rendering Ext configuration
        // Syncronization2 Ext configuration
        // TODO: Feature Configuration Should probably be settable Outside of here maybe in Device Requirments Struct
        // let mut dynamic_rendering_feature =
        // vk::PhysicalDeviceDynamicRenderingFeaturesKHR::default().dynamic_rendering(true);

        // let mut synchronization2_feature =
        // vk::PhysicalDeviceSynchronization2FeaturesKHR::default().synchronization2(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&device_extension_names)
            .enabled_features(&features)
            .queue_create_infos(std::slice::from_ref(&queue_create_infos));

        let device_create_info = dev_requirments
            .device_extended_info
            .iter_mut()
            .fold(device_create_info, |dev_info, info| {
                dev_info.push_next(info.as_mut())
            });

        //Create Logical Device
        let device = unsafe {
            instance
                .instance
                .create_device(p_device, &device_create_info, None)?
        };

        // Get Graphics queue for logical devices
        let graphics_queue = unsafe { device.get_device_queue(ideal_graphics_queue, 0u32) };

        Ok(Self {
            p_device,
            device,
            graphics_queue,
        })
    }

    fn pick_device<F>(
        instance: &Instance,
        score_function: F,
        dev_requirments: &VKDeviceRequirments,
        vulkan_surface: &VKSurface,
    ) -> Result<(vk::PhysicalDevice, u32 /* queue_index */), Box<dyn error::Error>>
    where
        F: Fn(&vk::PhysicalDevice, &Instance) -> u64,
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

    /// # Safety
    /// Read VK Docs For Destruction Order
    /// Device must be destroyed before the instance
    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();
        self.device.destroy_device(None);
    }
}

/// Function for Checking Requirments
type ReqFn<'a> = Box<dyn Fn(&vk::PhysicalDevice, &Instance, Option<&VKSurface>) -> bool + 'a>;

/// Struct for holding and testing Device Requirments
/// Example Use:
/// ```
/// let physical_device = ...;
/// let DeviceRequirments = DeviceRequirments::default().push_ext(ash::khr::dynamic_rendering::NAME);
/// printf("Compatible {:?}", DeviceRequirments.check_device(physical_device));
/// ```
pub struct VKDeviceRequirments<'a> {
    pub required_extentions: Vec<&'static CStr>,
    pub device_extended_info: Vec<Box<dyn vk::ExtendsDeviceCreateInfo + 'a>>,
    pub requirement_functions: Vec<ReqFn<'a>>,
    pub required_queue_flags: vk::QueueFlags,
}

impl<'a> VKDeviceRequirments<'a> {
    /// Adds a vulkan extention name to the requirments
    pub fn push_ext(mut self, ext_name: &'static CStr) -> Self {
        self.required_extentions.push(ext_name);
        self
    }

    /// Adds Structures that extend the creation of logical Devices to the requirments
    /// This is so they can be used on logical Device creation
    pub fn push_info<T>(mut self, dev_ext_info: T) -> Self
    where
        T: vk::ExtendsDeviceCreateInfo + 'a,
    {
        self.device_extended_info.push(Box::new(dev_ext_info));
        self
    }

    /// Adds a 'fn(vk::PhysicalDevice, &Instance, Option<&VKSurface>) -> bool' to the device compatability check process
    /// fn must return whether device meats functions requirments.
    pub fn push_fn<F>(mut self, fn_test: F) -> Self
    where
        F: Fn(&vk::PhysicalDevice, &Instance, Option<&VKSurface>) -> bool + 'a,
    {
        self.requirement_functions.push(Box::new(fn_test));
        self
    }

    // add queue flag requirments
    pub fn add_queue_flag(mut self, queue_flag: vk::QueueFlags) -> Self {
        self.required_queue_flags |= queue_flag;
        self
    }

    /// Checks if Physical Device is Compatible
    /// surface_requirment is an optional type for checking if the queue Supports the surface we wan't to display to
    /// checked_queue is an Optional Arguments for Obtaining the Queue Index that was
    // Maybe upgrade to -> Result Type as we currently treat less related errors as an incompatible device
    // Most of the errors are VKResult errors Retainging to memory issues unlikely at early initialisation.
    // TODO: Return Reason for Compatibiliy issue in Result With Custom Error Type
    pub fn device_compat(
        &self,
        physical_device: &vk::PhysicalDevice,
        instance: &Instance,
        surface_requirment: Option<&VKSurface>,
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
            .any(|func| func(physical_device, instance, surface_requirment));

        let queue_family_prop =
            unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

        // first suported queu_prop
        let queue_passes = queue_family_prop.iter().enumerate().any(|queue_prop| {
            let mut suported = queue_prop.1.queue_flags.contains(self.required_queue_flags);
            // if we got passed a surface Requirment Check it is Supported
            if let Some(surface_req) = surface_requirment {
                suported |= surface_req
                    .queue_supports_surface(*physical_device, queue_prop.0 as u32)
                    .unwrap_or(false);
            }
            if let Some(queue_index) = checked_queue.as_mut() {
                if suported {
                    // set supported queue_index to be passed back
                    **queue_index = queue_prop.0 as u32;
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

impl Default for VKDeviceRequirments<'_> {
    fn default() -> Self {
        Self {
            required_extentions: Vec::new(),
            device_extended_info: Vec::new(),
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

    let mesh_shading = device_extensions.iter().any(|extension_prop| {
        extension_prop.extension_name_as_c_str().unwrap_or_default() == ash::ext::mesh_shader::NAME
    });

    // Mesh Shading Modern
    if mesh_shading {
        score += 10;
    }

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

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
