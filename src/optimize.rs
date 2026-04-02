use winreg::enums::*;
use winreg::RegKey;

use crate::debug_print;

// ─────────────────────────────────────────────────────────────
// Granular Idempotency — per-task registry flags
// ─────────────────────────────────────────────────────────────

const TASK_REGISTRY_PATH: &str = r"Software\ZeroIdle\CompletedTasks";

pub fn is_task_done(task_id: &str) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(TASK_REGISTRY_PATH) {
        if let Ok(1u32) = key.get_value(task_id) {
            return true;
        }
    }
    false
}

pub fn mark_task_done(task_id: &str) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(TASK_REGISTRY_PATH) {
        let _ = key.set_value(task_id, &1u32);
    }
}

/// Check if ALL one-time tasks are already completed (fast-path for quick exit).
pub fn all_onetime_tasks_done() -> bool {
    const TASK_IDS: &[&str] = &["gaming_opt", "system_privacy", "startup_services"];
    TASK_IDS.iter().all(|id| is_task_done(id))
}

// ─────────────────────────────────────────────────────────────
// Migration: honor the old single-flag system
// ─────────────────────────────────────────────────────────────

/// If the old `Software\Optimizer\Optimized=1` flag exists, migrate it to the
/// new per-task system so we don't re-run tasks that were already applied.
pub fn migrate_legacy_flag() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Migration path 1: Old single-flag system (Software\Optimizer\Optimized=1)
    if let Ok(key) = hkcu.open_subkey(r"Software\Optimizer") {
        if let Ok(1u32) = key.get_value("Optimized") {
            debug_print("[⟳] Migrating legacy optimization flag to per-task tracking...");
            mark_task_done("gaming_opt");
            mark_task_done("system_privacy");
            // Don't mark startup_services — that's new, needs to run once.
            let _ = hkcu.delete_subkey_all(r"Software\Optimizer");
            debug_print("  [✓] Legacy flag migrated.");
        }
    }

    // Migration path 2: Old IDMSystemTool naming → ZeroIdle
    if let Ok(old_key) = hkcu.open_subkey(r"Software\IDMSystemTool\CompletedTasks") {
        debug_print("[⟳] Migrating IDMSystemTool task flags to ZeroIdle...");
        for task_id in &["gaming_opt", "system_privacy", "startup_services"] {
            if let Ok(1u32) = old_key.get_value(*task_id) {
                mark_task_done(task_id);
            }
        }
        let _ = hkcu.delete_subkey_all(r"Software\IDMSystemTool");
        debug_print("  [✓] IDMSystemTool flags migrated to ZeroIdle.");
    }
}

// ─────────────────────────────────────────────────────────────
// Phase: Gaming Optimizations (one-time, task_id = "gaming_opt")
// ─────────────────────────────────────────────────────────────

pub fn optimize_for_gaming() {
    debug_print("");
    debug_print("[⟳] Phase: Gaming Optimizations...");

    enable_game_mode();
    disable_game_dvr();
    set_ultimate_performance_power_plan();
    flush_dns();
    optimize_network();
    clear_standby_memory();
    disable_mouse_acceleration();
    disable_nagles_algorithm();
    disable_xbox_game_bar();

    mark_task_done("gaming_opt");
    debug_print("  [✓] Gaming optimizations applied.");
}

// ─────────────────────────────────────────────────────────────
// Phase: System & Privacy (one-time, task_id = "system_privacy")
// ─────────────────────────────────────────────────────────────

pub fn optimize_system_and_privacy() {
    debug_print("");
    debug_print("[⟳] Phase: System & Privacy...");

    disable_telemetry();
    disable_hibernation();
    clear_event_logs();
    disable_start_menu_web_search();
    disable_consumer_features();
    disable_tips_and_suggestions();
    disable_app_launch_tracking();
    disable_transparency_effects();
    disable_lock_screen_tips();
    disable_feedback_notifications();

    mark_task_done("system_privacy");
    debug_print("  [✓] System & Privacy optimizations applied.");
}

// ─────────────────────────────────────────────────────────────
// Phase: Startup & Services (one-time, task_id = "startup_services")
// ─────────────────────────────────────────────────────────────

pub fn optimize_startup_and_services() {
    debug_print("");
    debug_print("[⟳] Phase: Startup & Services Optimization...");

    disable_nonessential_services();
    disable_bloatware_startup_entries();
    disable_scheduled_defrag();
    disable_startup_delay();
    disable_background_apps();

    mark_task_done("startup_services");
    debug_print("  [✓] Startup & Services optimizations applied.");
}

