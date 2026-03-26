use std::path::{Path, PathBuf};

use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, IPersistFile, STGM_READ,
        },
        UI::Shell::{IShellLinkW, ShellLink, SLR_NO_UI},
    },
};

use crate::util::{to_wide, wide_buf_to_string};

#[derive(Debug, Clone)]
pub struct ResolvedShortcut {
    pub target_path: PathBuf,
    pub arguments: Option<String>,
    pub icon_path: Option<PathBuf>,
}

pub fn resolve_lnk(lnk_path: &Path) -> Result<ResolvedShortcut, String> {
    let wide_path = to_wide(lnk_path);

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| format!("CoInitializeEx failed: {e}"))?;
    }

    let resolved = (|| -> Result<ResolvedShortcut, String> {
        let shell_link: IShellLinkW = unsafe {
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance(ShellLink) failed: {e}"))?
        };

        let persist: IPersistFile = shell_link
            .cast()
            .map_err(|e| format!("cast to IPersistFile failed: {e}"))?;

        unsafe {
            persist
                .Load(PCWSTR(wide_path.as_ptr()), STGM_READ)
                .map_err(|e| format!("IPersistFile::Load failed: {e}"))?;
            let _ = shell_link.Resolve(HWND(std::ptr::null_mut()), SLR_NO_UI.0 as u32);
        }

        let mut target_buf = [0u16; 32768];
        unsafe { shell_link.GetPath(&mut target_buf, std::ptr::null_mut(), 0) }
            .map_err(|e| format!("IShellLinkW::GetPath failed: {e}"))?;
        let target_str = wide_buf_to_string(&target_buf);
        let target_path = PathBuf::from(target_str);

        let mut args_buf = [0u16; 32768];
        unsafe { shell_link.GetArguments(&mut args_buf) }
            .map_err(|e| format!("IShellLinkW::GetArguments failed: {e}"))?;
        let args = wide_buf_to_string(&args_buf);
        let arguments = if args.trim().is_empty() {
            None
        } else {
            Some(args)
        };

        let mut icon_buf = [0u16; 32768];
        let mut icon_index: i32 = 0;
        unsafe { shell_link.GetIconLocation(&mut icon_buf, &mut icon_index) }
            .map_err(|e| format!("IShellLinkW::GetIconLocation failed: {e}"))?;
        let icon_str = wide_buf_to_string(&icon_buf);
        let icon_path = if icon_str.trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(icon_str))
        };

        Ok(ResolvedShortcut {
            target_path,
            arguments,
            icon_path,
        })
    })();

    unsafe { CoUninitialize() };
    resolved
}
