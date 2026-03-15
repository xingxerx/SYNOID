// SYNOID Temporal Denoising Kernel
// Multi-frame noise reduction using temporal coherence

__global__ void temporal_denoise(
    unsigned char* current_frame,
    unsigned char* prev_frame,
    unsigned char* output,
    int width,
    int height,
    int channels,
    float temporal_strength,
    float spatial_threshold
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * channels;

    for (int c = 0; c < channels; c++) {
        float curr = current_frame[idx + c];
        float prev = prev_frame[idx + c];

        // Calculate temporal difference
        float diff = fabsf(curr - prev);

        // Adaptive blending based on motion detection
        // Small diff = likely noise, blend more
        // Large diff = likely motion, preserve current
        float blend_factor = fminf(diff / spatial_threshold, 1.0f);
        blend_factor = 1.0f - (1.0f - blend_factor) * temporal_strength;

        // Temporal averaging with motion adaptation
        float denoised = curr * blend_factor + prev * (1.0f - blend_factor);

        output[idx + c] = (unsigned char)fminf(fmaxf(denoised, 0.0f), 255.0f);
    }
}

// Advanced version with spatial-temporal filtering
__global__ void temporal_denoise_advanced(
    unsigned char* current_frame,
    unsigned char* prev_frame,
    unsigned char* output,
    int width,
    int height,
    int channels,
    float temporal_strength,
    float spatial_threshold
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * channels;

    // Spatial kernel for local averaging
    const int kernel_radius = 1;
    float spatial_weights[9] = {
        0.077847, 0.123317, 0.077847,
        0.123317, 0.195346, 0.123317,
        0.077847, 0.123317, 0.077847
    };

    for (int c = 0; c < channels; c++) {
        float curr_center = current_frame[idx + c];
        float prev_center = prev_frame[idx + c];

        // Spatial averaging around current pixel
        float spatial_sum = 0.0f;
        float weight_sum = 0.0f;

        int ki = 0;
        for (int dy = -kernel_radius; dy <= kernel_radius; dy++) {
            for (int dx = -kernel_radius; dx <= kernel_radius; dx++) {
                int nx = x + dx;
                int ny = y + dy;

                if (nx >= 0 && nx < width && ny >= 0 && ny < height) {
                    int nidx = (ny * width + nx) * channels + c;
                    float neighbor = current_frame[nidx];

                    // Bilateral-style weighting
                    float spatial_dist = sqrtf(dx*dx + dy*dy);
                    float value_dist = fabsf(neighbor - curr_center);

                    float weight = spatial_weights[ki] * expf(-(value_dist * value_dist) / (2.0f * spatial_threshold * spatial_threshold));

                    spatial_sum += neighbor * weight;
                    weight_sum += weight;
                }
                ki++;
            }
        }

        float spatial_filtered = spatial_sum / fmaxf(weight_sum, 1e-6f);

        // Temporal blending
        float temporal_diff = fabsf(spatial_filtered - prev_center);
        float blend_factor = fminf(temporal_diff / spatial_threshold, 1.0f);
        blend_factor = 1.0f - (1.0f - blend_factor) * temporal_strength;

        float denoised = spatial_filtered * blend_factor + prev_center * (1.0f - blend_factor);

        output[idx + c] = (unsigned char)fminf(fmaxf(denoised, 0.0f), 255.0f);
    }
}

// Launch configuration:
// Block: (16, 16, 1)
// Grid: ((width + 15) / 16, (height + 15) / 16, 1)
