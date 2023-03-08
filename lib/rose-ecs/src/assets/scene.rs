use std::ops;

use assets_manager::{
    AnyCache,
    Asset,
    BoxedError,
    Compound,
    loader::TomlLoader,
    SharedString,
};
use serde::{Deserialize, Serialize};

use rose_core::transform::{Transformed as TransformedCore, TransformExt};

use crate::{
    assets::object::{ObjectDesc, TransformDesc},
    components::{CameraParams, Light},
};

#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize)]
pub struct Transformed<T> {
    #[serde(default)]
    pub transform: TransformDesc,
    #[serde(flatten)]
    pub value: T,
}

impl<T> ops::Deref for Transformed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Named<T> {
    pub name: SharedString,
    #[serde(flatten)]
    pub value: T,
}

impl<T> ops::Deref for Named<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NamedObject {
    pub object: SharedString,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SceneDesc {
    pub camera: Transformed<CameraParams>,
    pub lights: Vec<Named<Transformed<Light>>>,
    pub objects: Vec<Named<Transformed<NamedObject>>>,
}

impl Default for SceneDesc {
    fn default() -> Self {
        Self {
            camera: Transformed::default(),
            lights: vec![],
            objects: vec![],
        }
    }
}

impl Asset for SceneDesc {
    const EXTENSIONS: &'static [&'static str] = &["scene", "toml"];
    type Loader = TomlLoader;
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub camera: TransformedCore<CameraParams>,
    pub lights: Vec<Named<TransformedCore<Light>>>,
    pub objects: Vec<Named<TransformedCore<ObjectDesc>>>,
}

impl Compound for Scene {
    fn load(cache: AnyCache, id: &SharedString) -> Result<Self, BoxedError> {
        tracing::debug!(message="Loading scene", %id);
        let desc = cache.load_owned::<SceneDesc>(id);
        let desc = match desc {
            Ok(desc) => desc,
            Err(err) => {
                tracing::error!(message="Cannot load scene", %id, %err);
                return Err(Box::new(err));
            }
        };
        tracing::debug!(message = "Scene description", ?desc);
        let camera = desc.camera.value.transformed(desc.camera.transform.into());
        let lights = desc
            .lights
            .into_iter()
            .map(|light| Named {
                name: light.name,
                value: light.value.value.transformed(light.value.transform.into()),
            })
            .collect();
        let objects = desc
            .objects
            .into_iter()
            .map(|obj| {
                cache
                    .load_owned(&obj.value.object)
                    .map(|asset: ObjectDesc| Named {
                        name: obj.value.value.object,
                        value: asset.transformed(obj.value.transform.into()),
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            camera,
            lights,
            objects,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::SceneDesc;

    #[test]
    fn test_scene_desc_parse_empty() {
        let input = "";
        let desc: SceneDesc = toml::de::from_str(input).unwrap();
        println!("{:#?}", desc);
    }

    #[test]
    fn test_scene_desc_parse_object() {
        let input = r#"
        [[objects]]
        object = "objects.suzanne"
        [objects.transform]
        translation = [2, 0, 0]
        "#;
        let desc: SceneDesc = toml::de::from_str(input).unwrap();
        println!("{:#?}", desc);
    }

    #[test]
    fn test_scene_desc_parse_light() {
        let input = r#"
        [[lights]]
        kind = "Ambient"
        color = [0.1, 0.1, 0.1]
        "#;
        let desc: SceneDesc = toml::de::from_str(input).unwrap();
        println!("{:#?}", desc);
    }

    #[test]
    fn test_load_scene_desc_file() {
        let input = include_str!("fixtures/scene_example.toml");
        let desc: SceneDesc = toml::de::from_str(input).unwrap();
        println!("{:#?}", desc);
    }
}
