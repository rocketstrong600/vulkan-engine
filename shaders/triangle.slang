struct FatVertex
{
    float4 position : SV_POSITION;
    float3 color : COLOR;
};

struct VertInput
{
  float3 position : POSITION;
  float3 color : COLOR;
};

struct CameraData {
    float4x4 cameraMatrix;
};

[[vk::push_constant]]
ConstantBuffer<CameraData> camera;

[shader("vertex")]
FatVertex vertexMain(VertInput input)
{
    FatVertex result;

    result.position = mul(camera.cameraMatrix,float4(input.position, 1.0));
    result.color = input.color;

    return result;
}

[shader("fragment")]
float4 fragMain(FatVertex input) : SV_TARGET
{
    return float4(input.color, 1.0);
}