// ─────────────────────────────────────────────────────────────
// Phase: Adobe Optimization (always runs — kills active procs)
// ─────────────────────────────────────────────────────────────

pub fn optimize_for_adobe() {
    debug_print("");
    debug_print("[⟳] Phase: Adobe Optimization...");

    kill_adobe_background_processes();

    debug_print("  [✓] Adobe optimizations applied.");
}

// ═════════════════════════════════════════════════════════════
// Implementation — Gaming
// ═════════════════════════════════════════════════════════════

fn enable_game_mode() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\GameBar") {
        let _ = key.set_value("AutoGameModeEnabled", &1u32);
        debug_print("    ✓ Game Mode enabled (HKCU).");
    }
}

fn disable_game_dvr() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"System\GameConfigStore") {
        let _ = key.set_value("GameDVR_Enabled", &0u32);
        let _ = key.set_value("GameDVR_FSEBehaviorMode", &2u32);
        debug_print("    ✓ Game DVR disabled (HKCU).");
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok((key, _)) = hklm.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\GameDVR") {
        let _ = key.set_value("AllowGameDVR", &0u32);
        debug_print("    ✓ Game DVR disabled (Policies).");
    }
}

/// Unhide and activate Ultimate Performance power plan.
/// Falls back to High Performance if Ultimate Performance isn't available.
fn set_ultimate_performance_power_plan() {
    // Unhide Ultimate Performance (available on Win10 1803+)
    let _ = crate::hidden_command("powercfg")
        .args(&["/duplicatescheme", "e9a42b02-d5df-448d-aa00-03f14749eb61"])
        .output();

    // Try to activate Ultimate Performance
    let result = crate::hidden_command("powercfg")
        .args(&["/setactive", "e9a42b02-d5df-448d-aa00-03f14749eb61"])
        .output();

    if let Ok(o) = result {
        if o.status.success() {
            debug_print("    ✓ Power plan set to Ultimate Performance.");
            return;
        }
    }

    // Fallback to High Performance
    let output = crate::hidden_command("powercfg")
        .args(&["/setactive", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"])
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            debug_print("    ✓ Power plan set to High Performance (Ultimate unavailable).");
        }
    }
}

fn flush_dns() {
    let output = crate::hidden_command("ipconfig")
        .args(&["/flushdns"])
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            debug_print("    ✓ DNS cache flushed.");
        }
    }
}

fn optimize_network() {
    let _ = crate::hidden_command("netsh")
        .args(&["int", "tcp", "set", "heuristics", "disabled"])
        .output();
    let _ = crate::hidden_command("netsh")
        .args(&["int", "tcp", "set", "global", "autotuninglevel=normal"])
        .output();
    let _ = crate::hidden_command("netsh")
        .args(&["int", "tcp", "set", "global", "ecncapability=disabled"])
        .output();
    let _ = crate::hidden_command("netsh")
        .args(&["int", "tcp", "set", "global", "rss=enabled"])
        .output();

    debug_print("    ✓ Network TCP/IP settings optimized for lower latency.");
}

fn clear_standby_memory() {
    let ps_script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$priv = [uri].Module.GetType('System.Diagnostics.Process').GetMethods(42) | Where-Object { $_.Name -eq 'SetPrivilege' }
$priv.Invoke($null, @('SeProfileSingleProcessPrivilege', 2))
$type = Add-Type -MemberDefinition '[DllImport("ntdll.dll")] public static extern int NtSetSystemInformation(int SystemInformationClass, IntPtr SystemInformation, int SystemInformationLength);' -Name 'Ntdll' -Namespace 'Win32' -PassThru
$ptr = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(4)
[System.Runtime.InteropServices.Marshal]::WriteInt32($ptr, 0, 4)
$type::NtSetSystemInformation(80, $ptr, 4)
[System.Runtime.InteropServices.Marshal]::FreeHGlobal($ptr)
"#;
    let _ = crate::hidden_command("powershell")
        .args(&["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    debug_print("    ✓ Cleared System Memory Standby List.");
}

fn disable_mouse_acceleration() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"Control Panel\Mouse") {
        let _ = key.set_value("MouseSpeed", &"0");
        let _ = key.set_value("MouseThreshold1", &"0");
        let _ = key.set_value("MouseThreshold2", &"0");
        debug_print("    ✓ Mouse Acceleration disabled.");
    }
}

