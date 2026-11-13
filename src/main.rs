// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod blocks;
mod build;
mod codegen;
mod examples;
mod i18n;
mod model;
mod resourcepack;
mod theme;
mod tools;
mod ui;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Headless helpers, used for testing and scripting:
    //   minecraft-builder --export <project.mcb> <out-dir>
    //   minecraft-builder --selftest <paper|fabric> <out-dir>
    // Used by the installer: store the language picked during installation.
    if args.get(1).map(|a| a == "--lang").unwrap_or(false) {
        let mut s = build::AppSettings::load();
        s.german = args.get(2).map(|l| l != "en").unwrap_or(true);
        s.save();
        std::process::exit(0);
    }
    if args.get(1).map(|a| a == "--setup-tools").unwrap_or(false) {
        std::process::exit(tools::setup_cli());
    }
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
        let dir = PathBuf::from(&args[3]);
        let mut out = codegen::generate(&project);
        let project_file = if args[1] == "--export" {
            Some(PathBuf::from(&args[2]))
        } else {
            // The self test bundles a tiny resource pack.
            let _ = std::fs::create_dir_all(&dir);
            if let Err(e) = std::fs::write(dir.join("testpack.zip"), examples::test_pack_zip()) {
                eprintln!("cannot write test pack: {e}");
            }
            Some(dir.join("project.mcb"))
        };
        match codegen::pack_files(&project, project_file.as_deref()) {
            Ok(f) => out.files.extend(f),
            Err(e) => {
                eprintln!("ERROR {e}");
                std::process::exit(1);
            }
        }
        for p in &out.problems {
            eprintln!(
                "{} [{:?}] {}",
                if p.error { "ERROR" } else { "warn " },
                p.node,
                p.msg
            );
        }
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
