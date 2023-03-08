use std::{any::Any, fmt, path::Path};

use eyre::{Context, Result};
use glam::{vec3, Vec3};

use rose_core::{camera::ViewUniformBuffer, screen_draw::ScreenDraw};
use violette::{
    framebuffer::Framebuffer,
    program::{UniformBlockIndex, UniformLocation},
    texture::{SampleMode, Texture, TextureWrap},
};

#[derive(Debug, Copy, Clone)]
pub struct MaterialInfo<'a> {
    pub position: &'a Texture<[f32; 3]>,
    pub albedo: &'a Texture<[f32; 3]>,
    pub normal_coverage: &'a Texture<[f32; 4]>,
    pub roughness_metal: &'a Texture<[f32; 2]>,
}

pub trait Environment: fmt::Debug + Any {
    fn draw(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
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
    draw: ScreenDraw,
    u_view: UniformBlockIndex,
    u_horizon_color: UniformLocation,
    u_zenith_color: UniformLocation,
    u_ground_color: UniformLocation,
    u_normal: UniformLocation,
}

impl Environment for SimpleSky {
    fn draw(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw_impl(frame, camera, mat_info)?;
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
        let draw = ScreenDraw::load("assets/shaders/env/simple_sky.glsl")
            .with_context(|| "Loading simple sky background shader")?;
        let u_view = draw.uniform_block("View");
        let u_horizon_color = draw.uniform("horizon_color");
        let u_zenith_color = draw.uniform("zenith_color");
        let u_ground_color = draw.uniform("ground_color");
        let u_normal = draw.uniform("normal_map");

        Ok(Self {
            params,
            draw,
            u_view,
            u_horizon_color,
            u_zenith_color,
            u_ground_color,
            u_normal,
        })
    }

    fn draw_impl(
        &self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw.bind_block(&camera.slice(0..=0), self.u_view, 0)?;
        self.draw
            .set_uniform(self.u_horizon_color, self.params.horizon_color)?;
        self.draw
            .set_uniform(self.u_ground_color, self.params.ground_color)?;
        self.draw
            .set_uniform(self.u_zenith_color, self.params.zenith_color)?;
        self.draw
            .set_uniform(self.u_normal, mat_info.normal_coverage.as_uniform(0)?)?;
        self.draw.draw(frame)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EnvironmentMap {
    draw: ScreenDraw,
    map: Texture<[f32; 3]>,
    u_view: UniformBlockIndex,
    u_sampler: UniformLocation,
    u_albedo: UniformLocation,
    u_normal: UniformLocation,
    u_rough_metal: UniformLocation,
}

impl Environment for EnvironmentMap {
    fn draw(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw_impl(frame, camera, mat_info)?;
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
        let draw = ScreenDraw::load("assets/shaders/env/equirectangular.glsl")?;
        let u_view = draw.uniform_block("View");
        let u_sampler = draw.uniform("env_map");
        let u_albedo = draw.uniform("frame_albedo");
        let u_normal = draw.uniform("frame_normal");
        let u_rough_metal = draw.uniform("frame_rough_metal");

        let map = Texture::load_rgb32f(texture)?;
        map.wrap_s(TextureWrap::Repeat)?;
        map.wrap_t(TextureWrap::Repeat)?;
        map.filter_min(SampleMode::Linear)?;
        map.filter_mag(SampleMode::Linear)?;
        map.filter_min_mipmap(SampleMode::Linear, SampleMode::Linear)?;
        map.generate_mipmaps()?;
        Ok(Self {
            draw,
            map,
            u_view,
            u_sampler,
            u_albedo,
            u_normal,
            u_rough_metal,
        })
    }

    fn draw_impl(
        &self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw.bind_block(&camera.slice(0..=0), self.u_view, 0)?;
        self.draw
            .set_uniform(self.u_albedo, mat_info.albedo.as_uniform(0)?)?;
        self.draw
            .set_uniform(self.u_normal, mat_info.normal_coverage.as_uniform(1)?)?;
        self.draw
            .set_uniform(self.u_rough_metal, mat_info.roughness_metal.as_uniform(2)?)?;
        self.draw
            .set_uniform(self.u_sampler, self.map.as_uniform(3)?)?;
        self.draw.draw(frame)?;
        Ok(())
    }
}
