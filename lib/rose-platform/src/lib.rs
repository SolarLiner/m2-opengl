use std::sync::RwLock;
use std::{
    ffi::CString,
    fs::File,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use eyre::{eyre, Context, Result};
use glutin::{
    config::{Api, ConfigTemplateBuilder},
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::GetGlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use histo::Histogram;
use raw_window_handle::HasRawWindowHandle;
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, EnvFilter};
pub use winit::dpi::{LogicalSize, PhysicalSize};
pub use winit::event as events;
use winit::event_loop::ControlFlow;
pub use winit::window::WindowBuilder;
use winit::{
    event::{ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopBuilder,
    window::Fullscreen,
};

use crate::circbuffer::CircBuffer;

pub mod circbuffer;

#[derive(Debug, Copy, Clone)]
pub struct TickContext {
    pub dt: Duration,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub struct RenderStats {
    fps_circ: CircBuffer<f32>,
    fps_hist: Histogram,
}

impl RenderStats {
    pub fn percentile(&self, pc: usize) -> u64 {
        self.fps_hist
            .buckets()
            .nth(pc)
            .map(|bucket| bucket.start())
            .unwrap_or(0)
    }

    pub fn fps_average(&self) -> f32 {
        self.fps_circ.iter().sum::<f32>() / self.fps_circ.len() as f32
    }

    pub fn fps_history(&self) -> impl '_ + Iterator<Item = f32> {
        self.fps_circ.iter().copied()
    }

    fn add_frame_time(&mut self, fps: f32) {
        self.fps_hist.add(fps as _);
        self.fps_circ.add(fps);
    }
}

#[derive(Debug)]
pub struct RenderContext<'stats, 'flow> {
    pub elapsed: Duration,
    pub stats: &'stats RenderStats,
    pub dt: Duration,
    control_flow: &'flow mut ControlFlow,
}

impl<'stats, 'flow> RenderContext<'stats, 'flow> {
    pub fn quit(&mut self) {
        self.control_flow.set_exit();
    }
}

#[cfg(not(feature = "ui"))]
pub struct UiContext<'stats> {
    pub elapsed: Duration,
    pub dt: Duration,
    pub stats: &'stats RenderStats,
}

#[cfg(feature = "ui")]
pub struct UiContext<'stats, 'ui> {
    pub elapsed: Duration,
    pub dt: Duration,
    pub stats: &'stats RenderStats,
    pub egui: &'ui egui::Context,
}

#[allow(unused_variables)]
pub trait Application: Sized + Send + Sync {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb
    }
    fn new(size: PhysicalSize<f32>) -> Result<Self>;
    fn resize(&mut self, _size: PhysicalSize<u32>) -> Result<()> {
        Ok(())
    }
    fn interact(&mut self, _event: WindowEvent) -> Result<()> {
        Ok(())
    }
    /// /!\ Does not run on the main thread. OpenGL calls are unsafe here.
    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        Ok(())
    }
    fn render(&mut self, ctx: RenderContext) -> Result<()>;
    #[cfg(feature = "ui")]
    fn ui(&mut self, ctx: UiContext) {}
}

