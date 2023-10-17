use cc;

fn main() {
    cc::Build::new()
        .flag("-Wno-unused-parameter") // Suppress unused parameter warnings
        .flag("-Wno-tautological-pointer-compare") // Suppress always false comparison warning
        .flag("-Wno-unused-function") // Suppress unused function warnings
        .file("manymouse/linux_evdev.c")
        .file("manymouse/macosx_hidmanager.c")
        .file("manymouse/macosx_hidutilities.c")
        .file("manymouse/manymouse.c")
        .file("manymouse/windows_wminput.c")
        .file("manymouse/x11_xinput2.c")
        .compile("libmanymouse.a");

    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=IOKit");

    let bindings = bindgen::Builder::default()
        .header("manymouse/manymouse.h")
        .generate()
        .expect("Failed to generate bindings");

    bindings
        .write_to_file("src/mischief/bindings.rs")
        .expect("Failed to write bindings");
}
