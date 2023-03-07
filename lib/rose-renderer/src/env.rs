use std::{any::Any, fmt};
use std::path::Path;

use eyre::{Context, Result};
use glam::{vec3, Vec3};

use rose_core::camera::ViewUniformBuffer;
use rose_core::screen_draw::ScreenDraw;
use violette::{framebuffer::Framebuffer, program::UniformLocation, texture::Texture};
use violette::base::resource::Resource;
use violette::program::UniformBlockIndex;
use violette::texture::TextureWrap;

pub trait Environment: fmt::Debug + Any {
    fn process_background(&mut self, frame: &Framebuffer, camera: &ViewUniformBuffer)
                          -> Result<()>;

    fn illuminate_scene(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        normal_coverage: &Texture<[f32; 4]>,
    ) -> Result<()>;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug, Copy, Clone)]
pub struct SimpleSkyParams {
    pub horizon_color: Vec3,
    pub zenith_color: Vec3,
    pub ground_color: Vec3,
}

impl Default for SimpleSkyParams {
    fn default() -> Self {
        Self {
            horizon_color: Vec3::ONE,
            zenith_color: vec3(0.1, 0.3, 0.7),
            ground_color: vec3(0.2, 0.15, 0.1),
        }
    }
}

impl SimpleSkyParams {
    #[cfg(feature = "debug-ui")]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("simple-sky-params")
            .num_columns(2)
            .show(ui, |ui| {
                let horizon_label = ui.label("Horizon").id;
                ui.color_edit_button_rgb(self.horizon_color.as_mut())
                    .labelled_by(horizon_label);
                ui.end_row();

                let zenith_label = ui.label("Zenith").id;
                ui.color_edit_button_rgb(self.zenith_color.as_mut())
                    .labelled_by(zenith_label);
                ui.end_row();

                let ground_label = ui.label("Ground").id;
                ui.color_edit_button_rgb(self.ground_color.as_mut())
                    .labelled_by(ground_label);
            });
    }
}

#[derive(Debug)]
pub struct SimpleSky {
    pub params: SimpleSkyParams,
    background_paint: ScreenDraw,
    illuminate_paint: ScreenDraw,
    u_bg_view: UniformBlockIndex,
    u_bg_horizon_color: UniformLocation,
    u_bg_zenith_color: UniformLocation,
    u_bg_ground_color: UniformLocation,
    u_ill_normal: UniformLocation,
    u_ill_horizon_color: UniformLocation,
    u_ill_zenith_color: UniformLocation,
    u_ill_ground_color: UniformLocation,
    u_ill_view: UniformBlockIndex,
}

impl Environment for SimpleSky {
    fn process_background(
        &mut self,
        frame: &Framebuffer,
        view_uniform: &ViewUniformBuffer,
    ) -> Result<()> {
        self.background_paint
            .bind_block(self.u_bg_view, &view_uniform.slice(0..=0))?;
        self.background_paint
            .set_uniform(self.u_bg_horizon_color, self.params.horizon_color)?;
        self.background_paint
            .set_uniform(self.u_bg_ground_color, self.params.ground_color)?;
        self.background_paint
            .set_uniform(self.u_bg_zenith_color, self.params.zenith_color)?;
        self.background_paint.draw(frame)?;
        Ok(())
    }

