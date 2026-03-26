use std::{
    fs::File,
    mem::{size_of, zeroed},
    path::Path,
};

use png::{BitDepth, ColorType, Encoder};
use windows::{
    core::PCWSTR,
    Win32::{
        Graphics::Gdi::{
            DeleteObject, GetDIBits, GetDC, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
            WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO},
        },
    },
};

use crate::util::to_wide;

pub fn extract_icon_png_for_path(icon_source: &Path, out_png: &Path) -> Result<(), String> {
    let wide = to_wide(icon_source);

    let mut info: SHFILEINFOW = unsafe { zeroed() };
    let flags = SHGFI_ICON | SHGFI_LARGEICON;
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };

    if ok == 0 || info.hIcon.0.is_null() {
        return Err("extract icon failed (SHGetFileInfoW returned no icon)".to_string());
    }

    let hicon = info.hIcon;
    let result = hicon_to_png(hicon, out_png);
    unsafe { let _ = DestroyIcon(hicon); }
    result
}

fn hicon_to_png(hicon: HICON, out_png: &Path) -> Result<(), String> {
    let mut icon_info: ICONINFO = unsafe { zeroed() };
    if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_err() {
        return Err("GetIconInfo failed".to_string());
    }

    let hbm_color = icon_info.hbmColor;
    let hbm_mask = icon_info.hbmMask;

    if hbm_color.0.is_null() {
        unsafe {
            if !hbm_mask.0.is_null() {
                let _ = DeleteObject(hbm_mask.into());
            }
        }
        return Err("icon has no color bitmap".to_string());
    }

    let mut bmp: BITMAP = unsafe { zeroed() };
    let got = unsafe {
        GetObjectW(
            hbm_color.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as _),
        )
    };
    if got == 0 {
        unsafe {
            let _ = DeleteObject(hbm_color.into());
            if !hbm_mask.0.is_null() {
                let _ = DeleteObject(hbm_mask.into());
            }
        }
        return Err("GetObjectW failed".to_string());
    }

    let width = bmp.bmWidth as i32;
    let height = bmp.bmHeight as i32;
    if width <= 0 || height <= 0 {
        unsafe {
            let _ = DeleteObject(hbm_color.into());
            if !hbm_mask.0.is_null() {
                let _ = DeleteObject(hbm_mask.into());
            }
        }
        return Err("invalid icon bitmap size".to_string());
    }

    let mut bmi: BITMAPINFO = unsafe { zeroed() };
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height, // top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0 as u32,
        ..Default::default()
    };

    let mut bgra = vec![0u8; (width * height * 4) as usize];
    let hdc = unsafe { GetDC(None) };
    if hdc.0.is_null() {
        unsafe {
            let _ = DeleteObject(hbm_color.into());
            if !hbm_mask.0.is_null() {
                let _ = DeleteObject(hbm_mask.into());
            }
        }
        return Err("GetDC failed".to_string());
    }

    let scanlines = unsafe {
        GetDIBits(
            hdc,
            hbm_color,
            0,
            height as u32,
            Some(bgra.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    unsafe { ReleaseDC(None, hdc) };

    unsafe {
        let _ = DeleteObject(hbm_color.into());
        if !hbm_mask.0.is_null() {
            let _ = DeleteObject(hbm_mask.into());
        }
    }

    if scanlines == 0 {
        return Err("GetDIBits failed".to_string());
    }

    let mut rgba = bgra;
    for p in rgba.chunks_exact_mut(4) {
        let b = p[0];
        let r = p[2];
        p[0] = r;
        p[2] = b;
    }

    let file = File::create(out_png)
        .map_err(|e| format!("create png failed ({}): {e}", out_png.display()))?;
    let mut encoder = Encoder::new(file, width as u32, height as u32);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header failed: {e}"))?;
    writer
        .write_image_data(&rgba)
        .map_err(|e| format!("png write failed: {e}"))?;
    Ok(())
}
