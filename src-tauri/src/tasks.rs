use std::path::Path;

use windows::{
    core::{BSTR, Interface},
    Win32::Foundation::{RPC_E_CHANGED_MODE, VARIANT_BOOL},
    Win32::System::{
        Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        TaskScheduler::{
            IExecAction, IRegisteredTask, ITaskDefinition, ITaskFolder, ITaskService, ITimeTrigger,
            TaskScheduler, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN,
            TASK_RUNLEVEL_HIGHEST, TASK_TRIGGER_TIME,
        },
        Variant::VARIANT,
    },
};

fn fmt_win(context: &str, e: windows::core::Error) -> String {
    format!("{context} (0x{:08X}): {}", e.code().0 as u32, e.message())
}

fn with_com<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    // Tauri may already initialize COM on the current thread (often as STA).
    // If the apartment model differs, CoInitializeEx returns RPC_E_CHANGED_MODE.
    // In that case, keep going and avoid calling CoUninitialize().
    let mut should_uninit = false;
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr == RPC_E_CHANGED_MODE {
            // COM already initialized with a different apartment model on this thread.
        } else {
            hr.ok().map_err(|e| fmt_win("CoInitializeEx failed", e))?;
            should_uninit = true;
        }
    }
    let result = f();
    if should_uninit {
        unsafe { CoUninitialize() };
    }
    result
}

fn root_folder(service: &ITaskService) -> Result<ITaskFolder, String> {
    let folder = unsafe { service.GetFolder(&BSTR::from("\\")) }
        .map_err(|e| fmt_win("ITaskService::GetFolder failed", e))?;
    Ok(folder)
}

fn connect_service(service: &ITaskService) -> Result<(), String> {
    let empty = VARIANT::default();
    unsafe {
        service
            .Connect(&empty, &empty, &empty, &empty)
            .map_err(|e| fmt_win("ITaskService::Connect failed", e))
    }
}

fn ensure_time_trigger(task_def: &ITaskDefinition) -> Result<(), String> {
    let triggers = unsafe { task_def.Triggers() }
        .map_err(|e| fmt_win("ITaskDefinition::Triggers failed", e))?;
    let trigger = unsafe { triggers.Create(TASK_TRIGGER_TIME) }
        .map_err(|e| fmt_win("ITriggerCollection::Create failed", e))?;
    // Some trigger types require a StartBoundary; use a far-future value and disable the trigger.
    let time: ITimeTrigger = trigger
        .cast()
        .map_err(|e| fmt_win("cast ITrigger -> ITimeTrigger failed", e))?;
    unsafe { time.SetStartBoundary(&BSTR::from("2099-01-01T00:00:00")) }
        .map_err(|e| fmt_win("ITimeTrigger::SetStartBoundary failed", e))?;
    // Disable it so it won't auto-run; task is started manually via Run().
    let enabled: VARIANT_BOOL = false.into();
    unsafe { trigger.SetEnabled(enabled) }
        .map_err(|e| fmt_win("ITrigger::SetEnabled failed", e))?;
    Ok(())
}

fn set_principal(task_def: &ITaskDefinition) -> Result<(), String> {
    let principal = unsafe { task_def.Principal() }
        .map_err(|e| fmt_win("ITaskDefinition::Principal failed", e))?;
    unsafe { principal.SetRunLevel(TASK_RUNLEVEL_HIGHEST) }
        .map_err(|e| fmt_win("IPrincipal::SetRunLevel failed", e))?;
    unsafe { principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN) }
        .map_err(|e| fmt_win("IPrincipal::SetLogonType failed", e))?;
    Ok(())
}

