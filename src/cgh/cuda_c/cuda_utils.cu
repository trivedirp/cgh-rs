// real and img vector to a c32 vector
extern "C" __global__ void floattocplx(
    const float* real,
    const float* imag,
    float2* out,
    int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        out[idx].x = real[idx];
        out[idx].y = imag[idx];
    }
}

// abs and arg vector to a c32 vector   
extern "C" __global__ void absargtocplx(
    const float* abs,
    const float* arg,
    float2* out,
    int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        float a = abs[idx];
        float p = arg[idx];
        out[idx].x = a * cosf(p);
        out[idx].y = a * sinf(p);
    }
}

// get arg from c32 vector
extern "C" __global__ void get_arg(
    const float2* input, 
    float* output,
    int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        float re = input[idx].x;
        float im = input[idx].y;
        output[idx] = atan2f(im, re);
    }
}

// get_abs of c32 vector
extern "C" __global__ void get_abs(
    const float2* input, 
    float* output,
    int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        float re = input[idx].x;
        float im = input[idx].y;
        output[idx] = sqrtf(re * re + im * im);
    }
}

// binarize float to u8
extern "C" __global__ void binarize(
    const float* arg,
    unsigned char* out,
    int n,
    int n_bins
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        float val = fabsf(arg[idx]) * (n_bins - 1) / (2.0f * 3.141592653589793f);
        out[idx] = (unsigned char)floorf(val);
    }
}

//  rotate_xy.cu
extern "C" __global__ void rotate_xy(
    const int* x,
    const int* y,
    int* out_x,
    int* out_y,
    int n,
    float angle_rad
) {
    float c = cosf(angle_rad);
    float s = sinf(angle_rad);
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        float xf = (float)x[idx];
        float yf = (float)y[idx];
        float rx = c * xf - s * yf;
        float ry = s * xf + c * yf;
        out_x[idx] = (int)floorf(rx);
        out_y[idx] = (int)floorf(ry);
    }
}

extern "C" __global__ void compute_slm_phase(
    float* slm_ph_mod2pi,
    int slm_size_x,
    int slm_size_y,
    int slmc_x,
    int slmc_y,
    float dk,
    float k_tot_sq,
    float z_shift,
    float pitch_x_pix,
    float pitch_y_pix
) {
    float fl_2pi = 2.0f * 3.141592653589793f;
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x >= slm_size_x || y >= slm_size_y) return;
    int idx = y * slm_size_x + x;
        float k_x = (x - slmc_x) * dk;
        float k_y = (y - slmc_y) * dk;
        float k_xy_sq = k_x * k_x + k_y * k_y;
        float k_z = sqrtf(k_tot_sq - k_xy_sq);
        float phz = z_shift * k_z;
        float phx = fmodf(x, pitch_x_pix) * fl_2pi / pitch_x_pix;
        float phy = fmodf(y, pitch_y_pix) * fl_2pi / pitch_y_pix;
        float slm_ph = phx + phy + phz;
        slm_ph_mod2pi[idx] = fmodf(fmodf(slm_ph, fl_2pi) + fl_2pi, fl_2pi);
}