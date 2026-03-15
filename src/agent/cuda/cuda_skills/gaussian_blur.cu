// SYNOID Gaussian Blur Kernel
// Separable 2-pass Gaussian blur optimized with shared memory

#define KERNEL_RADIUS {RADIUS}
#define KERNEL_SIZE (2 * KERNEL_RADIUS + 1)

__constant__ float gaussian_kernel[KERNEL_SIZE];

// Horizontal blur pass
__global__ void gaussian_blur_horizontal(
    unsigned char* input,
    unsigned char* temp,
    int width,
    int height,
    int channels
) {
    __shared__ float shared_data[16][16 + 2*KERNEL_RADIUS];

    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    int tx = threadIdx.x;
    int ty = threadIdx.y;

    if (y >= height) return;

    // Load data into shared memory with halo
    for (int c = 0; c < channels; c++) {
        // Main data
        if (x < width) {
            int idx = (y * width + x) * channels + c;
            shared_data[ty][tx + KERNEL_RADIUS] = input[idx];
        }

        // Left halo
        if (tx < KERNEL_RADIUS) {
            int halo_x = max(0, x - KERNEL_RADIUS);
            int idx = (y * width + halo_x) * channels + c;
            shared_data[ty][tx] = input[idx];
        }

        // Right halo
        if (tx < KERNEL_RADIUS) {
            int halo_x = min(width - 1, x + blockDim.x);
            int idx = (y * width + halo_x) * channels + c;
            shared_data[ty][tx + blockDim.x + KERNEL_RADIUS] = input[idx];
        }
    }

    __syncthreads();

    // Apply horizontal blur
    if (x < width && y < height) {
        for (int c = 0; c < channels; c++) {
            float sum = 0.0f;

            for (int i = -KERNEL_RADIUS; i <= KERNEL_RADIUS; i++) {
                sum += shared_data[ty][tx + KERNEL_RADIUS + i] * gaussian_kernel[i + KERNEL_RADIUS];
            }

            int idx = (y * width + x) * channels + c;
            temp[idx] = (unsigned char)fminf(fmaxf(sum, 0.0f), 255.0f);
        }
    }
}

// Vertical blur pass
__global__ void gaussian_blur_vertical(
    unsigned char* temp,
    unsigned char* output,
    int width,
    int height,
    int channels
) {
    __shared__ float shared_data[16 + 2*KERNEL_RADIUS][16];

    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    int tx = threadIdx.x;
    int ty = threadIdx.y;

    if (x >= width) return;

    // Load data into shared memory with halo
    for (int c = 0; c < channels; c++) {
        // Main data
        if (y < height) {
            int idx = (y * width + x) * channels + c;
            shared_data[ty + KERNEL_RADIUS][tx] = temp[idx];
        }

        // Top halo
        if (ty < KERNEL_RADIUS) {
            int halo_y = max(0, y - KERNEL_RADIUS);
            int idx = (halo_y * width + x) * channels + c;
            shared_data[ty][tx] = temp[idx];
        }

        // Bottom halo
        if (ty < KERNEL_RADIUS) {
            int halo_y = min(height - 1, y + blockDim.y);
            int idx = (halo_y * width + x) * channels + c;
            shared_data[ty + blockDim.y + KERNEL_RADIUS][tx] = temp[idx];
        }
    }

    __syncthreads();

    // Apply vertical blur
    if (x < width && y < height) {
        for (int c = 0; c < channels; c++) {
            float sum = 0.0f;

            for (int i = -KERNEL_RADIUS; i <= KERNEL_RADIUS; i++) {
                sum += shared_data[ty + KERNEL_RADIUS + i][tx] * gaussian_kernel[i + KERNEL_RADIUS];
            }

            int idx = (y * width + x) * channels + c;
            output[idx] = (unsigned char)fminf(fmaxf(sum, 0.0f), 255.0f);
        }
    }
}

// Launch configuration:
// Block: (16, 16, 1)
// Grid: ((width + 15) / 16, (height + 15) / 16, 1)
// Shared memory: 4096 bytes
