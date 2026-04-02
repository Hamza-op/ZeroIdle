#![windows_subsystem = "windows"]

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod admin;
mod cleanup;
mod idm;
mod killer;
mod optimize;
mod startup;

/// Toggle this to `false` for production silent mode.
const DEBUG: bool = true;

/// Log file path for diagnosing issues when console is hidden.
fn get_log_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|mut p| {
        p.push("ZeroIdle");
        let _ = std::fs::create_dir_all(&p);
        p.push("debug.log");
        p
    })
}

static LOG_MUTEX: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

pub fn debug_print(msg: &str) {
    if DEBUG {
        let _lock = LOG_MUTEX.lock().unwrap();
        println!("  {}", msg);
        if let Some(log_path) = get_log_path() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(f, "[{}] {}", timestamp, msg);
                let _ = f.flush();
            }
        }
    }
}

pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

// ─────────────────────────────────────────────────────────────
// Phase Tracking
// ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum PhaseStatus {
    Pending,
    Running,
    Done,
}

#[derive(Clone)]
struct Phase {
    name: &'static str,
    status: PhaseStatus,
    detail: String,
}

struct TaskState {
    pub is_done: bool,
    pub phases: Vec<Phase>,
    pub active_phase: usize,
    pub cleanup_stats: Option<cleanup::CleanupStats>,
}

