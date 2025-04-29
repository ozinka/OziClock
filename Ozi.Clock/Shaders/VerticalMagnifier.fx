sampler2D input : register(s0);

float centerY;        // normalized center of the lens (0–1)
float bandHeight;     // normalized lens height (e.g., 0.2)
float magnification;  // e.g., 2.0

float4 main(float2 uv : TEXCOORD) : COLOR
{
    float dy = uv.y - centerY;
    float halfBand = bandHeight / 2;

    if (abs(dy) < halfBand)
    {
        uv.y = centerY + dy / magnification;
    }

    return tex2D(input, uv);
}