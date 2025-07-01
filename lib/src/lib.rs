use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// A simple calculator struct that can be used from C/C++
#[repr(C)]
pub struct Calculator {
    value: f64,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator { value: 0.0 }
    }

    pub fn add(&mut self, x: f64) {
        self.value += x;
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }
}

/// Create a new calculator instance
#[no_mangle]
pub extern "C" fn calculator_new() -> *mut Calculator {
    Box::into_raw(Box::new(Calculator::new()))
}

/// Free a calculator instance
#[no_mangle]
pub extern "C" fn calculator_free(calc: *mut Calculator) {
    if !calc.is_null() {
        unsafe {
            let _ = Box::from_raw(calc);
        }
    }
}

/// Add a value to the calculator
#[no_mangle]
pub extern "C" fn calculator_add(calc: *mut Calculator, value: f64) {
    if !calc.is_null() {
        unsafe {
            (*calc).add(value);
        }
    }
}

/// Get the current value from the calculator
#[no_mangle]
pub extern "C" fn calculator_get_value(calc: *const Calculator) -> f64 {
    if !calc.is_null() {
        unsafe { (*calc).get_value() }
    } else {
        0.0
    }
}

/// Simple string processing function
#[no_mangle]
pub extern "C" fn process_string(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let input_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let processed = format!("Processed: {}", input_str.to_uppercase());
    
    match CString::new(processed) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string allocated by process_string
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Simple math function
#[no_mangle]
pub extern "C" fn multiply_by_two(x: c_int) -> c_int {
    x * 2
}

/// Get library version
#[no_mangle]
pub extern "C" fn get_version() -> *const c_char {
    "1.0.0\0".as_ptr() as *const c_char
}