struct FatVertex
{
    float2 position : SV_POSITION;
    float3 color : COLOR;
};

struct VertInput
{
  float2 position : POSITION;
  float3 color : COLOR;
};

[shader("vertex")]
FatVertex vertexMain(VertInput input)
{
    FatVertex result;

    result.position = input.position;
    result.color = input.color;

    return result;
}

[shader("fragment")]
float4 fragMain(FatVertex input) : SV_TARGET
{
    return float4(input.color, 1.0);
}
