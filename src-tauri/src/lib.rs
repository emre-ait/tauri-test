use pyo3::prelude::*;
use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
async fn process_image(_app_handle: AppHandle, image_data: String) -> Result<String, String> {
	println!("Rust: Starting Python image processing");
    
    // Get the resource directory path outside of the Python context
    let python_scripts_dir = _app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("python_scripts");
    
    let python_scripts_str = python_scripts_dir.to_str().ok_or("Failed to convert path to string")?;
    
    match Python::with_gil(|py| -> PyResult<String> {
        // Add python_scripts directory to Python path
        let sys = py.import("sys")?;
        sys.getattr("path")?.call_method1("append", (python_scripts_str,))?;
        
        let module = py.import("hello")?;
        let result: String = module
            .getattr("process_image")?
            .call1((image_data,))?
            .extract()?;
        Ok(result)
    }) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
async fn calculate(_app_handle: AppHandle, operation: String, a: f64, b: f64) -> Result<String, String> {
	println!("Rust: Starting Python calculator with {} {} {}", operation, a, b);
    
    // Get the resource directory path outside of the Python context
    let python_scripts_dir = _app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("python_scripts");
    
    let python_scripts_str = python_scripts_dir.to_str().ok_or("Failed to convert path to string")?;
    
    match Python::with_gil(|py| -> PyResult<String> {
        // Add python_scripts directory to Python path
        let sys = py.import("sys")?;
        sys.getattr("path")?.call_method1("append", (python_scripts_str,))?;
        
        let module = py.import("hello")?;
        let result: String = module
            .getattr("calculate")?
            .call1((operation, a, b))?
            .extract()?;
        Ok(result)
    }) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
async fn mif_reader(app_handle: AppHandle, file_path: String, layer_index: i32, variant_index: i32, scale: i32) -> Result<String, String> {
    println!("Rust: Starting MIF reader with file: {}", file_path);
    
    // Get the resource directory path outside of the Python context
    let python_scripts_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("python_scripts");
    
    let python_scripts_str = python_scripts_dir.to_str().ok_or("Failed to convert path to string")?;
    println!("Python scripts path: {}", python_scripts_str);
    
    match Python::with_gil(|py| -> PyResult<String> {
        // Print Python's sys.path for debugging
        let sys = py.import("sys")?;
        println!("Python sys.path before:");
        let path = sys.getattr("path")?.extract::<Vec<String>>()?;
        for p in path.iter() {
            println!("  {}", p);
        }
        
        // Add python_scripts directory to Python path
        sys.getattr("path")?.call_method1("append", (python_scripts_str,))?;
        
        println!("Python sys.path after:");
        let path = sys.getattr("path")?.extract::<Vec<String>>()?;
        for p in path.iter() {
            println!("  {}", p);
        }
        
        let module = py.import("hello")?;
        let result: String = module
            .getattr("mif_reader")?
            .call1((file_path, layer_index, variant_index, scale))?
            .extract()?;
        Ok(result)
    }) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string())
    }
}

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("tauri-test/cpp/hello.h");
        
        pub fn say_hello();
        pub fn add(a: f64, b: f64) -> f64;
        pub fn subtract(a: f64, b: f64) -> f64;
        pub fn multiply(a: f64, b: f64) -> f64;
        pub fn divide(a: f64, b: f64) -> f64;
    }
}

#[tauri::command]
fn call_cpp_hello() {
    ffi::say_hello();
}

#[tauri::command]
fn cpp_calculate(operation: &str, a: f64, b: f64) -> Result<f64, String> {
    match operation {
        "add" => Ok(ffi::add(a, b)),
        "subtract" => Ok(ffi::subtract(a, b)),
        "multiply" => Ok(ffi::multiply(a, b)),
        "divide" => {
            if b == 0.0 {
                Err("Division by zero!".to_string())
            } else {
                Ok(ffi::divide(a, b))
            }
        },
        _ => Err(format!("Unknown operation: {}", operation))
    }
}

#[tauri::command]
fn process_file() -> String {
    "success".to_string()
}

#[tauri::command]
fn show_alert() -> String {
    "Hello from Rust!".to_string()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            calculate,
            process_image,
            mif_reader,
            call_cpp_hello,
            process_file,
            show_alert,
            cpp_calculate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} 