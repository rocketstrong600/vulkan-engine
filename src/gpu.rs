pub mod device;

use ash::{vk, Entry, Instance};
use std::error;
use std::ffi::{c_char, CString};
use std::ptr;

pub struct VulkanInstance {
    entry: Entry,
    pub instance: Instance,
}

impl VulkanInstance {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        // Load Vulkan Library
        let entry = unsafe { Entry::load()? };

        let app_info = vk::ApplicationInfo::default().api_version(vk::make_api_version(0, 1, 3, 0));

        let instance = Self::create_instance(&entry, &app_info, &[])?;

        Ok(Self {
            entry: entry,
            instance: instance,
        })
    }

    fn create_instance(
        entry: &Entry,
        app_info: &vk::ApplicationInfo,
        extension_names: &[*const c_char],
    ) -> Result<Instance, Box<dyn error::Error>> {
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(app_info)
            .enabled_extension_names(extension_names);
        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(instance)
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
