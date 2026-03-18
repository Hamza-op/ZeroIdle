//! IDM Activation script execution

use crate::debug_print;
use std::process::Command;
use std::os::windows::process::CommandExt;

pub fn run_activator() {
    debug_print("[⟳] Checking for latest IDM Activator...");

    // 1. Get latest tag using curl natively (incredibly faster than PowerShell startup)
    let curl_out = Command::new("curl")
        .args(["-s", "https://codeberg.org/api/v1/repos/oop7/IDM-Activator/releases/latest"])
        .creation_flags(0x08000000)
        .output();

    let mut latest_tag_str = String::new();
    if let Ok(out) = curl_out {
        let json = String::from_utf8_lossy(&out.stdout);
        // Simple manual JSON parse since we don't have serde_json
        if let Some(idx) = json.find("\"tag_name\":\"") {
            let start = idx + 12;
            if let Some(end) = json[start..].find("\"") {
                latest_tag_str = json[start..start + end].to_string();
            }
        }
    }

    if latest_tag_str.is_empty() {
        debug_print("  [✗] Failed to fetch latest IDM release. Skipping.");
        return;
    }

    // 2. Check registry using winreg natively
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let reg_path = r"Software\MetaLensOptimizer";
    
    // Create or open the key
    let key = match hkcu.open_subkey(reg_path) {
        Ok(k) => k,
        Err(_) => hkcu.create_subkey(reg_path).unwrap().0,
    };
    
    if let Ok(current_tag) = key.get_value::<String, _>("IdmActivatorVersion") {
        if current_tag == latest_tag_str {
            debug_print(&format!("  [✓] IDM already patched with latest version ({}). Skipping.", current_tag));
            return;
        }
    }

    debug_print(&format!("  [⟳] New version {} found. Launching patcher...", latest_tag_str));

    // 3. Fallback to PowerShell only for downloading and executing the patch
    let ps_script = format!(r#"
$ErrorActionPreference = 'Stop'
try {{
    $latestTag = "{}"
    $apiUrl = "https://codeberg.org/api/v1/repos/oop7/IDM-Activator/releases/latest"
    $release = Invoke-RestMethod -Uri $apiUrl
    $asset = $release.assets | Where-Object {{ $_.name -eq 'IDM-Activator.zip' }} | Select-Object -First 1
    
    if (-not $asset) {{
        Write-Error "Could not find IDM-Activator.zip in the latest release."
        exit 1
    }}
    
    $downloadUrl = $asset.browser_download_url
    
    $zipFile = Join-Path $env:TEMP "IDM-Activator.zip"
    $extractPath = Join-Path $env:TEMP "IDM-Activator"
    
    if (Test-Path $extractPath) {{ Remove-Item -Path $extractPath -Recurse -Force }}
    Invoke-RestMethod -Uri $downloadUrl -OutFile $zipFile
    Expand-Archive -Path $zipFile -DestinationPath $extractPath -Force
    
    $batPath = Join-Path $extractPath "IDM-Activator\script.bat"
    if (-not (Test-Path $batPath)) {{
        $batPath = Get-ChildItem -Path $extractPath -Filter "script.bat" -Recurse | Select-Object -ExpandProperty FullName -First 1
    }}
    
    if ($batPath) {{        
        $inputPath = Join-Path $env:TEMP "idm_input.txt"
        Set-Content -Path $inputPath -Value "y`r`n1`r`n`r`n`r`n"
        
        $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c `"$batPath`"" -RedirectStandardInput $inputPath -Wait -PassThru -WindowStyle Hidden
        
        if ($process.ExitCode -ne 0) {{
            Write-Warning "Activator script returned non-zero exit code: $($process.ExitCode)"
        }}
        
        if (Test-Path $inputPath) {{ Remove-Item $inputPath -Force }}
    }} else {{
        Write-Error "script.bat not found in the downloaded archive."
        exit 1
    }}
    
    if (Test-Path $zipFile) {{ Remove-Item $zipFile -Force }}
    if (Test-Path $extractPath) {{ Remove-Item $extractPath -Recurse -Force }}
}} catch {{
    Write-Error $_.Exception.Message
    exit 1
}}
"#, latest_tag_str);

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .creation_flags(0x08000000)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                // Save version after successful completion
                let _ = key.set_value("IdmActivatorVersion", &latest_tag_str);
                debug_print("  [✓] IDM Activator script executed successfully.");
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                debug_print(&format!("  [✗] IDM Activator script failed: {}", err.trim()));
            }
        }
        Err(e) => {
            debug_print(&format!("  [✗] Failed to execute PowerShell: {}", e));
        }
    }
}
