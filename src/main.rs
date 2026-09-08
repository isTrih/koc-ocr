use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};
use whwjm_ocr::{
    DEFAULT_MODEL_CDN_BASE_URL, ExportEvent, ExportOptions, OcrModelTier, ScreenshotKind,
    desktop_ocr_engine_config, ensure_ocr_models_available_with_progress, export_csv_with_events,
};

#[derive(Debug, Parser)]
#[command(
    name = "koc-ocr-cli",
    version,
    about = "KOC 抖音 OCR 命令行工具",
    author = "Lilith@HUAHaohui"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 下载并初始化 OCR 模型。
    Init {
        /// 模型档位。
        #[arg(long, value_enum, default_value_t = CliModelTier::Medium)]
        model: CliModelTier,

        /// 模型保存目录。
        #[arg(long, default_value_os_t = default_model_dir())]
        model_dir: PathBuf,

        /// 模型 CDN 地址。
        #[arg(long, default_value = DEFAULT_MODEL_CDN_BASE_URL)]
        cdn: String,
    },

    /// 识别目录内数字命名图片，并导出 CSV 表格。
    Export {
        /// 图片目录。目录内图片建议使用 1.png、2.jpg 这样的数字文件名。
        image_dir: PathBuf,

        /// 截图类型。auto 会自动区分 live/video；指定 live 或 video 会把整个目录按该类型处理。
        #[arg(long = "type", value_enum, default_value_t = CliScreenshotKind::Auto)]
        screenshot_type: CliScreenshotKind,

        /// 模型档位。
        #[arg(long, value_enum, default_value_t = CliModelTier::Medium)]
        model: CliModelTier,

        /// 模型保存目录。
        #[arg(long, default_value_os_t = default_model_dir())]
        model_dir: PathBuf,

        /// CSV 输出目录，默认输出到图片目录。
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 模型 CDN 地址。
        #[arg(long, default_value = DEFAULT_MODEL_CDN_BASE_URL)]
        cdn: String,

        /// 不自动下载缺失模型。适合离线或由 init 预先准备模型的场景。
        #[arg(long)]
        no_download: bool,

        /// 只输出 CSV 文件路径，不把 CSV 内容打印到终端。
        #[arg(long)]
        quiet: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliModelTier {
    Tiny,
    Small,
    Medium,
}

impl std::fmt::Display for CliModelTier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_ocr_model_tier().as_str())
    }
}

impl CliModelTier {
    fn as_ocr_model_tier(self) -> OcrModelTier {
        match self {
            Self::Tiny => OcrModelTier::Tiny,
            Self::Small => OcrModelTier::Small,
            Self::Medium => OcrModelTier::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliScreenshotKind {
    Auto,
    Live,
    Video,
}

impl std::fmt::Display for CliScreenshotKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_screenshot_kind().as_str())
    }
}

impl CliScreenshotKind {
    fn as_screenshot_kind(self) -> ScreenshotKind {
        match self {
            Self::Auto => ScreenshotKind::Auto,
            Self::Live => ScreenshotKind::Live,
            Self::Video => ScreenshotKind::Video,
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Init {
            model,
            model_dir,
            cdn,
        } => init_models(model, &model_dir, &cdn),
        Command::Export {
            image_dir,
            screenshot_type,
            model,
            model_dir,
            output,
            cdn,
            no_download,
            quiet,
        } => export_images(ExportCommand {
            image_dir,
            screenshot_type,
            model,
            model_dir,
            output,
            cdn,
            no_download,
            quiet,
        }),
    }
}

struct ExportCommand {
    image_dir: PathBuf,
    screenshot_type: CliScreenshotKind,
    model: CliModelTier,
    model_dir: PathBuf,
    output: Option<PathBuf>,
    cdn: String,
    no_download: bool,
    quiet: bool,
}

fn init_models(model: CliModelTier, model_dir: &Path, cdn: &str) -> Result<()> {
    println!("初始化 {} 模型：{}", model, model_dir.display());
    ensure_ocr_models_available_with_progress(
        model_dir,
        model.as_ocr_model_tier(),
        cdn,
        &mut print_model_download_event,
    )?;
    println!("模型已就绪");
    Ok(())
}

fn export_images(command: ExportCommand) -> Result<()> {
    let output_dir = command
        .output
        .clone()
        .unwrap_or_else(|| command.image_dir.clone());
    let mut options = ExportOptions::new(&command.image_dir)
        .with_model_dir(&command.model_dir)
        .with_output_dir(&output_dir)
        .with_model_tier(command.model.as_ocr_model_tier())
        .with_screenshot_kind(command.screenshot_type.as_screenshot_kind())
        .with_ocr_engine_config(desktop_ocr_engine_config());

    options.model_cdn_base_url = command.cdn;
    options.download_missing_models = !command.no_download;

    let summary = export_csv_with_events(options, |event| match event {
        ExportEvent::ModelDownload(progress) => {
            print_model_download_event(ExportEvent::ModelDownload(progress))
        }
        ExportEvent::Image(progress) => {
            println!(
                "处理 {}/{} {}{}",
                progress.current,
                progress.total,
                progress.image_path.display(),
                if progress.cache_hit {
                    "（缓存）"
                } else {
                    ""
                }
            );
        }
    })?;

    match command.screenshot_type {
        CliScreenshotKind::Live => print_csv_result("直播", &summary.live_csv_path, command.quiet)?,
        CliScreenshotKind::Video => {
            print_csv_result("视频", &summary.video_csv_path, command.quiet)?
        }
        CliScreenshotKind::Auto => {
            print_csv_result("直播", &summary.live_csv_path, command.quiet)?;
            print_csv_result("视频", &summary.video_csv_path, command.quiet)?;
        }
    }

    Ok(())
}

fn print_model_download_event(event: ExportEvent) {
    let ExportEvent::ModelDownload(progress) = event else {
        return;
    };

    let total = progress
        .total_bytes
        .map(format_bytes)
        .unwrap_or_else(|| "未知大小".to_owned());
    let percent = progress
        .total_bytes
        .filter(|total| *total > 0)
        .map(|total| {
            format!(
                "{:.1}%",
                progress.downloaded_bytes as f64 * 100.0 / total as f64
            )
        })
        .unwrap_or_else(|| "--".to_owned());

    println!(
        "下载 {}：{} / {}，{}，{}",
        progress.file_name,
        format_bytes(progress.downloaded_bytes),
        total,
        percent,
        format_speed(progress.bytes_per_second)
    );
}

fn print_csv_result(label: &str, csv_path: &Path, quiet: bool) -> Result<()> {
    println!("{}CSV：{}", label, csv_path.display());
    if quiet {
        return Ok(());
    }

    let csv = fs::read_to_string(csv_path)
        .with_context(|| format!("读取 CSV 失败：{}", csv_path.display()))?;
    println!();
    println!("----- {} CSV 内容 -----", label);
    print!("{csv}");
    if !csv.ends_with('\n') {
        println!();
    }
    println!("----------------------");
    Ok(())
}

fn default_model_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("koc-ocr-cli").join("models");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("koc-ocr-cli")
                .join("models");
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("koc-ocr-cli")
            .join("models");
    }

    PathBuf::from("models")
}

fn format_speed(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second.max(0.0) as u64))
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{} B", bytes as u64)
    }
}
