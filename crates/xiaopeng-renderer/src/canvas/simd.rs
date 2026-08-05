use xiaopeng_common::Color;
use tiny_skia::PremultipliedColorU8;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn blend_text_mask_avx2(
    mask: &[u8],
    pixels: &mut [PremultipliedColorU8],
    color: &Color,
) {
    let len = mask.len();
    let mut i = 0;

    let color_r = color.r as i32;
    let color_g = color.g as i32;
    let color_b = color.b as i32;
    let color_a = color.a as i32;

    // Load base color as 32-bit floats for fast multiplication
    let v_r = _mm256_set1_ps(color_r as f32);
    let v_g = _mm256_set1_ps(color_g as f32);
    let v_b = _mm256_set1_ps(color_b as f32);
    let v_a = _mm256_set1_ps(color_a as f32);
    let v_255_inv = _mm256_set1_ps(1.0 / 255.0);

    while i + 8 <= len {
        // Load 8 bytes of mask alpha
        // _mm_loadl_epi64 loads 64-bit (8 bytes).
        let mask_ptr = mask.as_ptr().add(i) as *const i64;
        let mask_u8 = _mm_loadl_epi64(mask_ptr as *const __m128i);
        
        // Convert 8 bytes of u8 into 8 ints of i32 (AVX2 256-bit register)
        let mask_i32 = _mm256_cvtepu8_epi32(mask_u8);
        
        // Convert to float
        let mask_f32 = _mm256_cvtepi32_ps(mask_i32);
        
        // Calculate effective alpha: (mask * color_a) / 255.0
        let a_f32 = _mm256_mul_ps(_mm256_mul_ps(mask_f32, v_a), v_255_inv);
        
        // Premultiply R, G, B
        let r_f32 = _mm256_mul_ps(_mm256_mul_ps(v_r, a_f32), v_255_inv);
        let g_f32 = _mm256_mul_ps(_mm256_mul_ps(v_g, a_f32), v_255_inv);
        let b_f32 = _mm256_mul_ps(_mm256_mul_ps(v_b, a_f32), v_255_inv);
        
        // Convert back to i32
        let a_i32 = _mm256_cvtps_epi32(a_f32);
        let r_i32 = _mm256_cvtps_epi32(r_f32);
        let g_i32 = _mm256_cvtps_epi32(g_f32);
        let b_i32 = _mm256_cvtps_epi32(b_f32);
        
        // Now we need to pack R, G, B, A back into 8 PremultipliedColorU8.
        // PremultipliedColorU8 is 4 bytes. We need to interleave R, G, B, A into a single 32-bit int per pixel.
        // Format of PremultipliedColorU8 in tiny_skia is typically [R, G, B, A] in memory (rgba) or [B, G, R, A] (bgra).
        // Since we are writing safe code inside the loop, we can extract the 8 integers and write them cleanly
        // to avoid incorrect endianness assumptions.
        // (SIMD extraction takes a bit, but it's still extremely fast compared to scalar).
        
        let mut a_arr = [0i32; 8];
        let mut r_arr = [0i32; 8];
        let mut g_arr = [0i32; 8];
        let mut b_arr = [0i32; 8];
        
        _mm256_storeu_si256(a_arr.as_mut_ptr() as *mut __m256i, a_i32);
        _mm256_storeu_si256(r_arr.as_mut_ptr() as *mut __m256i, r_i32);
        _mm256_storeu_si256(g_arr.as_mut_ptr() as *mut __m256i, g_i32);
        _mm256_storeu_si256(b_arr.as_mut_ptr() as *mut __m256i, b_i32);

        for j in 0..8 {
            pixels[i + j] = tiny_skia::PremultipliedColorU8::from_rgba(
                r_arr[j] as u8,
                g_arr[j] as u8,
                b_arr[j] as u8,
                a_arr[j] as u8,
            ).unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
        }
        
        i += 8;
    }

    // Scalar fallback for remaining pixels
    while i < len {
        let alpha = mask[i];
        let a = ((alpha as u16 * color.a as u16) / 255) as u8;
        let c = tiny_skia::ColorU8::from_rgba(color.r, color.g, color.b, a);
        pixels[i] = c.premultiply();
        i += 1;
    }
}

/// SIMD Accelerated RGB to RGBA conversion for Image Decoding.
/// Useful when decoding JPEGs (which are 24-bit RGB) into the 32-bit RGBA 
/// framebuffers required by wgpu and tiny_skia.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn rgb_to_rgba_avx2(rgb: &[u8], rgba: &mut [u8], alpha: u8) {
    let mut i = 0; // index in rgb (3 bytes per pixel)
    let mut j = 0; // index in rgba (4 bytes per pixel)
    let len = rgb.len();

    // AVX2 can process 32 bytes at a time, which doesn't perfectly align with 3-byte RGB.
    // However, we can process 16 pixels at a time: 16 * 3 = 48 bytes of RGB -> 16 * 4 = 64 bytes of RGBA.
    // For simplicity in this engine, we'll process 8 pixels at a time (24 bytes RGB -> 32 bytes RGBA).
    
    // Create a 256-bit vector with the alpha channel broadcasted
    let _v_alpha = _mm256_set1_epi32(alpha as i32); // actually we need it in the highest byte

    // We can do this using scalar for now if the exact shuffle mask is too complex to write blindly,
    // but a proper SIMD implementation uses _mm256_shuffle_epi8 (pshufb).
    // Let's implement a straightforward vectorization for 4 pixels (12 bytes -> 16 bytes) using SSE.
    
    let alpha_mask = _mm_set1_epi32((alpha as i32) << 24);

    while i + 12 <= len {
        // Load 12 bytes of RGB (plus 4 garbage/next bytes which we will mask out)
        // We use _mm_loadu_si128 to load 16 bytes safely if we are not at the end.
        // To be strictly safe against page faults, we just load 12 bytes manually if near end,
        // but let's assume we padded the buffer or we just do scalar if len - i < 16.
        if i + 16 > len { break; }
        
        let chunk = _mm_loadu_si128(rgb.as_ptr().add(i) as *const __m128i);
        
        // RGB RGB RGB RGB ... -> RGBA RGBA RGBA RGBA
        // Shuffle mask to expand 3-byte pixels to 4-byte pixels:
        // Pixel 0: bytes 0, 1, 2, (fill alpha) -> indices 0, 1, 2, 0x80
        // Pixel 1: bytes 3, 4, 5, (fill alpha) -> indices 3, 4, 5, 0x80
        // Pixel 2: bytes 6, 7, 8, (fill alpha) -> indices 6, 7, 8, 0x80
        // Pixel 3: bytes 9, 10, 11, (fill alpha) -> indices 9, 10, 11, 0x80
        
        let shuffle_mask = _mm_set_epi8(
            -1, 11, 10, 9,
            -1, 8, 7, 6,
            -1, 5, 4, 3,
            -1, 2, 1, 0
        );
        
        // pshufb will put 0 where the mask has the highest bit set (-1 = 0xFF).
        let expanded = _mm_shuffle_epi8(chunk, shuffle_mask);
        
        // OR with the alpha mask to set the alpha channel
        let final_rgba = _mm_or_si128(expanded, alpha_mask);
        
        _mm_storeu_si128(rgba.as_mut_ptr().add(j) as *mut __m128i, final_rgba);
        
        i += 12;
        j += 16;
    }

    // Scalar fallback
    while i + 2 < len {
        rgba[j] = rgb[i];
        rgba[j + 1] = rgb[i + 1];
        rgba[j + 2] = rgb[i + 2];
        rgba[j + 3] = alpha;
        i += 3;
        j += 4;
    }
}

