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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![calculate, process_image, mif_reader])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} 