#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod rand {
    include!(concat!(env!("OUT_DIR"), "/rand.rs"));
}

mod app;
mod node;
mod thread;
mod view;

use self::app::App;

#[cfg(not(target_arch = "wasm32"))]
use {
    log::warn,
    ron::ser::{to_string_pretty, PrettyConfig},
    serde::Serialize,
    std::{
        env::args,
        fs::OpenOptions,
        io::Write,
        path::{Path, PathBuf},
    },
};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let path_arg = args().nth(1).map(PathBuf::from);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Noise Gen",
        native_options,
        Box::new(|cc| {
            let mut app = App::new(cc);
            if let Some(path) = path_arg {
                app.open(path);
            }

            Ok(Box::new(app))
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window.")
            .document()
            .expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_ron<T>(path: impl AsRef<Path>, value: &T) -> anyhow::Result<()>
where
    T: ?Sized + Serialize,
{
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .inspect_err(|err| {
            warn!("Unable to create file: {err}");
        })?
        .write_all(to_string_pretty(value, PrettyConfig::default())?.as_bytes())
        .inspect_err(|err| {
            warn!("Unable to write file: {err}");
        })?;

    Ok(())
}
