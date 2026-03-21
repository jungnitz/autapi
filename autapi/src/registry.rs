use std::{
    any::TypeId,
    collections::{HashMap, btree_map},
    panic,
};

use crate::{
    openapi::{Components, MaybeRef, OpenApi, Ref, Schema},
    schema::ToSchema,
};

/// Keeps track of components during schema generation.
#[derive(Default)]
pub struct Registry {
    openapi: OpenApi,
    schema_by_type_id: HashMap<TypeId, String>,
}

impl Registry {
    pub fn register_schema<T: ToSchema + ?Sized>(&mut self) -> Ref {
        let type_id = typeid::of::<T>();
        if let Some(name) = self.schema_by_type_id.get(&type_id) {
            return Ref::ref_schema_component(name.as_str());
        }
        let name = T::name();
        self.schema_by_type_id.insert(type_id, name.to_string());
        let schema = T::schema(self);
        match self.components_mut().schemas.entry(name.to_string()) {
            btree_map::Entry::Occupied(entry) => {
                if entry.get() != &schema {
                    panic!("colliding schemas for name {name}")
                }
            }
            btree_map::Entry::Vacant(entry) => {
                entry.insert(schema);
            }
        }
        Ref::ref_schema_component(&name)
    }
    pub fn resolve_schema<'a>(&'a self, schema: &'a MaybeRef<Schema>) -> Option<&'a Schema> {
        self.components().resolve_schema(schema)
    }
    pub fn schema_by_name(&self, name: &str) -> Option<&MaybeRef<Schema>> {
        self.components().schemas.get(name)
    }
    pub fn components(&self) -> &Components {
        &self.openapi.components
    }
    pub fn components_mut(&mut self) -> &mut Components {
        &mut self.openapi.components
    }
    pub fn openapi_mut(&mut self) -> &mut OpenApi {
        &mut self.openapi
    }
    pub fn into_openapi(self) -> OpenApi {
        self.openapi
    }
}