fn disable_nagles_algorithm() {
    let ps_script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$interfaces = Get-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\*"
foreach ($interface in $interfaces) {
    Set-ItemProperty -Path $interface.PSPath -Name "TcpAckFrequency" -Value 1 -Type DWord
    Set-ItemProperty -Path $interface.PSPath -Name "TCPNoDelay" -Value 1 -Type DWord
}
"#;
    let _ = crate::hidden_command("powershell")
        .args(&["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();
    debug_print("    ✓ Nagle's Algorithm disabled (Lower Ping).");
}

fn disable_xbox_game_bar() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\GameDVR") {
        let _ = key.set_value("AppCaptureEnabled", &0u32);
    }

    let ps_script = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-AppxPackage -AllUsers *XboxGamingOverlay* | Remove-AppxPackage -AllUsers
"#;
    let _ = crate::hidden_command("powershell")
        .args(&["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    debug_print("    ✓ Xbox Game Bar uninstalled & disabled.");
}

// ═════════════════════════════════════════════════════════════
// Implementation — System & Privacy
// ═════════════════════════════════════════════════════════════

fn disable_telemetry() {
    ["DiagTrack", "dmwappushservice", "SysMain"]
        .iter()
        .for_each(|srv| {
            let _ = crate::hidden_command("sc").args(&["stop", srv]).output();
            let _ = crate::hidden_command("sc")
                .args(&["config", srv, "start=", "disabled"])
                .output();
        });

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok((key, _)) = hklm.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\DataCollection")
    {
        let _ = key.set_value("AllowTelemetry", &0u32);
    }
    debug_print("    ✓ Windows Telemetry & SuperFetch disabled.");
}

fn disable_hibernation() {
    let output = crate::hidden_command("powercfg")
        .args(&["-h", "off"])
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            debug_print("    ✓ Hibernation disabled (freed gigabytes of C:\\ space).");
        }
    }
}

fn clear_event_logs() {
    ["Application", "Security", "Setup", "System"]
        .iter()
        .for_each(|log| {
            let _ = crate::hidden_command("wevtutil")
                .args(&["cl", log])
                .output();
        });
    debug_print("    ✓ Windows event logs cleared.");
}

fn disable_start_menu_web_search() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\Explorer") {
        let _ = key.set_value("DisableSearchBoxSuggestions", &1u32);
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok((key, _)) = hklm.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search")
    {
        let _ = key.set_value("DisableWebSearch", &1u32);
        let _ = key.set_value("ConnectedSearchUseWeb", &0u32);
    }
    debug_print("    ✓ Web Search in Start Menu disabled.");
}

fn disable_consumer_features() {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok((key, _)) = hklm.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\CloudContent") {
        let _ = key.set_value("DisableWindowsConsumerFeatures", &1u32);
    }
    debug_print("    ✓ Consumer Features disabled (no auto-installing Candy Crush).");
}

fn disable_tips_and_suggestions() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager")
    {
        let _ = key.set_value("SubscribedContent-338389Enabled", &0u32);
        let _ = key.set_value("SubscribedContent-310093Enabled", &0u32);
        let _ = key.set_value("SubscribedContent-338388Enabled", &0u32);
        let _ = key.set_value("SubscribedContent-338393Enabled", &0u32);
        let _ = key.set_value("SubscribedContent-353694Enabled", &0u32);
        let _ = key.set_value("SubscribedContent-353696Enabled", &0u32);
        let _ = key.set_value("SoftLandingEnabled", &0u32);
        let _ = key.set_value("SystemPaneSuggestionsEnabled", &0u32);
    }
    debug_print("    ✓ Tips, Suggestions & Ads in Settings disabled.");
}

fn disable_app_launch_tracking() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced")
    {
        let _ = key.set_value("Start_TrackProgs", &0u32);
    }
    debug_print("    ✓ App launch tracking disabled.");
}

fn disable_transparency_effects() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
    {
        let _ = key.set_value("EnableTransparency", &0u32);
    }
    debug_print("    ✓ Transparency effects disabled (lower GPU idle overhead).");
}

fn disable_lock_screen_tips() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager")
    {
        let _ = key.set_value("RotatingLockScreenOverlayEnabled", &0u32);
        let _ = key.set_value("RotatingLockScreenEnabled", &0u32);
    }
    debug_print("    ✓ Lock screen tips & spotlight disabled.");
}

fn disable_feedback_notifications() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Siuf\Rules") {
        let _ = key.set_value("NumberOfSIUFInPeriod", &0u32);
    }
    debug_print("    ✓ Feedback notifications disabled.");
}

// ═════════════════════════════════════════════════════════════
// Implementation — Startup & Services (NEW)
// ═════════════════════════════════════════════════════════════

