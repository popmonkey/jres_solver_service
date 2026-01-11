fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/shim.cc")
        .include("vendor/jres_solver/include")
        .include("vendor/highs/include")
        .include(".")
        .flag_if_supported("-std=c++14")
        .compile("jres_solver_bridge");

    println!("cargo:rustc-link-search=native=vendor/jres_solver/lib");
    println!("cargo:rustc-link-lib=static=jres_solver");
    
    // Link against Highs (dependency of jres_solver)
    println!("cargo:rustc-link-search=native=vendor/highs/lib");
    println!("cargo:rustc-link-lib=static=highs");
    // Highs might have dependencies, usually it's just standard C++ libs which Rust handles,
    // but sometimes -lz or -lm are needed. Assuming standard for now.

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/shim.cc");
    println!("cargo:rerun-if-changed=src/shim.h");
    println!("cargo:rerun-if-changed=vendor/jres_solver/include/jres_solver/jres_solver.hpp");
    println!("cargo:rerun-if-changed=vendor/jres_solver/lib/libjres_solver.a");
}
