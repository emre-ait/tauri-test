fn main() {
    tauri_build::build();
    
    cxx_build::bridge("src/lib.rs")
        .file("cpp/hello.cpp")
        .file("cpp/math_lib.cpp")
        .flag_if_supported("-std=c++14")
        .compile("hello-world");
        
    println!("cargo:rerun-if-changed=cpp/hello.cpp");
    println!("cargo:rerun-if-changed=cpp/hello.h");
    println!("cargo:rerun-if-changed=cpp/math_lib.cpp");
    println!("cargo:rerun-if-changed=cpp/math_lib.h");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