fn set_settings(task_def: &ITaskDefinition) -> Result<(), String> {
    let settings = unsafe { task_def.Settings() }
        .map_err(|e| fmt_win("ITaskDefinition::Settings failed", e))?;
    unsafe { settings.SetAllowDemandStart(true.into()) }
        .map_err(|e| fmt_win("ITaskSettings::SetAllowDemandStart failed", e))?;
    unsafe { settings.SetStartWhenAvailable(true.into()) }
        .map_err(|e| fmt_win("ITaskSettings::SetStartWhenAvailable failed", e))?;
    // No execution time limit
    unsafe { settings.SetExecutionTimeLimit(&BSTR::from("PT0S")) }
        .map_err(|e| fmt_win("ITaskSettings::SetExecutionTimeLimit failed", e))?;
    Ok(())
}

fn set_action_exec(task_def: &ITaskDefinition, exe_path: &str, arguments: Option<&str>) -> Result<(), String> {
    let actions = unsafe { task_def.Actions() }
        .map_err(|e| fmt_win("ITaskDefinition::Actions failed", e))?;
    let action = unsafe { actions.Create(TASK_ACTION_EXEC) }
        .map_err(|e| fmt_win("IActionCollection::Create failed", e))?;
    let exec: IExecAction = action
        .cast()
        .map_err(|e| fmt_win("cast IAction -> IExecAction failed", e))?;

    unsafe { exec.SetPath(&BSTR::from(exe_path)) }
        .map_err(|e| fmt_win("IExecAction::SetPath failed", e))?;
    if let Some(args) = arguments {
        let trimmed = args.trim();
        if !trimmed.is_empty() {
            unsafe { exec.SetArguments(&BSTR::from(trimmed)) }
                .map_err(|e| fmt_win("IExecAction::SetArguments failed", e))?;
        }
    }
    if let Some(dir) = Path::new(exe_path).parent().and_then(|p| p.to_str()) {
        unsafe { exec.SetWorkingDirectory(&BSTR::from(dir)) }
            .map_err(|e| fmt_win("IExecAction::SetWorkingDirectory failed", e))?;
    }
    Ok(())
}

pub fn create_task(task_name: &str, exe_path: &str, arguments: Option<&str>) -> Result<(), String> {
    with_com(|| {
        let service: ITaskService = unsafe {
            CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| fmt_win("CoCreateInstance(TaskScheduler) failed", e))?
        };
        connect_service(&service)?;
        let folder = root_folder(&service)?;

        let task_def: ITaskDefinition = unsafe { service.NewTask(0) }
            .map_err(|e| fmt_win("ITaskService::NewTask failed", e))?;

        set_principal(&task_def)?;
        set_settings(&task_def)?;
        ensure_time_trigger(&task_def)?;
        set_action_exec(&task_def, exe_path, arguments)?;

        let _task: IRegisteredTask = unsafe {
            let empty = VARIANT::default();
            folder.RegisterTaskDefinition(
                &BSTR::from(task_name),
                &task_def,
                TASK_CREATE_OR_UPDATE.0,
                &empty,
                &empty,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty,
            )
        }
        .map_err(|e| fmt_win("RegisterTaskDefinition failed", e))?;

        Ok(())
    })
}

pub fn run_task(task_name: &str) -> Result<(), String> {
    with_com(|| {
        let service: ITaskService = unsafe {
            CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| fmt_win("CoCreateInstance(TaskScheduler) failed", e))?
        };
        connect_service(&service)?;
        let folder = root_folder(&service)?;
        let task: IRegisteredTask = unsafe { folder.GetTask(&BSTR::from(task_name)) }
            .map_err(|e| fmt_win("GetTask failed", e))?;
        let empty = VARIANT::default();
        unsafe { task.Run(&empty) }.map_err(|e| fmt_win("Run failed", e))?;
        Ok(())
    })
}

pub fn delete_task(task_name: &str) -> Result<(), String> {
    with_com(|| {
        let service: ITaskService = unsafe {
            CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| fmt_win("CoCreateInstance(TaskScheduler) failed", e))?
        };
        connect_service(&service)?;
        let folder = root_folder(&service)?;
        unsafe { folder.DeleteTask(&BSTR::from(task_name), 0) }
            .map_err(|e| fmt_win("DeleteTask failed", e))?;
        Ok(())
    })
}
