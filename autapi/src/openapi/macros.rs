macro_rules! define_openapi_spec_object {
    (
        $([$($feature:ident),*]:)?
        $(#[$attr:meta])*
        pub struct $name:ident {
            $(
            $(#[$field_attr:meta])*
            pub $field_name:ident: $field_ty:ident $(<$($field_gen:ty),*>)?,
            )*
        }
    ) => {
        #[serde_with::skip_serializing_none]
        #[derive(PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[non_exhaustive]
        $(#[$attr])*
        pub struct $name {
            $(
            $(#[$field_attr])*
            pub $field_name: $field_ty $(<$($field_gen,)*>)?,
            )*
        }
        impl $name {
            $(
                $crate::openapi::macros::define_openapi_spec_object!(@with_fn $field_name $field_ty $(<$($field_gen),*>)?);
                $crate::openapi::macros::define_openapi_spec_object!(@with_entry $field_name $field_ty $(<$($field_gen),*>)?);
            )*
            $crate::openapi::macros::define_openapi_spec_object!(@impl_features [$($($feature),*)?] {
                $(#[$attr])*
                pub struct $name {
                    $(
                    $(#[$field_attr])*
                    pub $field_name: $field_ty $(<$($field_gen,)*>)?,
                    )*
                }
            });
        }
    };
    (@with_entry $field_name:ident Map<$key:ty, $value:ty>) => {
        pastey::paste! {
            pub fn [< with_ $field_name _entry>](mut self, key: impl Into<$key>, value: impl Into<$value>) -> Self {
                self.$field_name.insert(key.into(), value.into());
                self
            }
        }
    };
    (@with_entry $field_name:ident Vec<$value:ty>) => {
        pastey::paste! {
            pub fn [< with_ $field_name _entry>](mut self, value: impl Into<$value>) -> Self {
                self.$field_name.push(value.into());
                self
            }
        }
    };
    (@with_entry $field_name:ident Extensions) => {
        pastey::paste! {
            pub fn with_extension(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
                self.insert_extension(key, value);
                self
            }
            pub fn insert_extension(&mut self, key:  impl Into<String>, value: impl Into<Value>) -> Option<serde_json::Value> {
                self.$field_name.insert(key.into(), value.into())
            }
        }
    };
    (@with_entry $field_name:ident $field_ty:ty) => {};
    (@with_fn $field_name:ident Option<$field_ty:ty>) => {
        pastey::paste! {
            pub fn [< with_ $field_name>](mut self, value: impl Into<$field_ty>) -> Self {
                self.$field_name = Some(value.into());
                self
            }
            pub fn [< with_maybe_ $field_name>](mut self, value: impl Into<Option<$field_ty>>) -> Self {
                self.$field_name = value.into();
                self
            }
        }
    };
    (@with_fn $field_name:ident $field_ty:ty) => {
        pastey::paste! {
            pub fn [< with_ $field_name>](mut self, value: impl Into<$field_ty>) -> Self {
                self.$field_name = value.into();
                self
            }
        }
    };
    (@impl_features [$($feature:ident),*] $def:tt) => {
        $($crate::openapi::macros::define_openapi_spec_object!(@impl_feature $feature $def);)*
    };
    (@impl_feature override_with {
        $(#[$attr:meta])*
        pub struct $name:ident {
            $(
            $(#[$field_attr:meta])*
            pub $field_name:ident: $field_ty:ident $(<$($field_gen:ty,)*>)?,
            )*
        }
    }
    ) => {
        pub fn override_with(&mut self, other: Self) {
            $(
                if !$crate::private::is_default(&other.$field_name) {
                    self.$field_name = other.$field_name;
                }
            )*
        }
    }
}

pub(crate) use define_openapi_spec_object;
