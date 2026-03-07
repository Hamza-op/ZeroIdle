# IDM Fix & System Optimizer

A lightweight, high-performance system maintenance tool written in Rust using `egui`. It features a sleek, animated UI and provides one-click optimizations for Windows systems.

## 🚀 Features

*   **🔑 IDM Activation Reset:** Resets the Internet Download Manager trial activation securely and resolves the fake serial number popup.
*   **🧹 Temporary File Cleanup:** Safely sweeps local system temporary folders, clearing out junk to save disk space.
*   **🎮 Gaming Optimizations:** Modifies Windows registry settings for gaming performance, enabling GameDVR tuning and power throttling offsets.
*   **🎨 Adobe Optimization:** Clears out massive cache footprints created by various Adobe products.
*   **🛡 System & Privacy:** Implements core privacy and telemetry disabling natively via registry keys across Windows components.

## 🛠️ Built With

*   [Rust](https://www.rust-lang.org/) - Core logic and safe systems programming.
*   [egui](https://github.com/emilk/egui) - The immediate mode GUI library for Rust.
*   [eframe](https://crates.io/crates/eframe) - The framework for writing egui apps.
*   [winreg](https://crates.io/crates/winreg) - For managing Windows Registry Keys.

## 📦 Download

You can download the pre-compiled standalone executable from the repository here:
*   [idm-system-tool.exe](target/release/idm-system-tool.exe)

> **Note:** The program must run with Administrator Privileges to successfully modify registry keys and system-level directories. It will auto-prompt for elevation if started normally.

## 💻 Development & Compilation

If you wish to compile the tool yourself from the source code:

1.  Make sure you have [Rust and Cargo installed](https://rustup.rs/).
2.  Clone the repository:
    ```bash
    git clone https://your-repository-url.git
    cd idm-system-tool
    ```
3.  Build the project in release mode:
    ```bash
    cargo run --release
    ```

## 🔄 Startup Behavior

The `idm-system-tool` ensures your system and IDM specifically remain optimized and functional without manual intervention.

To achieve this, upon execution, the program will silently register itself to run when Windows boots up using the following registry key:
`HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
under the value `MetaLensOptimizer`.

This allows the IDM fix and temporary file cache clearing to automatically maintain a clean environment every time you turn on your PC.

### How to Stop/Remove from Startup

If you no longer wish for the tool to run automatically, you can easily disable it using the standard Windows settings:

**Option 1: Windows Task Manager**
1. Press `Ctrl + Shift + Esc` to open the Task Manager.
2. Navigate to the **Startup apps** tab (the speedometer icon on Windows 11).
3. Locate `idm-system-tool` in the list.
4. Right-click on it and select **Disable**.

**Option 2: Registry Editor**
1. Press `Win + R`, type `regedit`, and hit Enter.
2. Navigate to: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
3. Delete the `MetaLensOptimizer` value.

## ⚖️ Disclaimer

This tool makes direct modifications to the Windows Registry. Ensure you understand what these optimizations perform before executing them on a production machine.
