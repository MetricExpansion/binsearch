use std::{ptr::null_mut};

use floatrun::FloatRun;

mod c {
    use std::os::raw::{c_uchar, c_float};
    extern "C" {
        pub fn search(found_range: *mut *const c_uchar, found_length: *mut usize, data: *const c_uchar, length: usize, min: c_float, use_min: bool, max: c_float, use_max: bool, min_length: usize);
    }
}

pub fn search(data: &[u8], min: Option<f32>, max: Option<f32>, min_length: usize) -> (Option<FloatRun>, &[u8]) {
    let mut result_ptr: *const u8 = null_mut();
    let mut result_len = 0;
    unsafe {
        c::search(&mut result_ptr, &mut result_len, data.as_ptr(), data.len(), min.unwrap_or_else(|| 0.0), min.is_some(), max.unwrap_or_else(|| 0.0), max.is_some(), min_length);
        if !result_ptr.is_null() {
            let result = std::slice::from_raw_parts(result_ptr, result_len);
            let remain = std::slice::from_raw_parts(result_ptr.add(result_len), data.as_ptr_range().end as usize - (result_ptr.add(result_len) as usize));
            (Some(FloatRun { address: result_ptr, values: result.chunks(4).map(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]])).collect() }), remain)
        } else {
            (None, data)
        }
    }
}
