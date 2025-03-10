use ash::util::read_spv;
use ash::vk;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::hash::Hash;
use std::path::Path;

use crate::renderer::device::VKDevice;

pub struct VKShader<'a> {
    pub shader_module: vk::ShaderModule,
    pub shader_info: vk::PipelineShaderStageCreateInfo<'a>,
}

impl<'a> VKShader<'a> {
    pub fn new(
        vk_device: &VKDevice,
        shader_path: &'static str,
        shader_stage: vk::ShaderStageFlags,
        shader_entry: &'static CStr,

        vk_shader_loader: &mut VKShaderLoader<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file_data = vk_shader_loader.load_shader(shader_path)?;
        let create_info = vk::ShaderModuleCreateInfo::default().code(file_data);
        let shader_module = unsafe { vk_device.device.create_shader_module(&create_info, None)? };

        let create_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(shader_stage)
            .module(shader_module)
            .name(shader_entry);
        Ok(Self {
            shader_module,
            shader_info: create_info,
        })
    }

    pub fn destroy(&mut self, vk_device: &VKDevice) {
        unsafe {
            vk_device
                .device
                .destroy_shader_module(self.shader_module, None)
        };
    }
}

// Probably be replaced with future asset System
#[derive(Default)]
pub struct VKShaderLoader<P>
where
    P: AsRef<Path> + Eq + Hash,
{
    pub files: HashMap<P, Result<Vec<u32>, std::io::Error>>,
}

impl<P> VKShaderLoader<P>
where
    P: AsRef<Path> + Eq + Hash + Clone,
{
    pub fn load_shader(&mut self, path: P) -> Result<&Vec<u32>, std::io::Error> {
        if path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("spirv") {
            let file_data = self.files.entry(path).or_insert_with_key(|path| {
                let mut file = File::open(path)?;
                read_spv(&mut file)
            });
            file_data
                .as_ref()
                .map_err(|err| std::io::Error::new(err.kind(), err.to_string()))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Wrong File Extention",
            ))
        }
    }
}
