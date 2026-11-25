#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod logo;
mod mnp;
mod model;
mod schedule;
mod studio;

use std::{path::PathBuf, time::Duration};

use eframe::egui::{self, Event, ViewportBuilder, ViewportCommand};
use studio::Studio;

struct Screenshot {
    path: PathBuf,
    frames: u16,
}

struct Window {
    studio: Studio,
    screenshot: Option<Screenshot>,
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.studio.show(ctx);
        let Some(shot) = &mut self.screenshot else {
            return;
        };
        shot.frames += 1;
        if shot.frames == 120 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        }) {
            let pixels: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            let [width, height] = image.size;
            let result = (|| {
                if let Some(parent) = shot.path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                image::save_buffer(
                    &shot.path,
                    &pixels,
                    width as u32,
                    height as u32,
                    image::ColorType::Rgba8,
                )
                .map_err(std::io::Error::other)
            })();
            if let Err(error) = result {
                eprintln!("Capture impossible : {error}");
                std::process::exit(1);
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
            self.screenshot = None;
        } else if shot.frames >= 600 {
            eprintln!("La fenêtre n’a pas fourni de capture.");
            std::process::exit(1);
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn main() -> eframe::Result {
    let mut network_example = false;
    let mut open = None;
    let mut screenshot = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--network-example") => network_example = true,
            Some("--open") => open = Some(argument_path(&mut args, "--open")),
            Some("--screenshot") => {
                screenshot = Some(Screenshot {
                    path: argument_path(&mut args, "--screenshot"),
                    frames: 0,
                })
            }
            _ if mnp::is_project(std::path::Path::new(&arg))
                || mnp::is_diagram(std::path::Path::new(&arg))
                || std::path::Path::new(&arg)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("logique")) =>
            {
                open = Some(arg.into())
            }
            _ => {}
        }
    }
    let mut viewport = ViewportBuilder::default()
        .with_inner_size([1200.0, 760.0])
        .with_min_inner_size([860.0, 540.0]);
    let capturing = screenshot.is_some();
    if capturing {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if capturing {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            builder.with_activate_ignoring_other_apps(false);
        }));
    }
    eframe::run_native(
        "Atelier logique",
        options,
        Box::new(|cc| {
            studio::theme::style(&cc.egui_ctx);
            let mut studio = Studio::new();
            if let Some(path) = open {
                studio.open_path(path);
            } else if network_example {
                studio.load_network_example();
            }
            Ok(Box::new(Window { studio, screenshot }))
        }),
    )
}

fn argument_path(args: &mut impl Iterator<Item = std::ffi::OsString>, flag: &str) -> PathBuf {
    args.next().map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("Indiquer le chemin du fichier après {flag}.");
        std::process::exit(2);
    })
}
