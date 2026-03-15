// SYNOID Color Grading Kernel
// High-performance LUT-based color grading for cinematic looks

__global__ void color_grading_lut(
    unsigned char* input,
    unsigned char* output,
    float* lut,  // 3D LUT (64x64x64 typical)
    int width,
    int height,
    int lut_size,
    float intensity
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * 3;  // RGB channels

    // Read input pixel
    float r = input[idx + 0] / 255.0f;
    float g = input[idx + 1] / 255.0f;
    float b = input[idx + 2] / 255.0f;

    // 3D LUT lookup with trilinear interpolation
    float lut_coord_r = r * (lut_size - 1);
    float lut_coord_g = g * (lut_size - 1);
    float lut_coord_b = b * (lut_size - 1);

    int ir = (int)lut_coord_r;
    int ig = (int)lut_coord_g;
    int ib = (int)lut_coord_b;

    float fr = lut_coord_r - ir;
    float fg = lut_coord_g - ig;
    float fb = lut_coord_b - ib;

    // Clamp indices
    ir = min(ir, lut_size - 2);
    ig = min(ig, lut_size - 2);
    ib = min(ib, lut_size - 2);

    // Sample 8 corners of LUT cube
    #define LUT_IDX(r,g,b,c) (((r)*lut_size*lut_size + (g)*lut_size + (b))*3 + (c))

    float c000[3], c001[3], c010[3], c011[3], c100[3], c101[3], c110[3], c111[3];

    for (int c = 0; c < 3; c++) {
        c000[c] = lut[LUT_IDX(ir,   ig,   ib,   c)];
        c001[c] = lut[LUT_IDX(ir,   ig,   ib+1, c)];
        c010[c] = lut[LUT_IDX(ir,   ig+1, ib,   c)];
        c011[c] = lut[LUT_IDX(ir,   ig+1, ib+1, c)];
        c100[c] = lut[LUT_IDX(ir+1, ig,   ib,   c)];
        c101[c] = lut[LUT_IDX(ir+1, ig,   ib+1, c)];
        c110[c] = lut[LUT_IDX(ir+1, ig+1, ib,   c)];
        c111[c] = lut[LUT_IDX(ir+1, ig+1, ib+1, c)];
    }

    // Trilinear interpolation
    float result[3];
    for (int c = 0; c < 3; c++) {
        float c00 = c000[c] * (1-fr) + c100[c] * fr;
        float c01 = c001[c] * (1-fr) + c101[c] * fr;
        float c10 = c010[c] * (1-fr) + c110[c] * fr;
        float c11 = c011[c] * (1-fr) + c111[c] * fr;

        float c0 = c00 * (1-fg) + c10 * fg;
        float c1 = c01 * (1-fg) + c11 * fg;

        result[c] = c0 * (1-fb) + c1 * fb;
    }

    // Blend with original based on intensity
    float out_r = r * (1 - intensity) + result[0] * intensity;
    float out_g = g * (1 - intensity) + result[1] * intensity;
    float out_b = b * (1 - intensity) + result[2] * intensity;

    // Write output
    output[idx + 0] = (unsigned char)(fminf(fmaxf(out_r * 255.0f, 0.0f), 255.0f));
    output[idx + 1] = (unsigned char)(fminf(fmaxf(out_g * 255.0f, 0.0f), 255.0f));
    output[idx + 2] = (unsigned char)(fminf(fmaxf(out_b * 255.0f, 0.0f), 255.0f));
}

// Launch configuration:
// Block: (16, 16, 1)
// Grid: ((width + 15) / 16, (height + 15) / 16, 1)
