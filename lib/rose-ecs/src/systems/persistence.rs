use std::{any::type_name, collections::HashMap};
use std::any::TypeId;

use assets_manager::{AnyCache, Compound, Handle};
use eyre::Result;
use hecs::{
    Component,
    EntityBuilder, EntityRef, serialize::row::{self, DeserializeContext, SerializeContext}, World,
};
use serde::{
    de::{self, IntoDeserializer, MapAccess},
    Deserialize,
    Deserializer, ser::{self, SerializeMap}, Serialize, Serializer,
};
use serde::ser::SerializeSeq;

use rose_core::utils::thread_guard::ThreadGuard;

pub trait SerializableComponent:
Component + serde::Serialize + serde::Deserialize<'static>
{}

impl<C: Component + serde::Serialize + serde::Deserialize<'static>> SerializableComponent for C {}

fn serialize<C: SerializableComponent>(
    entity: &EntityRef<'_>,
) -> Result<Option<serde_json::Value>> {
    if let Some(cmp) = entity.get::<&C>() {
        Ok(Some(serde_json::to_value(&*cmp)?))
        // Ok(Some(cmp.serialize(toml_edit::ser::ValueSerializer::new())?))
    } else {
        Ok(None)
    }
}

fn deserialize<C: SerializableComponent>(
    builder: &mut EntityBuilder,
    value: serde_json::Value,
) -> Result<()> {
    let cmp = C::deserialize(value.into_deserializer())?;
    builder.add(cmp);
    Ok(())
}

#[derive(Copy, Clone)]
struct DynPersistence {
    name: &'static str,
    serialize: &'static dyn Fn(&EntityRef<'_>) -> Result<Option<serde_json::Value>>,
    deserialize: &'static dyn Fn(&mut EntityBuilder, serde_json::Value) -> Result<()>,
}

impl DynPersistence {
    fn new<C: SerializableComponent>() -> Self {
        Self {
            name: type_name::<C>(),
            serialize: &serialize::<C>,
            deserialize: &deserialize::<C>,
        }
    }
}

fn load_asset<A: Compound>(
    cache: AnyCache<'static>,
    entity: &mut EntityBuilder,
    id: &str,
) -> Result<()> {
    entity.add(cache.load::<A>(id)?);
    Ok(())
}

fn get_id<A: Compound>(entity: &EntityRef<'_>) -> Option<String> {
    entity
        .get::<&Handle<'static, A>>()
        .map(|r| r.id().to_string())
}

struct DynAsset {
    name: &'static str,
    load: &'static dyn Fn(AnyCache<'static>, &mut EntityBuilder, &str) -> Result<()>,
    get_id: &'static dyn Fn(&EntityRef<'_>) -> Option<String>,
}

impl DynAsset {
    pub fn new<A: Compound>() -> Self {
        Self {
            name: type_name::<A>(),
            load: &load_asset::<A>,
            get_id: &get_id::<A>,
        }
    }
}

pub struct PersistenceSystem {
    asset_cache: Option<ThreadGuard<AnyCache<'static>>>,
    registry: HashMap<TypeId, ThreadGuard<DynPersistence>>,
    asset_types: HashMap<TypeId, ThreadGuard<DynAsset>>,
    type_map: HashMap<&'static str, TypeId>,
}

impl PersistenceSystem {
    pub fn new() -> Self {
        Self {
            asset_cache: None,
            registry: HashMap::new(),
            asset_types: HashMap::new(),
            type_map: HashMap::new(),
        }
    }

    pub fn register_component<C: SerializableComponent>(&mut self) -> &mut Self {
        let type_id = TypeId::of::<C>();
        let dyn_persistence = DynPersistence::new::<C>();
        self.type_map.insert(dyn_persistence.name, type_id);
        self.registry.insert(type_id, ThreadGuard::new(dyn_persistence));
        self
    }

    pub fn register_asset<A: Compound>(&mut self) -> &mut Self {
        let type_id = TypeId::of::<A>();
        let dyn_asset = DynAsset::new::<A>();
        self.type_map.insert(dyn_asset.name, type_id);
        self.asset_types.insert(type_id, ThreadGuard::new(dyn_asset));
        self
    }

    pub fn deserialize_world<'de, D: Deserializer<'de>>(
        &mut self,
        cache: AnyCache<'static>,
        de: D,
    ) -> Result<World>
        where
            D::Error: 'static + Send + Sync,
    {
        self.asset_cache.replace(ThreadGuard::new(cache));
        Ok(row::deserialize(self, de)?)
    }

    pub fn serialize_world<S: Serializer>(
        &mut self,
        cache: AnyCache<'static>,
        ser: S,
        world: &World,
    ) -> Result<()>
        where
            S::Error: 'static + Send + Sync,
    {
        self.asset_cache.replace(ThreadGuard::new(cache));
        row::serialize(world, self, ser)?;
        Ok(())
    }
}

impl DeserializeContext for PersistenceSystem {
    fn deserialize_entity<'de, M>(
        &mut self,
        mut map: M,
        entity: &mut EntityBuilder,
    ) -> std::result::Result<(), M::Error>
        where
            M: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            let Some(type_id) = self.type_map.get(&*key) else { continue; };
            if let Some(pers) = self.registry.get(type_id) {
                let value = map.next_value::<serde_json::Value>()?;
                (pers.deserialize)(entity, value).map_err(de::Error::custom)?;
            } else if let Some(asset) = self.asset_types.get(type_id) {
                (asset.load)(*self.asset_cache.unwrap(), entity, &map.next_value::<String>()?)
                    .map_err(de::Error::custom)?;
            }
        }
        Ok(())
    }
}

impl SerializeContext for PersistenceSystem {
    fn serialize_entity<S>(
        &mut self,
        entity: EntityRef<'_>,
        mut map: S,
    ) -> std::result::Result<S::Ok, S::Error>
        where
            S: SerializeMap,
    {
        for pers in self.registry.values() {
            let Some(value) = (pers.serialize)(&entity).map_err(ser::Error::custom)? else { continue; };
            map.serialize_entry::<String, serde_json::Value>(&pers.name.to_string(), &value)?;
        }
        for asset in self.asset_types.values() {
            let Some(id) = (asset.get_id)(&entity) else { continue; };
            map.serialize_entry::<String, String>(&asset.name.to_string(), &id)?;
        }
        map.end()
    }
}
