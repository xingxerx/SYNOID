// SYNOID Unsharp Mask Kernel
// High-quality sharpening for video enhancement

__global__ void unsharp_mask(
    unsigned char* input,
    unsigned char* blurred,  // Pre-blurred version
    unsigned char* output,
    int width,
    int height,
    int channels,
    float amount,      // Sharpening strength (typically 0.5 - 2.0)
    float threshold    // Edge threshold to avoid noise amplification
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * channels;

    for (int c = 0; c < channels; c++) {
        float original = input[idx + c];
        float blur = blurred[idx + c];

        // Calculate difference (detail layer)
        float detail = original - blur;

        // Apply threshold to avoid sharpening noise
        if (fabsf(detail) < threshold) {
            detail = 0.0f;
        }

        // Apply unsharp mask: output = original + amount * detail
        float sharpened = original + amount * detail;

        output[idx + c] = (unsigned char)fminf(fmaxf(sharpened, 0.0f), 255.0f);
    }
}

// Single-pass unsharp mask with built-in blur (faster but lower quality)
__global__ void unsharp_mask_fast(
    unsigned char* input,
    unsigned char* output,
    int width,
    int height,
    int channels,
    float amount,
    float threshold
) {
    __shared__ float shared_data[18][18];  // 16x16 + 1-pixel border

    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    int tx = threadIdx.x;
    int ty = threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * channels;

    for (int c = 0; c < channels; c++) {
        // Load into shared memory with border
        shared_data[ty + 1][tx + 1] = input[idx + c];

        // Load borders
        if (tx == 0 && x > 0) {
            shared_data[ty + 1][0] = input[((y * width + (x-1)) * channels) + c];
        }
        if (tx == blockDim.x - 1 && x < width - 1) {
            shared_data[ty + 1][tx + 2] = input[((y * width + (x+1)) * channels) + c];
        }
        if (ty == 0 && y > 0) {
            shared_data[0][tx + 1] = input[(((y-1) * width + x) * channels) + c];
        }
        if (ty == blockDim.y - 1 && y < height - 1) {
            shared_data[ty + 2][tx + 1] = input[(((y+1) * width + x) * channels) + c];
        }

        __syncthreads();

        // 3x3 box blur in shared memory
        float blur_sum = 0.0f;
        for (int dy = 0; dy <= 2; dy++) {
            for (int dx = 0; dx <= 2; dx++) {
                blur_sum += shared_data[ty + dy][tx + dx];
            }
        }
        float blur = blur_sum / 9.0f;

        float original = shared_data[ty + 1][tx + 1];
        float detail = original - blur;

        // Apply threshold
        if (fabsf(detail) < threshold) {
            detail = 0.0f;
        }

        float sharpened = original + amount * detail;

        output[idx + c] = (unsigned char)fminf(fmaxf(sharpened, 0.0f), 255.0f);

        __syncthreads();
    }
}

// Launch configuration:
// Block: (16, 16, 1)
// Grid: ((width + 15) / 16, (height + 15) / 16, 1)
// Shared memory: 2048 bytes (for fast version)