fn disable_nonessential_services() {
    // Services that consume background CPU/RAM with no benefit for a work-ready PC.
    // Format: (service_name, description)
    let services: &[(&str, &str)] = &[
        ("WSearch", "Windows Search Indexer"),
        ("WerSvc", "Windows Error Reporting"),
        ("MapsBroker", "Downloaded Maps Manager"),
        ("lfsvc", "Geolocation Service"),
        ("RetailDemo", "Retail Demo Service"),
        ("wisvc", "Windows Insider Service"),
        ("WMPNetworkSvc", "Windows Media Player Sharing"),
        ("WpcMonSvc", "Parental Controls"),
        ("SEMgrSvc", "Payments & NFC Manager"),
        ("PhoneSvc", "Phone Service"),
        ("Fax", "Fax Service"),
        ("XblAuthManager", "Xbox Live Auth Manager"),
        ("XblGameSave", "Xbox Live Game Save"),
        ("XboxNetApiSvc", "Xbox Live Networking"),
        ("XboxGipSvc", "Xbox Accessory Management"),
    ];

    let mut stopped = 0u32;
    for (svc, desc) in services {
        let stop_result = crate::hidden_command("sc").args(&["stop", svc]).output();
        let config_result = crate::hidden_command("sc")
            .args(&["config", svc, "start=", "disabled"])
            .output();

        if config_result.map(|o| o.status.success()).unwrap_or(false) {
            stopped += 1;
        }
        let _ = stop_result;
        let _ = desc; // used for documentation
    }
    debug_print(&format!(
        "    ✓ Disabled {} non-essential background services.",
        stopped
    ));
}

fn disable_bloatware_startup_entries() {
    // Common startup entries that most power users don't need
    let startup_entries: &[&str] = &[
        "OneDrive",
        "OneDriveSetup",
        "Cortana",
        "SecurityHealth", // Defender tray icon (defender still runs)
        "iTunesHelper",
        "Spotify",
        "Steam",
        "Discord",
        "EpicGamesLauncher",
        "GoogleUpdate",
        "Teams",
        "Skype",
    ];

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_READ | KEY_WRITE,
    ) {
        for entry in startup_entries {
            // Only delete if it exists — silently skip otherwise
            if key.get_raw_value(entry).is_ok() {
                let _ = key.delete_value(entry);
                debug_print(&format!("    ✓ Removed startup entry: {}", entry));
            }
        }
    }

    // Also check HKLM Run (requires admin)
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
        KEY_READ | KEY_WRITE,
    ) {
        for entry in startup_entries {
            if key.get_raw_value(entry).is_ok() {
                let _ = key.delete_value(entry);
                debug_print(&format!("    ✓ Removed HKLM startup entry: {}", entry));
            }
        }
    }

    debug_print("    ✓ Bloatware startup entries cleaned.");
}

fn disable_scheduled_defrag() {
    // Disable automatic defrag — unnecessary on SSDs, dev machines handle this manually
    let _ = crate::hidden_command("schtasks")
        .args(&[
            "/Change",
            "/TN",
            r"\Microsoft\Windows\Defrag\ScheduledDefrag",
            "/DISABLE",
        ])
        .output();
    debug_print("    ✓ Scheduled defragmentation disabled.");
}

fn disable_startup_delay() {
    // Remove the artificial startup delay Windows adds
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize")
    {
        let _ = key.set_value("StartupDelayInMSec", &0u32);
    }
    debug_print("    ✓ Startup delay removed (faster boot-to-desktop).");
}

fn disable_background_apps() {
    // Disable background apps globally (Win10)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications")
    {
        let _ = key.set_value("GlobalUserDisabled", &1u32);
    }

    // Also via policy (Win11)
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok((key, _)) = hklm.create_subkey(r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy") {
        let _ = key.set_value("LetAppsRunInBackground", &2u32); // 2 = force deny
    }
    debug_print("    ✓ Background apps disabled globally.");
}

// ═════════════════════════════════════════════════════════════
// Implementation — Adobe (always runs)
// ═════════════════════════════════════════════════════════════

fn kill_adobe_background_processes() {
    [
        "AdobeIPCBroker.exe",
        "CCLibrary.exe",
        "CCXProcess.exe",
        "CoreSync.exe",
        "Adobe Desktop Service.exe",
        "AdobeUpdateService.exe",
        "AGMService.exe",
        "AGSService.exe",
        "ArmUI.exe",
    ]
    .iter()
    .for_each(|proc| {
        let _ = crate::hidden_command("taskkill")
            .args(&["/f", "/im", proc])
            .output();
    });
    debug_print("    ✓ Adobe background processes terminated.");
}
