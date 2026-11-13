// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod blocks;
mod build;
mod codegen;
mod examples;
mod i18n;
mod model;
mod theme;
mod ui;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Headless helpers, used for testing and scripting:
    //   minecraft-builder --export <project.mcb> <out-dir>
    //   minecraft-builder --selftest <paper|fabric> <out-dir>
    if args.len() >= 4 && (args[1] == "--export" || args[1] == "--selftest") {
        let project = if args[1] == "--export" {
            match model::Project::load(std::path::Path::new(&args[2])) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("cannot load project: {e}");
                    std::process::exit(2);
                }
            }
        } else {
            let platform = if args[2] == "fabric" {
                model::Platform::Fabric
            } else {
                model::Platform::Paper
            };
            examples::all_blocks_project(platform)
        };
        let out = codegen::generate(&project);
        for p in &out.problems {
            eprintln!(
                "{} [{:?}] {}",
                if p.error { "ERROR" } else { "warn " },
                p.node,
                p.msg
            );
        }
        let dir = PathBuf::from(&args[3]);
        if let Err(e) = codegen::write_to(&out, &dir, &project.settings.package) {
            eprintln!("cannot write: {e}");
            std::process::exit(2);
        }
        if args[1] == "--selftest" {
            let _ = project.save(&dir.join("project.mcb"));
        }
        println!("exported to {}", dir.display());
        std::process::exit(if out.has_errors() { 1 } else { 0 });
    }

    let open = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Minecraft Builder")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 560.0])
            .with_icon(theme::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Minecraft Builder",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, open)))),
    )
}