    fn illuminate_scene(
        &mut self,
        frame: &Framebuffer,
        view_uniform: &ViewUniformBuffer,
        normal_coverage: &Texture<[f32; 4]>,
    ) -> Result<()> {
        self.illuminate_paint
            .bind_block(self.u_ill_view, &view_uniform.slice(0..=0))?;
        self.illuminate_paint
            .set_uniform(self.u_ill_normal, normal_coverage.as_uniform(0)?)?;
        self.illuminate_paint
            .set_uniform(self.u_ill_horizon_color, self.params.horizon_color)?;
        self.illuminate_paint
            .set_uniform(self.u_ill_ground_color, self.params.ground_color)?;
        self.illuminate_paint
            .set_uniform(self.u_ill_zenith_color, self.params.zenith_color)?;
        self.illuminate_paint.draw(frame)?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SimpleSky {
    pub fn new(params: SimpleSkyParams) -> Result<Self> {
        let background_paint = ScreenDraw::load("assets/shaders/simple_sky_bg.frag.glsl")
            .with_context(|| "Loading simple sky background shader")?;
        // let u_bg_view_proj = background_paint.uniform("view_proj").unwrap();
        let u_bg_view = background_paint.uniform_block("view", 0).unwrap();
        let u_bg_horizon_color = background_paint.uniform("horizon_color").unwrap();
        let u_bg_zenith_color = background_paint.uniform("zenith_color").unwrap();
        let u_bg_ground_color = background_paint.uniform("ground_color").unwrap();

        let illuminate_paint = ScreenDraw::load("assets/shaders/simple_sky_illuminate.frag.glsl")
            .with_context(|| "Loading simple sky illuminate shader")?;
        let u_ill_view = illuminate_paint.uniform_block("view", 0).unwrap();
        let u_ill_normal = illuminate_paint.uniform("normal").unwrap();
        let u_ill_horizon_color = illuminate_paint.uniform("horizon_color").unwrap();
        let u_ill_zenith_color = illuminate_paint.uniform("zenith_color").unwrap();
        let u_ill_ground_color = illuminate_paint.uniform("ground_color").unwrap();

        Ok(Self {
            params,
            background_paint,
            illuminate_paint,
            u_bg_view,
            u_bg_horizon_color,
            u_bg_zenith_color,
            u_bg_ground_color,
            u_ill_view,
            u_ill_normal,
            u_ill_horizon_color,
            u_ill_ground_color,
            u_ill_zenith_color,
        })
    }
}

#[derive(Debug)]
pub struct EnvironmentMap {
    bg_draw: ScreenDraw,
    illum_draw: ScreenDraw,
    map: Texture<[f32; 3]>,
    u_bg_view: UniformBlockIndex,
    u_bg_sampler: UniformLocation,
    u_ill_view: UniformBlockIndex,
    u_ill_sampler: UniformLocation,
    u_ill_normal: UniformLocation,
}

impl Environment for EnvironmentMap {
    fn process_background(
        &mut self,
        frame: &Framebuffer,
        view_uniform: &ViewUniformBuffer,
    ) -> Result<()> {
        self.bg_draw
            .bind_block(self.u_bg_view, &view_uniform.slice(0..=0))?;
        self.bg_draw
            .set_uniform(self.u_bg_sampler, self.map.as_uniform(0)?)?;
        self.bg_draw.draw(frame)?;
        self.map.unbind();
        Ok(())
    }

    fn illuminate_scene(
        &mut self,
        frame: &Framebuffer,
        view_uniform: &ViewUniformBuffer,
        normal_coverage: &Texture<[f32; 4]>,
    ) -> Result<()> {
        self.illum_draw.bind_block(self.u_ill_view, &view_uniform.slice(0..=0))?;
        self.illum_draw.set_uniform(self.u_ill_sampler, self.map.as_uniform(0)?)?;
        self.illum_draw.set_uniform(self.u_ill_normal, normal_coverage.as_uniform(1)?)?;
        self.illum_draw.draw(frame)?;
        self.map.unbind();
        normal_coverage.unbind();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EnvironmentMap {
    pub fn new(texture: impl AsRef<Path>) -> Result<Self> {
        let bg_draw = ScreenDraw::load("assets/shaders/env/equirectangular.bg.glsl")?;
        let u_bg_view = bg_draw.uniform_block("View", 0).unwrap();
        let u_bg_sampler = bg_draw.uniform("env_map").unwrap();
        let illum_draw = ScreenDraw::load("assets/shaders/env/equirectangular.illum.glsl")?;
        let u_ill_view = illum_draw.uniform_block("View", 0).unwrap();
        let u_ill_sampler = illum_draw.uniform("env_map").unwrap();
        let u_ill_normal = illum_draw.uniform("normal_map").unwrap();
        let map = Texture::load_rgb32f(texture)?;
        map.wrap_s(TextureWrap::Repeat)?;
        map.wrap_t(TextureWrap::Repeat)?;
        map.generate_mipmaps()?;
        Ok(Self {
            bg_draw,
            illum_draw,
            map,
            u_bg_view,
            u_bg_sampler,
            u_ill_view,
            u_ill_sampler,
            u_ill_normal,
        })
    }
}
