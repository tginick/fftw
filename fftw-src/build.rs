#![allow(dead_code)]

use anyhow::Result;
use std::env::var;
use std::fs::{canonicalize, File};
use std::io::{copy, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

fn download_archive_windows(out_dir: &Path) -> Result<()> {
    if out_dir.join("libfftw3.dll").exists() && out_dir.join("libfftw3f.dll").exists() {
        return Ok(());
    }

    let archive = out_dir.join("fftw_windows.zip");
    if !archive.exists() {
        // Download
        let mut conn = ftp::FtpStream::connect("ftp.fftw.org:21")?;
        conn.login("anonymous", "anonymous")?;
        conn.cwd("pub/fftw")?;
        let buf = conn.simple_retr("fftw-3.3.5-dll64.zip")?.into_inner();
        // TODO calc checksum
        let mut f = File::create(&archive)?;
        f.write(&buf)?;
    }
    let f = File::open(&archive)?;
    let mut zip = ZipArchive::new(f)?;
    let target = var("TARGET").unwrap();
    for name in &["fftw3-3", "fftw3f-3"] {
        for ext in &["dll", "def"] {
            let filename = format!("lib{}.{}", name, ext);
            let mut zf = zip.by_name(&filename)?;
            let mut f = File::create(out_dir.join(filename))?;
            copy(&mut zf, &mut f)?;
        }
        run(cc::windows_registry::find_tool(&target, "lib.exe")
            .unwrap()
            .to_command()
            .arg("/MACHINE:X64")
            .arg(format!("/DEF:lib{}.def", name))
            .arg(format!("/OUT:lib{}.lib", name))
            .current_dir(out_dir))
    }
    Ok(())
}

fn build_unix(out_dir: &Path) {
    let src_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap()).join("fftw-3.3.8");
    let out_src_dir = out_dir.join("src");
    fs_extra::dir::copy(
        src_dir,
        &out_src_dir,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            buffer_size: 64000,
            copy_inside: true,
            content_only: true,
            depth: 0,
        },
    )
    .unwrap();
    if !out_dir.join("lib/libfftw3.a").exists() {
        build_fftw(&[], &out_src_dir, &out_dir);
    }
    if !out_dir.join("lib/libfftw3f.a").exists() {
        build_fftw(&["--enable-single"], &out_src_dir, &out_dir);
    }
}

fn build_fftw(flags: &[&str], src_dir: &Path, out_dir: &Path) {
    run(
        Command::new(canonicalize(src_dir.join("configure")).unwrap())
            .arg("--with-pic")
            .arg("--enable-static")
            .arg("--disable-doc")
            .arg(format!("--prefix={}", out_dir.display()))
            .args(flags)
            .current_dir(&src_dir),
    );
    run(Command::new("make")
        .arg(format!("-j{}", var("NUM_JOBS").unwrap()))
        .current_dir(&src_dir));
    run(Command::new("make").arg("install").current_dir(&src_dir));
}

fn run(command: &mut Command) {
    println!("Running: {:?}", command);
    match command.status() {
        Ok(status) => {
            if !status.success() {
                panic!("`{:?}` failed: {}", command, status);
            }
        }
        Err(error) => {
            panic!("failed to execute `{:?}`: {}", command, error);
        }
    }
}

fn main() {
    let target_arch = var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = var("CARGO_CFG_TARGET_OS").unwrap();
    let mut precompiled_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap());
    precompiled_dir.push("precompiled");

    if target_os == "windows" {
        //download_archive_windows(&out_dir).unwrap();
        precompiled_dir.push("windows");
        println!("cargo:rustc-link-search={}", precompiled_dir.display());
        println!("cargo:rustc-link-lib=static=fftw3");
        println!("cargo:rustc-link-lib=static=fftw3f");
    } else {
        //build_unix(&out_dir);
        precompiled_dir.push("linux");
        if target_arch == "arm" {
            precompiled_dir.push("armv7");

            println!("cargo:rustc-link-search={}", precompiled_dir.display());
            println!("cargo:rustc-link-lib=static=fftw3");
            println!("cargo:rustc-link-lib=static=fftw3f");
        } else if target_arch == "x86_64" {
            precompiled_dir.push("x64");

            println!("cargo:rustc-link-search={}", precompiled_dir.display());
            println!("cargo:rustc-link-lib=static=fftw3");
            println!("cargo:rustc-link-lib=static=fftw3f");
        } else if target_arch == "aarch64" {
            precompiled_dir.push("aarch64");

            // using elk's sdk, these are provided as so files
            println!("cargo:rustc-link-lib=fftw3");
            println!("cargo:rustc-link-lib=fftw3f");
        }
    }
}
