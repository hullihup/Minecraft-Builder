// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod blocks;
mod build;
mod codegen;
mod examples;
mod i18n;
mod item_render;
mod items;
mod mcp;
mod model;
mod recipe;
mod resourcepack;
mod server;
mod theme;
mod tools;
mod ui;
mod update;

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
    // minecraft-builder --render-item <pack.zip> <out-dir> <id>...: item pictures as PNG (for tests).
    if args.len() >= 5 && args[1] == "--render-item" {
        let mut r = item_render::Renderer::new(&[PathBuf::from(&args[2])]);
        let dir = PathBuf::from(&args[3]);
        let _ = std::fs::create_dir_all(&dir);
        for id in &args[4..] {
            match r.render(id) {
                Some(img) => {
                    let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
                    let out = image::RgbaImage::from_raw(img.size[0] as u32, img.size[1] as u32, raw).unwrap();
                    let name = id.replace(':', "_");
                    let _ = out.save(dir.join(format!("{name}.png")));
                }
                None => eprintln!("{id}: nothing to draw"),
            }
        }
        std::process::exit(0);
    }
    // minecraft-builder --starter <index> <paper|fabric> <out.mcb>: writes a template project (for tests).
    if args.len() >= 5 && args[1] == "--starter" {
        let platform = if args[3] == "fabric" {
            model::Platform::Fabric
        } else {
            model::Platform::Paper
        };
        let all = examples::Starter::for_platform(platform);
        let starter = args[2]
            .parse::<usize>()
            .ok()
            .and_then(|i| all.get(i).copied())
            .unwrap_or(examples::Starter::Welcome);
        let p = examples::new_project("Vorlage", platform, starter);
        match p.save(std::path::Path::new(&args[4])) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
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
