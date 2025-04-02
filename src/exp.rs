use ash::vk;
struct GameInfo {
    // data/options required to initialise vulkan
}

struct VulkanHandles {
    instance: vk::instance,
    device: vk::device,
    // etc ...
}

enum VulkanCTX {
    Initialized(VulkanHandles),
    Unitialized(GameInfo),
}
