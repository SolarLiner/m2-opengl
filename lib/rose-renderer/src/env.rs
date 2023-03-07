use std::{
    any::Any,
    fmt,
    path::Path,
};

use eyre::{Context, Result};
use glam::{vec3, Vec3};

use rose_core::{
    camera::ViewUniformBuffer,
    screen_draw::ScreenDraw,
};
use violette::{
    base::resource::Resource,
    framebuffer::Framebuffer,
    program::{
        UniformBlockIndex,
        UniformLocation,
    },
    texture::{
        SampleMode,
        Texture,
        TextureWrap,
    },
};

pub struct MaterialInfo<'a> {
    pub position: &'a Texture<[f32; 3]>,
    pub albedo: &'a Texture<[f32; 3]>,
    pub normal_coverage: &'a Texture<[f32; 4]>,
    pub roughness_metal: &'a Texture<[f32; 2]>,
}

pub trait Environment: fmt::Debug + Any {
    fn process_background(&mut self, frame: &Framebuffer, camera: &ViewUniformBuffer, mat_info: MaterialInfo)
                          -> Result<()>;

    fn illuminate_scene(
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
    u_is_illumination: UniformLocation,
    u_normal: UniformLocation,
}

impl Environment for SimpleSky {
    fn process_background(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw(frame, camera, mat_info, false)
    }

    fn illuminate_scene(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.draw(frame, camera, mat_info, true)
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
        // let u_bg_view_proj = background_paint.uniform("view_proj").unwrap();
        let u_view = draw.uniform_block("View", 0).unwrap();
        let u_horizon_color = draw.uniform("horizon_color").unwrap();
        let u_zenith_color = draw.uniform("zenith_color").unwrap();
        let u_ground_color = draw.uniform("ground_color").unwrap();
        let u_is_illumination = draw.uniform("is_illumination").unwrap();
        let u_normal = draw.uniform("normal_map").unwrap();

        Ok(Self {
            params,
            draw,
            u_view,
            u_horizon_color,
            u_zenith_color,
            u_ground_color,
            u_is_illumination,
            u_normal,
        })
    }

    fn draw(&self, frame: &Framebuffer, camera: &ViewUniformBuffer, mat_info: MaterialInfo, is_illumination: bool) -> Result<()> {
        self.draw.bind_block(self.u_view, &camera.slice(0..=0))?;
        self.draw.set_uniform(self.u_horizon_color, self.params.horizon_color)?;
        self.draw.set_uniform(self.u_ground_color, self.params.ground_color)?;
        self.draw.set_uniform(self.u_zenith_color, self.params.zenith_color)?;
        self.draw.set_uniform(self.u_normal, mat_info.normal_coverage.as_uniform(0)?)?;
        self.draw.set_uniform(self.u_is_illumination, is_illumination)?;
        self.draw.draw(frame)?;
        Ok(())
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
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.bg_draw
            .bind_block(self.u_bg_view, &camera.slice(0..=0))?;
        self.bg_draw
            .set_uniform(self.u_bg_sampler, self.map.as_uniform(0)?)?;
        self.bg_draw.draw(frame)?;
        self.map.unbind();
        Ok(())
    }

    fn illuminate_scene(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        self.illum_draw.bind_block(self.u_ill_view, &camera.slice(0..=0))?;
        self.illum_draw.set_uniform(self.u_ill_sampler, self.map.as_uniform(0)?)?;
        self.illum_draw.set_uniform(self.u_ill_normal, mat_info.normal_coverage.as_uniform(1)?)?;
        self.illum_draw.draw(frame)?;
        self.map.unbind();
        mat_info.normal_coverage.unbind();
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
        let illum_draw = ScreenDraw::load("../../../assets/shaders/env/equirectangular.glsl")?;
        let u_ill_view = illum_draw.uniform_block("View", 0).unwrap();
        let u_ill_sampler = illum_draw.uniform("env_map").unwrap();
        let u_ill_normal = illum_draw.uniform("normal_map").unwrap();
        let map = Texture::load_rgb32f(texture)?;
        map.wrap_s(TextureWrap::Repeat)?;
        map.wrap_t(TextureWrap::Repeat)?;
        map.filter_min(SampleMode::Linear)?;
        map.filter_mag(SampleMode::Linear)?;
        map.filter_min_mipmap(SampleMode::Linear, SampleMode::Linear)?;
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