impl TaskState {
    fn new() -> Self {
        Self {
            is_done: false,
            active_phase: 0,
            cleanup_stats: None,
            phases: vec![
                Phase {
                    name: "IDM Activator Script",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
                Phase {
                    name: "Temporary File Cleanup",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
                Phase {
                    name: "Gaming Optimizations",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
                Phase {
                    name: "Adobe Optimization",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
                Phase {
                    name: "System & Privacy",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
                Phase {
                    name: "Startup & Services",
                    status: PhaseStatus::Pending,
                    detail: String::new(),
                },
            ],
        }
    }

    fn start_phase(&mut self, idx: usize) {
        if idx < self.phases.len() {
            self.active_phase = idx;
            self.phases[idx].status = PhaseStatus::Running;
            self.phases[idx].detail.clear();
        }
    }

    fn set_detail(&mut self, idx: usize, detail: String) {
        if idx < self.phases.len() {
            self.phases[idx].detail = detail;
        }
    }

    fn complete_phase(&mut self, idx: usize) {
        if idx < self.phases.len() {
            self.phases[idx].status = PhaseStatus::Done;
            self.phases[idx].detail.clear();
        }
    }

    fn progress(&self) -> f32 {
        let done = self
            .phases
            .iter()
            .filter(|p| p.status == PhaseStatus::Done)
            .count();
        done as f32 / self.phases.len() as f32
    }
}

// ─────────────────────────────────────────────────────────────
// Color Palette — deep-space mission control
// ─────────────────────────────────────────────────────────────

const BG_BASE: egui::Color32 = egui::Color32::from_rgb(8, 8, 14);
const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(14, 14, 24);
const BG_ELEVATED: egui::Color32 = egui::Color32::from_rgb(20, 20, 34);

const ACCENT: egui::Color32 = egui::Color32::from_rgb(80, 200, 255); // Ice blue
const ACCENT_HOT: egui::Color32 = egui::Color32::from_rgb(255, 120, 50); // Warm amber
const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(50, 120, 180); // Muted steel

const SUCCESS: egui::Color32 = egui::Color32::from_rgb(70, 220, 130);
const WARN: egui::Color32 = egui::Color32::from_rgb(255, 180, 50);

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(210, 215, 230);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(110, 118, 145);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(50, 55, 72);

const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(24, 26, 40);
const RING_TRACK: egui::Color32 = egui::Color32::from_rgb(20, 22, 35);

// ─────────────────────────────────────────────────────────────
// GUI App
// ─────────────────────────────────────────────────────────────

struct MaintenanceApp {
    start_time: Instant,
    state: Arc<Mutex<TaskState>>,
    first_frame: bool,
    gui_alive: Arc<std::sync::atomic::AtomicBool>,
}

impl eframe::App for MaintenanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.gui_alive
                .store(true, std::sync::atomic::Ordering::Relaxed);
            debug_print("[✓] First frame rendering started.");
            self.first_frame = false;
        }

        let state = self.state.lock().unwrap();
        let is_done = state.is_done;
        let progress = state.progress();
        let phases: Vec<Phase> = state.phases.iter().cloned().collect();
        let cleanup_stats = state.cleanup_stats.clone();
        drop(state);

        if is_done {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let elapsed = self.start_time.elapsed().as_secs();
        let t = ctx.input(|i| i.time) as f32;

        let panel = egui::Frame::NONE
            .fill(BG_BASE)
            .inner_margin(egui::Margin::same(0));

        egui::CentralPanel::default().frame(panel).show(ctx, |ui| {
            let full_rect = ui.available_rect_before_wrap();
            let painter = ui.painter();

            // ── Top accent line — thin animated gradient ──
            let bar_h = 2.0;
            let steps = 40;
            let step_w = full_rect.width() / steps as f32;
            for i in 0..steps {
                let frac = i as f32 / steps as f32;
                let shift = ((t * 0.6 + frac * 2.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let c = lerp_color(ACCENT, ACCENT_HOT, shift);
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(full_rect.min.x + i as f32 * step_w, full_rect.min.y),
                        egui::vec2(step_w + 1.0, bar_h),
                    ),
                    0,
                    c,
                );
            }

            // ── Subtle corner accents (L-brackets) ──
            let corner_len = 14.0;
            let corner_col = egui::Color32::from_rgba_premultiplied(80, 200, 255, 30);
            let m = 6.0;
            // Top-left
            painter.line_segment(
                [
                    egui::pos2(full_rect.min.x + m, full_rect.min.y + bar_h + m),
                    egui::pos2(
                        full_rect.min.x + m,
                        full_rect.min.y + bar_h + m + corner_len,
                    ),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            painter.line_segment(
                [
                    egui::pos2(full_rect.min.x + m, full_rect.min.y + bar_h + m),
                    egui::pos2(
                        full_rect.min.x + m + corner_len,
                        full_rect.min.y + bar_h + m,
                    ),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            // Top-right
            painter.line_segment(
                [
                    egui::pos2(full_rect.max.x - m, full_rect.min.y + bar_h + m),
                    egui::pos2(
                        full_rect.max.x - m,
                        full_rect.min.y + bar_h + m + corner_len,
                    ),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            painter.line_segment(
                [
                    egui::pos2(full_rect.max.x - m, full_rect.min.y + bar_h + m),
                    egui::pos2(
                        full_rect.max.x - m - corner_len,
                        full_rect.min.y + bar_h + m,
                    ),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            // Bottom-left
            painter.line_segment(
                [
                    egui::pos2(full_rect.min.x + m, full_rect.max.y - m),
                    egui::pos2(full_rect.min.x + m, full_rect.max.y - m - corner_len),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            painter.line_segment(
                [
                    egui::pos2(full_rect.min.x + m, full_rect.max.y - m),
                    egui::pos2(full_rect.min.x + m + corner_len, full_rect.max.y - m),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            // Bottom-right
            painter.line_segment(
                [
                    egui::pos2(full_rect.max.x - m, full_rect.max.y - m),
                    egui::pos2(full_rect.max.x - m, full_rect.max.y - m - corner_len),
                ],
                egui::Stroke::new(1.0, corner_col),
            );
            painter.line_segment(
                [
                    egui::pos2(full_rect.max.x - m, full_rect.max.y - m),
                    egui::pos2(full_rect.max.x - m - corner_len, full_rect.max.y - m),
                ],
                egui::Stroke::new(1.0, corner_col),
            );

            // ── Content area ──
            let content_rect = full_rect.shrink2(egui::vec2(16.0, 0.0));
            let content_rect = egui::Rect::from_min_max(
                egui::pos2(content_rect.min.x, content_rect.min.y + bar_h + 10.0),
                egui::pos2(content_rect.max.x, content_rect.max.y - 6.0),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                ui.style_mut().visuals.override_text_color = Some(TEXT_PRIMARY);
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);

                // ── Header ──
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("ZEROIDLE")
                            .size(14.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    let (subtitle, sub_col) = if progress >= 1.0 {
                        ("All tasks completed.", SUCCESS)
                    } else {
                        ("Optimizing your system...", TEXT_SECONDARY)
                    };
                    ui.label(egui::RichText::new(subtitle).size(9.5).color(sub_col));
                });

                ui.add_space(6.0);

                // ── Progress Ring ──
                ui.vertical_centered(|ui| {
                    let ring_size = 80.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ring_size, ring_size),
                        egui::Sense::hover(),
                    );
                    let center = rect.center();
                    let radius = ring_size / 2.0 - 6.0;
                    let painter = ui.painter();

                    // Track ring
                    draw_arc(painter, center, radius, 3.0, 0.0, 1.0, RING_TRACK);

                    // Tick marks (static, subtle)
                    for i in 0..20 {
                        let angle =
                            (i as f32 / 20.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                        let r1 = radius + 3.0;
                        let r2 = radius + (if i % 5 == 0 { 6.0 } else { 4.0 });
                        let alpha = if i % 5 == 0 { 40u8 } else { 18u8 };
                        painter.line_segment(
                            [
                                egui::pos2(
                                    center.x + angle.cos() * r1,
                                    center.y + angle.sin() * r1,
                                ),
                                egui::pos2(
                                    center.x + angle.cos() * r2,
                                    center.y + angle.sin() * r2,
                                ),
                            ],
                            egui::Stroke::new(
                                0.8,
                                egui::Color32::from_rgba_premultiplied(210, 215, 230, alpha),
                            ),
                        );
                    }

                    if progress > 0.0 {
                        let arc_col = if progress >= 1.0 { SUCCESS } else { ACCENT };
                        draw_arc(painter, center, radius, 4.0, 0.0, progress, arc_col);

                        // Glow tip
                        if progress < 1.0 {
                            let angle =
                                -std::f32::consts::FRAC_PI_2 + progress * std::f32::consts::TAU;
                            let tip = egui::pos2(
                                center.x + angle.cos() * radius,
                                center.y + angle.sin() * radius,
                            );
                            let glow = ((t * 3.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
                            painter.circle_filled(
                                tip,
                                5.0,
                                egui::Color32::from_rgba_premultiplied(
                                    arc_col.r(),
                                    arc_col.g(),
                                    arc_col.b(),
                                    (glow * 35.0) as u8,
                                ),
                            );
                            painter.circle_filled(
                                tip,
                                2.5,
                                egui::Color32::from_rgba_premultiplied(
                                    255,
                                    255,
                                    255,
                                    (glow * 200.0) as u8,
                                ),
                            );
                        }
                    }

                    // Center percentage
                    let pct = (progress * 100.0) as u32;
                    let pct_col = if pct == 100 { SUCCESS } else { TEXT_PRIMARY };
                    painter.text(
                        egui::pos2(center.x, center.y - 3.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}%", pct),
                        egui::FontId::proportional(18.0),
                        pct_col,
                    );
                    painter.text(
                        egui::pos2(center.x, center.y + 12.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}s", elapsed),
                        egui::FontId::proportional(8.5),
                        TEXT_MUTED,
                    );
                });

                ui.add_space(5.0);

                // ── Thin separator line ──
                let sep_w = ui.available_width() * 0.6;
                let sep_x = content_rect.min.x + (content_rect.width() - sep_w) / 2.0;
                let sep_y = ui.cursor().min.y;
                ui.painter().line_segment(
                    [egui::pos2(sep_x, sep_y), egui::pos2(sep_x + sep_w, sep_y)],
                    egui::Stroke::new(0.5, BORDER_SUBTLE),
                );
                ui.add_space(5.0);

                // ── Phase List ──
                for phase in phases.iter() {
                    let is_active = phase.status == PhaseStatus::Running;
                    let is_done = phase.status == PhaseStatus::Done;

                    // Paint a custom row: left bar + content + right status
                    // Use a simple frame for the row
                    let row_bg = if is_active { BG_ELEVATED } else { BG_SURFACE };
                    let border_col = if is_active { ACCENT_DIM } else { BORDER_SUBTLE };

                    let card = egui::Frame::NONE
                        .fill(row_bg)
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin {
                            left: 10,
                            right: 8,
                            top: 5,
                            bottom: 5,
                        })
                        .stroke(egui::Stroke::new(0.5, border_col));

                    let resp = card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status dot — painted as a small filled circle via painter
                            let (dot_rect, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                            let dot_center = dot_rect.center();
                            if is_done {
                                ui.painter().circle_filled(dot_center, 3.5, SUCCESS);
                            } else if is_active {
                                let pulse = ((t * 3.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
                                let col = lerp_color(ACCENT, egui::Color32::WHITE, pulse * 0.3);
                                ui.painter().circle_filled(dot_center, 3.5, col);
                                // Outer pulse ring
                                let ring_alpha =
                                    ((t * 2.0).sin() * 80.0 + 80.0).clamp(20.0, 160.0) as u8;
                                ui.painter().circle_stroke(
                                    dot_center,
                                    5.5,
                                    egui::Stroke::new(
                                        0.8,
                                        egui::Color32::from_rgba_premultiplied(
                                            ACCENT.r(),
                                            ACCENT.g(),
                                            ACCENT.b(),
                                            ring_alpha,
                                        ),
                                    ),
                                );
                            } else {
                                ui.painter().circle_stroke(
                                    dot_center,
                                    3.0,
                                    egui::Stroke::new(0.8, TEXT_MUTED),
                                );
                            }

                            ui.add_space(4.0);

                            // Phase name
                            let name_col = if is_done {
                                TEXT_PRIMARY
                            } else if is_active {
                                ACCENT
                            } else {
                                TEXT_SECONDARY
                            };
                            let mut name_rt =
                                egui::RichText::new(phase.name).size(10.5).color(name_col);
                            if is_active {
                                name_rt = name_rt.strong();
                            }
                            ui.label(name_rt);

                            // Right-side status — use allocate to prevent expansion
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if is_done {
                                        ui.label(
                                            egui::RichText::new("done").size(8.0).color(SUCCESS),
                                        );
                                    } else if is_active {
                                        // Simple dot animation — safe ASCII
                                        let dots = match ((t * 3.0) as u32) % 4 {
                                            0 => ".",
                                            1 => "..",
                                            2 => "...",
                                            _ => "",
                                        };
                                        ui.label(
                                            egui::RichText::new(dots)
                                                .size(9.0)
                                                .strong()
                                                .color(ACCENT),
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new("--").size(8.0).color(TEXT_MUTED),
                                        );
                                    }
                                },
                            );
                        });

                        // Sub-detail for active phases
                        if !phase.detail.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(14.0);
                                ui.label(
                                    egui::RichText::new(&phase.detail)
                                        .size(8.0)
                                        .color(TEXT_SECONDARY)
                                        .italics(),
                                );
                            });
                        }
                    });

                    // Left accent bar
                    let r = resp.response.rect;
                    let bar_col = if is_done {
                        SUCCESS
                    } else if is_active {
                        ACCENT
                    } else {
                        BORDER_SUBTLE
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(r.min, egui::vec2(2.5, r.height())),
                        egui::CornerRadius {
                            nw: 4,
                            sw: 4,
                            ne: 0,
                            se: 0,
                        },
                        bar_col,
                    );

                    ui.add_space(1.5);
                }

                // ── Cleanup Stats ──
                if let Some(ref stats) = cleanup_stats {
                    ui.add_space(5.0);

                    let dash = egui::Frame::NONE
                        .fill(BG_SURFACE)
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin {
                            left: 10,
                            right: 10,
                            top: 6,
                            bottom: 6,
                        })
                        .stroke(egui::Stroke::new(0.5, BORDER_SUBTLE));

                    dash.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            stat_chip(
                                ui,
                                "FREED",
                                &cleanup::format_bytes(stats.bytes_freed),
                                SUCCESS,
                            );
                            ui.add_space(16.0);
                            stat_chip(ui, "FILES", &stats.deleted.to_string(), ACCENT);
                            ui.add_space(16.0);
                            stat_chip(ui, "SKIPPED", &stats.failed.to_string(), WARN);
                        });
                    });
                }

                // ── Footer ──
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("ZeroIdle v2.0  //  github.com/Hamza-op")
                            .size(7.5)
                            .color(TEXT_MUTED),
                    );
                });
            });
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

// ─────────────────────────────────────────────────────────────
// Drawing Helpers
// ─────────────────────────────────────────────────────────────

fn draw_arc(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    thickness: f32,
    start_frac: f32,
    end_frac: f32,
    color: egui::Color32,
) {
    let segments = 64;
    let start_angle = -std::f32::consts::FRAC_PI_2 + start_frac * std::f32::consts::TAU;
    let end_angle = -std::f32::consts::FRAC_PI_2 + end_frac * std::f32::consts::TAU;
    let step = (end_angle - start_angle) / segments as f32;

    (0..segments).for_each(|i| {
        let a1 = start_angle + step * i as f32;
        let a2 = start_angle + step * (i + 1) as f32;
        let p1 = egui::pos2(center.x + a1.cos() * radius, center.y + a1.sin() * radius);
        let p2 = egui::pos2(center.x + a2.cos() * radius, center.y + a2.sin() * radius);
        painter.line_segment([p1, p2], egui::Stroke::new(thickness, color));
    });
}

fn stat_chip(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(
            egui::RichText::new(label)
                .size(7.5)
                .strong()
                .color(TEXT_MUTED),
        );
        ui.label(egui::RichText::new(value).size(12.0).strong().color(color));
    });
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let inv = 1.0 - t;
    egui::Color32::from_rgb(
        (a.r() as f32 * inv + b.r() as f32 * t) as u8,
        (a.g() as f32 * inv + b.g() as f32 * t) as u8,
        (a.b() as f32 * inv + b.b() as f32 * t) as u8,
    )
}

