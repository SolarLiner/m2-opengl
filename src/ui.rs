use std::time::Duration;

use egui::epaint;
use egui_winit::winit::event_loop::EventLoopWindowTarget;
use eyre::Result;
use violette_low::framebuffer::Framebuffer;

use self::painter::UiImpl;

pub mod painter;

pub struct Ui {
    ctx: egui::Context,
    winit: egui_winit::State,
    painter: painter::UiImpl,
    shapes: Vec<epaint::ClippedShape>,
    tex_deltas: egui::TexturesDelta,
}

impl Ui {
    pub fn new<E>(event_loop: &EventLoopWindowTarget<E>) -> Result<Self> {
        let painter = UiImpl::new()?;

        Ok(Self {
            ctx: Default::default(),
            winit: egui_winit::State::new(event_loop),
            painter,
            shapes: Vec::new(),
            tex_deltas: Default::default(),
        })
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent) -> egui_winit::EventResponse {
        self.winit.on_event(&self.ctx, event)
    }

    pub fn run(&mut self, window: &winit::window::Window, runner: impl FnMut(&egui::Context)) -> Duration {
        let raw_input = self.winit.take_egui_input(window);
        let output = self.ctx.run(raw_input, runner);

        self.winit.handle_platform_output(window, &self.ctx, output.platform_output);
        self.shapes = output.shapes;
        self.tex_deltas.append(output.textures_delta);
        output.repaint_after
    }

    pub fn draw(&mut self, window: &winit::window::Window) -> Result<()> {
        for (id, delta) in &self.tex_deltas.set {
            self.painter.set_texture(*id, delta)?;
        }

        let primitives = self.ctx.tessellate(std::mem::take(&mut self.shapes));
        self.painter.draw(&Framebuffer::backbuffer(), window.inner_size(), self.ctx.pixels_per_point(), &primitives)?;

        for id in self.tex_deltas.free.drain(..) {
            self.painter.delete_texture(id);
        }
        Ok(())
    }
}