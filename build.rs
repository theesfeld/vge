use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let is_x86_64 = target.starts_with("x86_64-");
    let force_c = env::var("VGE_FORCE_C").is_ok();
    let use_asm = is_x86_64 && !force_c;

    println!("cargo:rerun-if-changed=asm/x86_64/vge.s");
    println!("cargo:rerun-if-changed=c/vge_portable.c");
    println!("cargo:rerun-if-changed=include/vge.h");
    println!("cargo:rerun-if-env-changed=VGE_FORCE_C");

    let mut build = cc::Build::new();
    build.include(manifest.join("include"));
    build.file(manifest.join("c/vge_portable.c"));
    build.warnings(true);

    println!("cargo:rustc-check-cfg=cfg(vge_asm)");

    if use_asm {
        build.define("VGE_USE_ASM", None);
        let asm_src = manifest.join("asm/x86_64/vge.s");
        let asm_obj = out.join("vge_asm.o");
        let status = Command::new("as")
            .args(["--64", "-o"])
            .arg(&asm_obj)
            .arg(&asm_src)
            .status()
            .expect("failed to run GNU as (binutils)");
        if !status.success() {
            panic!("as failed assembling asm/x86_64/vge.s");
        }
        build.object(&asm_obj);
        println!("cargo:rustc-cfg=vge_asm");
        println!("cargo:warning=VGE: linking x86_64 assembly hot path");
    } else {
        println!("cargo:warning=VGE: portable C raster path");
    }

    build.compile("vge_c");
    println!("cargo:rustc-link-lib=m");
}