// ─────────────────────────────────────────────────────────────
// Toast Notification (F14)
// ─────────────────────────────────────────────────────────────

fn send_toast_notification(title: &str, body: &str) {
    let ps_script = format!(
        r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null

$template = @"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>{}</text>
      <text>{}</text>
    </binding>
  </visual>
</toast>
"@

$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("ZeroIdle").Show($toast)
"#,
        title.replace('"', "'"),
        body.replace('"', "'"),
    );

    let _ = hidden_command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .spawn();
}

// ─────────────────────────────────────────────────────────────
// Main Execution
// ─────────────────────────────────────────────────────────────

/// Run all tasks without GUI (headless fallback).
fn run_headless() {
    debug_print("[i] Running in headless mode (no GUI)...");

    // Fast-path: log if all one-time optimizations are already applied
    if optimize::all_onetime_tasks_done() {
        debug_print(
            "[✓] All one-time optimizations already applied. Running always-run tasks only.",
        );
    }

    // Always-run tasks
    idm::run_activator();
    let stats = cleanup::clean_temp_files(None);
    let msg = format!(
        "Freed {} · {} files cleaned",
        cleanup::format_bytes(stats.bytes_freed),
        stats.deleted
    );
    send_toast_notification("ZeroIdle", &msg);

    // One-time tasks — skip if already completed
    if !optimize::is_task_done("gaming_opt") {
        optimize::optimize_for_gaming();
    } else {
        debug_print("[✓] Gaming optimizations already applied — skipped.");
    }

    // Adobe always runs (kills background procs)
    optimize::optimize_for_adobe();

    if !optimize::is_task_done("system_privacy") {
        optimize::optimize_system_and_privacy();
    } else {
        debug_print("[✓] System & Privacy already applied — skipped.");
    }

    if !optimize::is_task_done("startup_services") {
        optimize::optimize_startup_and_services();
    } else {
        debug_print("[✓] Startup & Services already applied — skipped.");
    }

    debug_print("[✓] Headless run complete.");
}

