//! Build script: embeds the application icon into Windows executables.
//!
//! Writes a compiled resource file (.res) by hand, so no resource compiler is
//! needed when cross-compiling from Linux. Does nothing on other targets.

use std::{env, fs, path::PathBuf};

const SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

fn entry(out: &mut Vec<u8>, ty: u16, id: u16, flags: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes()); // header size
    out.extend_from_slice(&[0xff, 0xff]);
    out.extend_from_slice(&ty.to_le_bytes());
    out.extend_from_slice(&[0xff, 0xff]);
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes()); // memory flags
    out.extend_from_slice(&0x0409u16.to_le_bytes()); // language
    out.extend_from_slice(&0u32.to_le_bytes()); // version
    out.extend_from_slice(&0u32.to_le_bytes()); // characteristics
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=packaging/icons");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut images = Vec::new();
    for size in SIZES {
        match fs::read(root.join(format!("packaging/icons/icon_{size}.png"))) {
            Ok(png) => images.push((size, png)),
            Err(e) => println!("cargo:warning=icono {size}px no disponible: {e}"),
        }
    }
    if images.is_empty() {
        return;
    }
    let mut res = Vec::new();
    entry(&mut res, 0, 0, 0, &[]); // mandatory empty first entry
    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    group.extend_from_slice(&(images.len() as u16).to_le_bytes());
    for (i, (size, png)) in images.iter().enumerate() {
        let id = i as u16 + 1;
        // PNG-compressed icon images (Windows Vista and later)
        entry(&mut res, RT_ICON, id, 0x1010, png);
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        group.extend_from_slice(&[dim, dim, 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes()); // planes
        group.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        group.extend_from_slice(&(png.len() as u32).to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    entry(&mut res, RT_GROUP_ICON, 1, 0x1030, &group);
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("finakids_icon.res");
    fs::write(&out, res).expect("write icon resource");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
