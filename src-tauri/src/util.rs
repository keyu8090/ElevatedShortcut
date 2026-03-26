use std::os::windows::ffi::OsStrExt;

pub fn to_wide(path: &std::path::Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    if wide.is_empty() || *wide.last().unwrap() != 0 {
        wide.push(0);
    }
    wide
}

pub fn to_wide_str(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}

pub fn wide_buf_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
