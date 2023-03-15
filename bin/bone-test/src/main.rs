use std::{path::PathBuf, rc::Rc};

use rose::{prelude::*, renderer::Renderer};

struct BoneTestApp {
    renderer: ThreadGuard<Renderer>,
    mesh: ThreadGuard<Rc<rose::renderer::Mesh>>,
    material: ThreadGuard<Rc<MaterialInstance>>,
}

impl Application for BoneTestApp {
    fn new(size: PhysicalSize<f32>, _scale_factor: f64) -> Result<Self> {
        let sizeu = UVec2::from_array(size.cast::<u32>().into());
        let base_dir = std::env::var("CARGO_PROJECT_DIR")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap();
        let bones_ix = uvec4(0, 1, 2, u32::MAX);
        let mut mesh = MeshBuilder::new(Vertex::new).uv_sphere(1., 12, 24);
        for vert in mesh.vertices.iter_mut() {
            vert.bones_ix = bones_ix.as_ivec4();
            vert.bones_weights = vec4(0., vert.position.y * 0.5 + 0.5, 0.5 - 0.5 * vert.position.y, 0.);
        }
        let mut mesh: rose::renderer::Mesh = mesh.upload()?.into();
        let root_bone = Bone::new(Mat4::IDENTITY);
        root_bone.add_child(Bone::new(Mat4::from_translation(Vec3::Y)));
        root_bone.add_child(Bone::new(Mat4::from_translation(Vec3::NEG_Y)));
        mesh.root_bone = Some(root_bone);
        let mut renderer = Renderer::new(sizeu, base_dir)?;
        renderer.set_environment(|w| SimpleSky::new(SimpleSkyParams::default(), w).unwrap());
        renderer.add_lights([
            rose::core::light::Light::Ambient {
                color: Vec3::splat(0.3),
            },
            rose::core::light::Light::Directional {
                dir: Vec3::ONE.normalize(),
                color: Vec3::ONE,
            },
        ])?;
        let mut material = MaterialInstance::create(None, None, None, None)?;
        material.update_uniforms(|u| {
            u.rough_metal_factor = vec2(0.5, 0.);
        })?;
        Ok(Self {
            renderer: ThreadGuard::new(renderer),
            mesh: ThreadGuard::new(Rc::new(mesh)),
            material: ThreadGuard::new(Rc::new(material)),
        })
    }

    fn resize(&mut self, _size: PhysicalSize<u32>, _scale_factor: f64) -> Result<()> {
        self.renderer.resize(UVec2::from_array(_size.into()))?;
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        // Update
        let root_bone = self.mesh.root_bone.as_ref().unwrap();
        let children = root_bone.children.borrow();
        let bone_l = &children[0];
        let bone_r = &children[1];
        let (sin, cos) = ctx.elapsed.as_secs_f32().sin_cos();
        // root_bone.update_transform(|_| Transform::translation(Vec3::Y * sin).matrix());
        bone_l.update_transform(|_| Transform::translation(vec3(sin, 1., cos)).matrix());
        bone_r.update_transform(|_| Transform::translation(vec3(cos, -1., sin)).matrix());

        // Render
        let size = ctx.window.inner_size().cast();
        let camera = Camera {
            transform: Transform::translation(vec3(-3., 3., 3.)).looking_at(Vec3::ZERO),
            projection: Projection {
                width: size.width,
                height: size.height,
                ..Default::default()
            },
        };
        self.renderer.begin_render(&camera)?;
        self.renderer.submit_mesh_standard(
            Rc::clone(&self.material),
            Rc::clone(&self.mesh).transformed(Transform::default()),
        );
        self.renderer.flush(ctx.dt, Vec3::ZERO)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    run::<BoneTestApp>("Bone deformation")
}
