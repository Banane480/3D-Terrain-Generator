use ash::vk;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VegetationInstance {
    pub model_col0: [f32; 4],
    pub model_col1: [f32; 4],
    pub model_col2: [f32; 4],
    pub model_col3: [f32; 4],
    pub variation: [f32; 4], // x = phase vent, y = variation teinte, z = amplitude balancement, w = reserve
}

impl VegetationInstance {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(1)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::INSTANCE)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 5] {
        [
            // location 4: model_col0 vec4
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(4)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(0),
            // location 5: model_col1 vec4
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(5)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(16),
            // location 6: model_col2 vec4
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(6)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(32),
            // location 7: model_col3 vec4
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(7)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(48),
            // location 8: variation vec4
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(8)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(64),
        ]
    }
}
