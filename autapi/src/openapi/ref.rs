use serde::{Deserialize, Serialize};

use crate::openapi::macros::define_openapi_spec_object;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaybeRef<T> {
    Ref(Ref),
    T(T),
}

impl<T> MaybeRef<T> {
    pub fn into_ref(self) -> Option<Ref> {
        match self {
            Self::Ref(r) => Some(r),
            _ => None,
        }
    }
    pub fn as_ref(&self) -> Option<&Ref> {
        match self {
            Self::Ref(r) => Some(r),
            _ => None,
        }
    }
    pub fn as_ref_mut(&mut self) -> Option<&mut Ref> {
        match self {
            Self::Ref(r) => Some(r),
            _ => None,
        }
    }

    pub fn into_inner(self) -> Option<T> {
        match self {
            Self::T(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_inner(&self) -> Option<&T> {
        match self {
            Self::T(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_inner_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::T(t) => Some(t),
            _ => None,
        }
    }
}

impl<T> From<T> for MaybeRef<T> {
    fn from(value: T) -> Self {
        Self::T(value)
    }
}

define_openapi_spec_object! {
    pub struct Ref {
        #[serde(rename = "$ref")]
        pub target: String,
        pub summary: Option<String>,
        pub description: Option<String>,
    }
}

impl Ref {
    const PREFIX_SCHEMA_COMPONENTS_REF: &str = "#/components/schemas/";

    pub fn new(target: String) -> Ref {
        Self {
            target,
            description: Default::default(),
            summary: Default::default(),
        }
    }
    pub fn ref_schema_component(name: &str) -> Ref {
        Self::new(format!("{}{}", Self::PREFIX_SCHEMA_COMPONENTS_REF, name))
    }

    pub fn get_components_schema_name(&self) -> Option<&str> {
        self.target.strip_prefix(Self::PREFIX_SCHEMA_COMPONENTS_REF)
    }
}