fn main() {
    if let Some(log_path) = get_log_path() {
        let _ = std::fs::write(&log_path, "");
    }
    debug_print("=== ZeroIdle starting ===");

    // Capture panics to log file (critical for diagnosing GPU/windowing crashes)
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("[PANIC] {}", info);
        debug_print(&msg);
    }));

    let args: Vec<String> = std::env::args().collect();
    kill_existing_instances();

    if args.iter().any(|a| a == "--daemon") {
        crate::killer::run_background_loop();
        return;
    }

    // Allow --headless flag to skip GUI entirely
    if args.iter().any(|a| a == "--headless") {
        if !admin::is_admin() {
            if admin::elevate_self() {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
        startup::ensure_startup_registered();
        run_headless();
        return;
    }

    if !admin::is_admin() {
        debug_print("[i] Not admin, requesting elevation...");
        if admin::elevate_self() {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    debug_print("[✓] Running as admin.");
    startup::ensure_startup_registered();
    optimize::migrate_legacy_flag();

    let state = Arc::new(Mutex::new(TaskState::new()));

    // Shared flag: set to true once the first frame renders
    let gui_alive = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let gui_alive_watchdog = gui_alive.clone();

    // Watchdog: if no frame renders in 10s, run headless and force-exit
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        if !gui_alive_watchdog.load(std::sync::atomic::Ordering::Relaxed) {
            debug_print("[⚠] GUI failed to render within 10s. Falling back to headless mode.");
            run_headless();
            std::process::exit(0);
        }
    });

    debug_print("[i] Launching GUI (glow/OpenGL backend)...");

    let try_run_gui = |renderer: eframe::Renderer,
                       state: Arc<Mutex<TaskState>>,
                       gui_alive: Arc<std::sync::atomic::AtomicBool>|
     -> Result<(), eframe::Error> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([310.0, 420.0])
                .with_resizable(false)
                .with_title("ZeroIdle"),
            renderer,
            ..Default::default()
        };

        eframe::run_native(
            "ZeroIdle",
            options,
            Box::new(move |_cc| {
                let state_clone = state.clone();
                thread::spawn(move || {
                    let cleanup_detail = Arc::new(Mutex::new(String::new()));
                    let run_phase =
                        |idx: usize,
                         detail_src: Option<Arc<Mutex<String>>>,
                         work: Box<dyn FnOnce() + Send>| {
                            state_clone.lock().unwrap().start_phase(idx);
                            if let Some(ref src) = detail_src {
                                let src2 = src.clone();
                                let sc2 = state_clone.clone();
                                let watcher = thread::spawn(move || loop {
                                    thread::sleep(Duration::from_millis(150));
                                    let detail = src2.lock().map(|s| s.clone()).unwrap_or_default();
                                    let mut st = sc2.lock().unwrap();
                                    if st.phases[idx].status != PhaseStatus::Running {
                                        break;
                                    }
                                    st.set_detail(idx, detail);
                                });
                                work();
                                state_clone.lock().unwrap().complete_phase(idx);
                                let _ = watcher.join();
                            } else {
                                work();
                                thread::sleep(Duration::from_millis(400));
                                state_clone.lock().unwrap().complete_phase(idx);
                            }
                        };

                    // === Skip helper for completed/inapplicable tasks ===
                    let skip_phase = |idx: usize, sc: &Arc<Mutex<TaskState>>, reason: &str| {
                        sc.lock().unwrap().start_phase(idx);
                        thread::sleep(Duration::from_millis(120));
                        sc.lock().unwrap().set_detail(idx, reason.into());
                        thread::sleep(Duration::from_millis(200));
                        sc.lock().unwrap().complete_phase(idx);
                    };

                    // === Phase 0: IDM Activator (always-run, but skip if IDM not installed) ===
                    if idm::is_idm_installed() {
                        run_phase(
                            0,
                            None,
                            Box::new(|| {
                                idm::run_activator();
                            }),
                        );
                    } else {
                        skip_phase(0, &state_clone, "IDM not installed — skipped");
                    }

                    // === Phase 1: Temp Cleanup (always-run) ===
                    let detail_clone = cleanup_detail.clone();
                    run_phase(
                        1,
                        Some(cleanup_detail),
                        Box::new(move || {
                            let stats = cleanup::clean_temp_files(Some(detail_clone));
                            CLEANUP_STATS.lock().unwrap().replace(stats);
                        }),
                    );

                    if let Some(stats) = CLEANUP_STATS.lock().unwrap().take() {
                        let msg = format!(
                            "Freed {} · {} files cleaned",
                            cleanup::format_bytes(stats.bytes_freed),
                            stats.deleted
                        );
                        state_clone.lock().unwrap().cleanup_stats = Some(stats);
                        send_toast_notification("System Optimizer", &msg);
                    }

                    // === One-time tasks — skip instantly if already done ===

                    // Phase 2: Gaming Optimizations
                    if optimize::is_task_done("gaming_opt") {
                        skip_phase(2, &state_clone, "already applied — skipped");
                    } else {
                        run_phase(
                            2,
                            None,
                            Box::new(|| {
                                optimize::optimize_for_gaming();
                            }),
                        );
                    }

                    // Phase 3: Adobe — always runs (kills background procs)
                    run_phase(
                        3,
                        None,
                        Box::new(|| {
                            optimize::optimize_for_adobe();
                        }),
                    );

                    // Phase 4: System & Privacy
                    if optimize::is_task_done("system_privacy") {
                        skip_phase(4, &state_clone, "already applied — skipped");
                    } else {
                        run_phase(
                            4,
                            None,
                            Box::new(|| {
                                optimize::optimize_system_and_privacy();
                            }),
                        );
                    }

                    // Phase 5: Startup & Services
                    if optimize::is_task_done("startup_services") {
                        skip_phase(5, &state_clone, "already applied — skipped");
                    } else {
                        run_phase(
                            5,
                            None,
                            Box::new(|| {
                                optimize::optimize_startup_and_services();
                            }),
                        );
                    }

                    thread::sleep(Duration::from_secs(3));
                    state_clone.lock().unwrap().is_done = true;
                });

                Ok(Box::new(MaintenanceApp {
                    start_time: Instant::now(),
                    state,
                    first_frame: true,
                    gui_alive,
                }))
            }),
        )
    };

    // Try glow (OpenGL) first — catch_unwind guards against hard panics in GPU init
    let glow_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        try_run_gui(eframe::Renderer::Glow, state.clone(), gui_alive.clone())
    }));

    match glow_result {
        Ok(Ok(_)) => {
            debug_print("[✓] GUI closed normally (glow).");
        }
        Ok(Err(e)) => {
            debug_print(&format!(
                "[✗] Glow backend failed: {}. Trying wgpu (D3D/Vulkan)...",
                e
            ));

            // Reset state for wgpu attempt
            let state2 = Arc::new(Mutex::new(TaskState::new()));
            let gui_alive2 = Arc::new(std::sync::atomic::AtomicBool::new(false));

            // New watchdog for wgpu attempt
            let gui_alive_watchdog2 = gui_alive2.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(10));
                if !gui_alive_watchdog2.load(std::sync::atomic::Ordering::Relaxed) {
                    debug_print("[⚠] wgpu GUI also failed to render. Falling back to headless.");
                    run_headless();
                    std::process::exit(0);
                }
            });

            let wgpu_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                try_run_gui(eframe::Renderer::Wgpu, state2, gui_alive2)
            }));

            match wgpu_result {
                Ok(Ok(_)) => {
                    debug_print("[✓] GUI closed normally (wgpu).");
                }
                Ok(Err(e2)) => {
                    debug_print(&format!(
                        "[✗] wgpu backend also failed: {}. Running headless.",
                        e2
                    ));
                    run_headless();
                }
                Err(_) => {
                    debug_print(
                        "[✗] wgpu backend panicked (no GPU/driver support). Running headless.",
                    );
                    run_headless();
                }
            }
        }
        Err(_) => {
            debug_print("[✗] Glow backend panicked. Running headless.");
            run_headless();
        }
    }
}

/// Kill all other running instances of our own executable.
fn kill_existing_instances() {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
    };

    let self_pid = unsafe { GetCurrentProcessId() };

    let self_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    let self_name = match self_name {
        Some(n) => n,
        None => return,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let handle = match snapshot {
            Ok(h) if !h.is_invalid() => h,
            _ => return,
        };

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(handle, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID != self_pid {
                    let name_len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name =
                        String::from_utf16_lossy(&entry.szExeFile[..name_len]).to_lowercase();

                    if name == self_name {
                        if let Ok(proc) = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID)
                        {
                            let _ = TerminateProcess(proc, 0);
                            let _ = windows::Win32::Foundation::CloseHandle(proc);
                            debug_print(&format!(
                                "[i] Killed old instance PID {}",
                                entry.th32ProcessID
                            ));
                        }
                    }
                }

                if Process32NextW(handle, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(handle);
    }
}

/// Global channel for cleanup stats
static CLEANUP_STATS: std::sync::LazyLock<Mutex<Option<cleanup::CleanupStats>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
