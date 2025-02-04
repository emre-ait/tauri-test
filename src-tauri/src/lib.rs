use pyo3::prelude::*;
use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
async fn process_image(app_handle: AppHandle, image_data: String) -> Result<String, String> {
	println!("Rust: Starting Python image processing");
    
    // Get the resource directory path
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("python_scripts");

    println!("Python scripts path: {:?}", resource_path);

    match Python::with_gil(|py| -> PyResult<String> {
        let sys = py.import("sys")?;
        sys.getattr("path")?.call_method1("append", (resource_path.to_str().unwrap(),))?;
        
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
async fn calculate(app_handle: AppHandle, operation: String, a: f64, b: f64) -> Result<String, String> {
	println!("Rust: Starting Python calculator with {} {} {}", operation, a, b);
    
    // Get the resource directory path
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("python_scripts");

    println!("Python scripts path: {:?}", resource_path);

    match Python::with_gil(|py| -> PyResult<String> {
        let sys = py.import("sys")?;
        sys.getattr("path")?.call_method1("append", (resource_path.to_str().unwrap(),))?;
        
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![calculate, process_image])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} 