pub fn run<App: 'static + Application>(title: &str) -> Result<()> {
    color_eyre::install()?;
    let fmt_layer =
        tracing_subscriber::fmt::Layer::default().with_filter(EnvFilter::from_default_env());
    let json_layer = tracing_subscriber::fmt::Layer::default()
        .json()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .with_writer(File::create("log.jsonl").unwrap());
    let registry = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(json_layer);
    #[cfg(feature = "tracy")]
    let registry = {
        let tracy_layer = tracing_tracy::TracyLayer::new();
        registry.with(tracy_layer)
    };
    registry.init();

    let event_loop = EventLoopBuilder::new().build();
    // The template will match only the configurations supporting rendering to
    // windows.
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder = DisplayBuilder::new().with_window_builder(Some(
        App::window_features(WindowBuilder::new()).with_title(title),
    ));

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
        .map_err(|err| eyre!("Cannot create OpenGL configuration & window: {}", err))?;
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
        .with_debug(cfg!(debug_assertions))
        .with_profile(glutin::context::GlProfile::Core)
        .with_robustness(glutin::context::Robustness::RobustLoseContextOnReset)
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(raw_window_handle);

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .context("Cannot create OpenGL display context")?
    };
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
    violette::load_with(|sym| {
        let sym = CString::new(sym).unwrap();
        gl_display.get_proc_address(sym.as_c_str()).cast()
    });
    violette::debug::hook_gl_to_tracing();

    let gl_version =
        violette::get_string(violette::gl::VERSION).unwrap_or_else(|_| "<None>".to_string());
    let gl_vendor =
        violette::get_string(violette::gl::VENDOR).unwrap_or_else(|_| "<None>".to_string());
    let gl_renderer =
        violette::get_string(violette::gl::RENDERER).unwrap_or_else(|_| "<None>".to_string());
    let gl_shading_language_version = violette::get_string(violette::gl::SHADING_LANGUAGE_VERSION)
        .unwrap_or_else(|_| "<None>".to_string());
    tracing::info!(target: "gl", version=%gl_version, vendor=%gl_vendor, render=%gl_renderer, shading_language=%gl_shading_language_version);

    let app = App::new(inner_size.cast()).context("Cannot run app")?;
    let app = Arc::new(Mutex::new(app));

    #[cfg(feature = "ui")]
    let mut ui = rose_ui::Ui::new(&event_loop, &window)?;

    let start = Instant::now();
    std::thread::spawn({
        let app = app.clone();
        move || {
            let mut last_tick = Instant::now();
            loop {
                let _span = tracing::trace_span!("loop_tick").entered();
                let tick_start = Instant::now();
                app.lock()
                    .unwrap()
                    .tick(TickContext {
                        elapsed: start.elapsed(),
                        dt: last_tick.elapsed(),
                    })
                    .unwrap();
                let tick_duration = tick_start.elapsed().as_secs_f32();
                last_tick = Instant::now();
                tracing::debug!(%tick_duration);
                std::thread::sleep(Duration::from_nanos(4_166_167)); // 240 FPS
            }
        }
    });

    let render_stats = Arc::new(RwLock::new(RenderStats {
        fps_circ: CircBuffer::new(60),
        fps_hist: Histogram::with_buckets(100),
    }));

    let mut last_frame_time = Instant::now();
    let mut next_frame_time = Instant::now() + Duration::from_nanos(16_666_667);
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait_until(next_frame_time);

        match event {
            Event::RedrawRequested(_) => {
                #[cfg(feature = "ui")]
                let next_run = {
                    let _span = tracing::debug_span!("ui").entered();
                    ui.run(&window, {
                        let app = app.clone();
                        let render_stats = render_stats.clone();
                        move |cx| {
                            app.lock().unwrap().ui(UiContext {
                                elapsed: start.elapsed(),
                                dt: last_frame_time.elapsed(),
                                stats: &render_stats.read().unwrap(),
                                egui: cx,
                            })
                        }
                    })
                    .min(Duration::from_nanos(16_666_667))
                };
                #[cfg(not(feature = "ui"))]
                let next_run = Duration::from_nanos(16_666_667);

                let mut app = app.lock().unwrap();
                let frame_start = Instant::now();
                app.render(RenderContext {
                    elapsed: start.elapsed(),
                    dt: last_frame_time.elapsed(),
                    stats: &render_stats.read().unwrap(),
                    control_flow,
                })
                .unwrap();
                #[cfg(feature = "ui")]
                {
                    ui.draw(&window).unwrap();
                }
                gl_surface.swap_buffers(&context).unwrap();
                let frame_time = frame_start.elapsed().as_secs_f32();
                render_stats
                    .write()
                    .unwrap()
                    .add_frame_time(frame_time.recip());
                tracing::debug!(%frame_time);
                next_frame_time = frame_start + next_run;
                last_frame_time = Instant::now();
            }
            Event::WindowEvent { event, .. } => match event {
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
                    app.lock().unwrap().resize(new_size).unwrap();
                    window.request_redraw();
                }
                event => {
                    #[cfg(feature = "ui")]
                    {
                        let response = ui.on_event(&event);
                        if !response.consumed {
                            app.lock().unwrap().interact(event).unwrap();
                        }
                        if response.repaint {
                            window.request_redraw();
                        }
                    }
                }
            },
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => window.request_redraw(),
            _ => {}
        }
    });
}
