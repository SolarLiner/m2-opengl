use std::{
    ffi::CString,
    fs::File,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Context;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin::{config::Api, display::GetGlDisplay};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, Version},
    prelude::*,
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
// use tracing::Event;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};
use tracing_tracy::TracyLayer;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, StartCause, VirtualKeyCode, WindowEvent},
    event::{Event, KeyboardInput},
    event_loop::EventLoopBuilder,
    window::{Fullscreen, WindowBuilder},
};

pub mod camera;
pub mod gbuffers;
pub mod light;
pub mod material;
pub mod mesh;
pub mod screen_draw;
pub mod transform;

pub trait Application: Sized + Send + Sync {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb
    }
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self>;
    fn resize(&mut self, size: PhysicalSize<u32>);
    fn interact(&mut self, event: WindowEvent);
    /// /!\ Does not run on the main thread. OpenGL calls are unsafe here.
    fn tick(&mut self, dt: Duration);
    fn render(&mut self);
}

pub fn run<App: 'static + Application>(title: &str) -> anyhow::Result<()> {
    color_eyre::install()?;
    let fmt_layer = tracing_subscriber::fmt::Layer::default()
        .pretty()
        .with_filter(EnvFilter::from_default_env());
    let json_layer = tracing_subscriber::fmt::Layer::default()
        .json()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .with_writer(File::create("log.jsonl").unwrap());
    let tracy_layer = TracyLayer::new();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(json_layer)
        .with(tracy_layer)
        .init();

    let event_loop = EventLoopBuilder::new().build();
    // The template will match only the configurations supporting rendering to
    // windows.
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder =
        DisplayBuilder::new().with_window_builder(Some(WindowBuilder::new().with_title(title)));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            // Find the config with the maximum number of samples, so our triangle will
            // be smooth.
            configs
                // .reduce(|accum, config| {
                //     let transparency_check = config.supports_transparency().unwrap_or(false)
                //         & !accum.supports_transparency().unwrap_or(false);
                //     if transparency_check || config.num_samples() > accum.num_samples() {
                //         config
                //     } else {
                //         accum
                //     }
                // })
                .inspect(|config| tracing::debug!(message="Potential config", api=?config.api(), depth_size=%config.depth_size()))
                .find(|config| config.api().contains(Api::OPENGL) && config.depth_size() >= 24)
                .expect("Couldn't find a suitable OpenGL configuration")
        })
        .map_err(|err| anyhow::anyhow!("Cannot create OpenGL configuration & window: {}", err))?;
    let window = window.expect("No window despite configuration");
    tracing::debug!(message="Using config", api=?gl_config.api(), depth_size=%gl_config.depth_size());

    let raw_window_handle = Some(window.raw_window_handle());

    // XXX The display could be obtained from the any object created by it, so we
    // can query it from the config.
    let gl_display = gl_config.display();

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new()
        .with_debug(true)
        .with_profile(glutin::context::GlProfile::Core)
        .with_robustness(glutin::context::Robustness::RobustNoResetNotification)
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_display
                    .create_context(&gl_config, &fallback_context_attributes)
                    .unwrap()
            })
    };
    // let attrs = window.build_surface_attributes(Default::default());
    let inner_size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.raw_window_handle(),
        inner_size.width.try_into().unwrap(),
        inner_size.height.try_into().unwrap(),
    );
    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };
    let context = not_current_gl_context
        .make_current(&gl_surface)
        .context("Cannot make OpenGL context current")?;
    violette_low::load_with(|sym| {
        let sym = CString::new(sym).unwrap();
        gl_display.get_proc_address(sym.as_c_str()).cast()
    });
    violette_low::debug::set_message_callback(|data| {
        use violette_low::debug::CallbackSeverity::*;
        match data.severity {
            Notification => {
                tracing::debug!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Low => {
                tracing::info!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Medium => {
                tracing::warn!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            High => {
                tracing::error!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
        };
    });

    let gl_version = violette_low::get_string(violette_low::gl::VERSION)
        .unwrap_or_else(|_| "<None>".to_string());
    let gl_vendor =
        violette_low::get_string(violette_low::gl::VENDOR).unwrap_or_else(|_| "<None>".to_string());
    let gl_renderer = violette_low::get_string(violette_low::gl::RENDERER)
        .unwrap_or_else(|_| "<None>".to_string());
    let gl_shading_language_version =
        violette_low::get_string(violette_low::gl::SHADING_LANGUAGE_VERSION)
            .unwrap_or_else(|_| "<None>".to_string());
    tracing::info!(target: "gl", version=%gl_version, vendor=%gl_vendor, render=%gl_renderer, shading_language=%gl_shading_language_version);

    let app = App::new(inner_size.cast()).context("Cannot run app")?;
    let app = Arc::new(Mutex::new(app));

    std::thread::spawn({
        let app = app.clone();
        move || {
            let mut last_tick = Instant::now();
            loop {
                let tick_start = Instant::now();
                app.lock().unwrap().tick(last_tick.elapsed());
                let tick_duration = tick_start.elapsed().as_secs_f32();
                last_tick = Instant::now();
                tracing::debug!(%tick_duration);
                std::thread::sleep(Duration::from_nanos(4_166_167)); // 240 FPS
            }
        }
    });

    let mut next_frame_time = Instant::now() + std::time::Duration::from_nanos(16_666_667);
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait_until(next_frame_time);

        match event {
            Event::RedrawRequested(_) => {
                let mut app = app.lock().unwrap();
                let frame_start = Instant::now();
                app.render();
                gl_surface.swap_buffers(&context).unwrap();
                let frame_time = frame_start.elapsed().as_secs_f32();
                tracing::debug!(%frame_time);
                next_frame_time = frame_start + Duration::from_nanos(16_666_667);
            }
            winit::event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.set_exit(),
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::F11),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    if window.fullscreen().is_some() {
                        window.set_fullscreen(None)
                    } else {
                        window.set_fullscreen(Some(Fullscreen::Borderless(None)))
                    }
                }
                WindowEvent::Resized(new_size) => {
                    gl_surface.resize(
                        &context,
                        new_size.width.try_into().unwrap(),
                        new_size.height.try_into().unwrap(),
                    );
                    app.lock().unwrap().resize(new_size);
                    window.request_redraw();
                }
                event => app.lock().unwrap().interact(event),
            },
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => window.request_redraw(),
            _ => {}
        }
    });
    Ok(())
}
