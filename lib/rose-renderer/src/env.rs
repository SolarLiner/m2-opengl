use std::{any::Any, fmt, path::Path};
use std::num::NonZeroU32;

use eyre::{Context, Report, Result};
use glam::{vec3, Vec3};

use rose_core::{camera::ViewUniformBuffer, screen_draw::ScreenDraw};
use rose_core::utils::reload_watcher::ReloadWatcher;
use violette::{
    framebuffer::Framebuffer,
    program::{UniformBlockIndex, UniformLocation},
    texture::{SampleMode, Texture, TextureWrap},
};
use violette::texture::Dimension;

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
    u_albedo: UniformLocation,
    u_normal: UniformLocation,
}

impl Environment for SimpleSky {
    fn draw(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        let draw = self.draw.program();
        draw.bind_block(&camera.slice(0..=0), self.u_view, 0)?;
        draw.set_uniform(self.u_horizon_color, self.params.horizon_color)?;
        draw.set_uniform(self.u_ground_color, self.params.ground_color)?;
        draw.set_uniform(self.u_zenith_color, self.params.zenith_color)?;
        draw.set_uniform(self.u_albedo, mat_info.albedo.as_uniform(0)?)?;
        draw.set_uniform(self.u_normal, mat_info.normal_coverage.as_uniform(1)?)?;
        self.draw.draw(frame)?;
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
    pub fn new(params: SimpleSkyParams, reload_watcher: &ReloadWatcher) -> Result<Self> {
        let draw = ScreenDraw::load("screen/env/simple_sky.glsl", reload_watcher)
            .with_context(|| "Loading simple sky background shader")?;
        let program = draw.program();
        let u_view = program.uniform_block("View");
        let u_horizon_color = program.uniform("horizon_color");
        let u_zenith_color = program.uniform("zenith_color");
        let u_ground_color = program.uniform("ground_color");
        let u_albedo = program.uniform("albedo");
        let u_normal = program.uniform("normal_map");
        drop(program);
        Ok(Self {
            params,
            draw,
            u_view,
            u_horizon_color,
            u_zenith_color,
            u_ground_color,
            u_albedo,
            u_normal,
        })
    }
}

#[derive(Debug)]
pub struct EnvironmentMap {
    draw: ScreenDraw,
    irradiance_texture: Texture<[f32; 3]>,
    specular_ibl: Texture<[f32; 3]>,
    map: Texture<[f32; 3]>,
    u_view: UniformBlockIndex,
    u_irradiance: UniformLocation,
    u_sampler: UniformLocation,
    u_albedo: UniformLocation,
    u_normal: UniformLocation,
    u_rough_metal: UniformLocation,
    u_specular: UniformLocation,
}

impl Environment for EnvironmentMap {
    fn draw(
        &mut self,
        frame: &Framebuffer,
        camera: &ViewUniformBuffer,
        mat_info: MaterialInfo,
    ) -> Result<()> {
        {
            let draw = self.draw.program();
            draw.bind_block(&camera.slice(0..=0), self.u_view, 0)?;
            draw.set_uniform(self.u_albedo, mat_info.albedo.as_uniform(0)?)?;
            draw.set_uniform(self.u_normal, mat_info.normal_coverage.as_uniform(1)?)?;
            draw.set_uniform(self.u_rough_metal, mat_info.roughness_metal.as_uniform(2)?)?;
            draw.set_uniform(self.u_sampler, self.map.as_uniform(3)?)?;
            draw.set_uniform(self.u_irradiance, self.irradiance_texture.as_uniform(4)?)?;
            draw.set_uniform(self.u_specular, self.specular_ibl.as_uniform(5)?)?;
        }
        self.draw.draw(frame)?;
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
    pub fn load(filepath: impl AsRef<Path>, reload_watcher: &ReloadWatcher) -> Result<Self> {
        let filepath = filepath.as_ref();
        let map = Texture::load_rgb32f(filepath)?;
        Self::new(map, reload_watcher)
    }

    fn new(
        map: Texture<[f32; 3]>,
        reload_watcher: &ReloadWatcher,
    ) -> Result<EnvironmentMap, Report> {
        let screen_draw = ScreenDraw::load("screen/env/equirectangular.glsl", reload_watcher)?;
        let draw = screen_draw.program();

        let u_view = draw.uniform_block("View");
        let u_sampler = draw.uniform("env_map");
        let u_irradiance = draw.uniform("irradiance_map");
        let u_albedo = draw.uniform("frame_albedo");
        let u_normal = draw.uniform("frame_normal");
        let u_rough_metal = draw.uniform("frame_rough_metal");
        let u_specular = draw.uniform("specular_map");
        drop(draw);

        let irradiance_texture = Self::build_irradiance_texture(
            &map,
            reload_watcher,
            NonZeroU32::new(256).unwrap(),
            NonZeroU32::new(128).unwrap(),
        )?;

        let specular_ibl = Self::build_specular_ibl(
            &map,
            reload_watcher,
            NonZeroU32::new(256).unwrap(),
            NonZeroU32::new(128).unwrap(),
        )?;

        map.wrap_s(TextureWrap::Repeat)?;
        map.wrap_t(TextureWrap::Repeat)?;
        map.filter_min(SampleMode::Linear)?;
        map.filter_mag(SampleMode::Linear)?;
        Ok(Self {
            draw: screen_draw,
            irradiance_texture,
            specular_ibl,
            map,
            u_view,
            u_sampler,
            u_irradiance,
            u_albedo,
            u_normal,
            u_rough_metal,
            u_specular,
        })
    }

    fn build_irradiance_texture(
        map: &Texture<[f32; 3]>,
        reload_watcher: &ReloadWatcher,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<Texture<[f32; 3]>> {
        let irradiance_texture =
            Texture::new(width, height, NonZeroU32::new(1).unwrap(), Dimension::D2);
        irradiance_texture.filter_min(SampleMode::Linear)?;
        irradiance_texture.filter_mag(SampleMode::Linear)?;
        irradiance_texture.reserve_memory()?;

        let irradiance_fbo = Framebuffer::new();
        irradiance_fbo.attach_color(0, irradiance_texture.mipmap(0).unwrap())?;
        irradiance_fbo.assert_complete()?;

        let make_irradiance = ScreenDraw::load("screen/env/irradiance.glsl", reload_watcher)?;
        make_irradiance.program().set_uniform(
            make_irradiance.program().uniform("env_map"),
            map.as_uniform(0)?,
        )?;
        Framebuffer::viewport(0, 0, width.get() as _, height.get() as _);
        make_irradiance.draw(&irradiance_fbo)?;
        Ok(irradiance_texture)
    }

    fn build_specular_ibl(
        map: &Texture<[f32; 3]>,
        reload_watcher: &ReloadWatcher,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<Texture<[f32; 3]>> {
        let specular_ibl = Texture::new(width, height, NonZeroU32::new(1).unwrap(), Dimension::D2);
        specular_ibl.filter_min_mipmap(SampleMode::Linear, SampleMode::Linear)?;
        specular_ibl.filter_mag(SampleMode::Linear)?;
        specular_ibl.reserve_memory()?;
        specular_ibl.generate_mipmaps()?;

        let specibl_fbo = Framebuffer::new();
        let draw = ScreenDraw::load(
            reload_watcher.base_path().join("screen/env/specular_ibl.glsl"),
            reload_watcher,
        )?;
        let u_env_map = draw.program().uniform("env_map");
        let u_roughness = draw.program().uniform("roughness");

        let mipmaps = specular_ibl.num_mipmaps();
        for mip in 0..mipmaps {
            let mipmap = specular_ibl.mipmap(mip).unwrap();
            let (mw, mh) = mipmap.size();
            specibl_fbo.attach_color(0, mipmap)?;
            Framebuffer::viewport(0, 0, mw.get() as _, mh.get() as _);
            let roughness = mip as f32 / (mipmaps as f32 - 1.);
            draw.program().set_uniform(u_roughness, roughness)?;
            draw.program()
                .set_uniform(u_env_map, map.as_uniform(0)?)?;
            draw.draw(&specibl_fbo)?;
        }

        Ok(specular_ibl)
    }
}
