mod macros;
mod r#ref;
mod schema;

use std::{
    collections::{BTreeMap, btree_map},
    ops::{Deref, DerefMut},
};

use http::Method;
use serde::{Deserialize, Serialize, ser::SerializeMap};
use serde_json::Value;

pub use self::{r#ref::*, schema::*};
use crate::private::{is_default, merge_maps};

pub type Map<K, V> = BTreeMap<K, V>;
pub type Entry<'a, K, V> = btree_map::Entry<'a, K, V>;

macros::define_openapi_spec_object! {
    pub struct OpenApi {
        pub openapi: String,
        pub info: Info,
        pub json_schema_dialect: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub servers: Vec<Server>,
        #[serde(default, skip_serializing_if = "is_default")]
        pub paths: Paths,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub webhooks: Map<String, PathItem>,
        #[serde(default, skip_serializing_if = "is_default")]
        pub components: Components,
        pub security: Option<Vec<SecurityRequirement>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub tags: Vec<Tag>,
        pub external_docs: Option<ExternalDocs>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl OpenApi {
    pub const DEFAULT_VERSION: &str = "3.1.0";

    pub fn new(version: String, info: Info) -> Self {
        Self {
            openapi: version,
            info,
            json_schema_dialect: Default::default(),
            servers: Default::default(),
            paths: Default::default(),
            webhooks: Default::default(),
            components: Default::default(),
            security: Default::default(),
            tags: Default::default(),
            external_docs: Default::default(),
            extensions: Default::default(),
        }
    }

    pub fn operations_mut(&mut self) -> impl Iterator<Item = &mut Operation> {
        self.paths
            .paths
            .values_mut()
            .flat_map(|item| item.operations_mut())
    }
}

impl Default for OpenApi {
    fn default() -> Self {
        Self::new(Self::DEFAULT_VERSION.to_owned(), Info::default())
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Info {
        pub title: String,
        pub summary: Option<String>,
        pub description: Option<String>,
        pub terms_of_service: Option<String>,
        pub contact: Option<Contact>,
        pub license: Option<License>,
        pub version: String,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Info {
    pub fn new(title: String) -> Self {
        Self::default().with_title(title)
    }
}

#[macro_export]
macro_rules! info_from_env {
    () => {
        $crate::openapi::Info::default()
            .with_title(env!("CARGO_PKG_NAME"))
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_maybe_description(option_env!("CARGO_PKG_DESCRIPTION").map(ToOwned::to_owned))
            .with_maybe_license(option_env!("CARGO_PKG_LICENSE").map(|license| {
                $crate::openapi::License::new(license.to_owned())
                    .with_identifier(license.to_owned())
            }))
    };
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Contact {
        pub name: Option<String>,
        pub url: Option<String>,
        pub email: Option<String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    pub struct License {
        pub name: String,
        pub identifier: Option<String>,
        pub url: Option<String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl License {
    pub fn new(name: String) -> Self {
        Self {
            name,
            identifier: Default::default(),
            url: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    pub struct Server {
        pub url: String,
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub variables: Map<String, ServerVariable>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Server {
    pub fn new(url: String) -> Self {
        Server {
            url,
            description: Default::default(),
            variables: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    pub struct ServerVariable {
        pub r#enum: Vec<String>,
        pub default: String,
        pub description: Option<String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl ServerVariable {
    pub fn new(r#enum: Vec<String>, default: String) -> Self {
        Self {
            r#enum,
            default,
            description: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Components {
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub schemas: Map<String, MaybeRef<Schema>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub responses: Map<String, MaybeRef<Response>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub parameters: Map<String, MaybeRef<Parameter>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub examples: Map<String, MaybeRef<Parameter>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub request_bodies: Map<String, MaybeRef<RequestBody>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub headers: Map<String, MaybeRef<Header>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub security_schemes: Map<String, MaybeRef<SecurityScheme>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub links: Map<String, MaybeRef<Link>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub callbacks: Map<String, MaybeRef<Callback>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub path_items: Map<String, PathItem>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Components {
    pub fn resolve_schema<'a>(&'a self, mut schema: &'a MaybeRef<Schema>) -> Option<&'a Schema> {
        loop {
            match schema {
                MaybeRef::T(schema) => break Some(schema),
                MaybeRef::Ref(r) => {
                    let name = r.get_components_schema_name()?;
                    schema = self.schemas.get(name)?;
                }
            }
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Paths {
        #[serde(flatten)]
        pub paths: Map<String, PathItem>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct PathItem {
        #[serde(rename = "$ref")]
        pub r#ref: Option<String>,
        pub summary: Option<String>,
        pub description: Option<String>,
        pub get: Option<Operation>,
        pub put: Option<Operation>,
        pub post: Option<Operation>,
        pub delete: Option<Operation>,
        pub options: Option<Operation>,
        pub head: Option<Operation>,
        pub patch: Option<Operation>,
        pub trace: Option<Operation>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub servers: Vec<Server>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub parameters: Vec<MaybeRef<Parameter>>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl PathItem {
    pub fn operation_by_method_mut(&mut self, method: Method) -> Option<&mut Option<Operation>> {
        Some(match method {
            Method::GET => &mut self.get,
            Method::PUT => &mut self.put,
            Method::POST => &mut self.post,
            Method::DELETE => &mut self.delete,
            Method::OPTIONS => &mut self.options,
            Method::HEAD => &mut self.head,
            Method::PATCH => &mut self.patch,
            Method::TRACE => &mut self.trace,
            _ => return None,
        })
    }
    pub fn operations_mut(&mut self) -> impl Iterator<Item = &mut Operation> {
        [
            &mut self.delete,
            &mut self.get,
            &mut self.head,
            &mut self.options,
            &mut self.patch,
            &mut self.post,
            &mut self.put,
            &mut self.trace,
        ]
        .into_iter()
        .filter_map(|op| op.as_mut())
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Operation {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub tags: Vec<String>,
        pub summary: Option<String>,
        pub description: Option<String>,
        pub external_docs: Option<ExternalDocs>,
        pub operation_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub parameters: Vec<MaybeRef<Parameter>>,
        pub request_body: Option<MaybeRef<RequestBody>>,
        #[serde(default, skip_serializing_if = "Responses::is_empty")]
        pub responses: Responses,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub callbacks: Map<String, MaybeRef<Callback>>,
        pub deprecated: Option<bool>,
        pub security: Option<Vec<SecurityRequirement>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub servers: Vec<Server>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Operation {
    pub fn add_response(&mut self, status_code: String, response: MaybeRef<Response>) {
        match self.responses.responses.entry(status_code) {
            Entry::Occupied(entry) => {
                panic!("colliding responses for status code {}", entry.key());
            }
            Entry::Vacant(entry) => {
                entry.insert(response);
            }
        }
    }
}

macros::define_openapi_spec_object! {
    pub struct ExternalDocs {
        pub description: Option<String>,
        pub url: String,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl ExternalDocs {
    pub fn new(url: String) -> Self {
        Self {
            url,
            description: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    pub struct Parameter {
        pub name: String,
        pub r#in: ParameterIn,
        pub description: Option<String>,
        pub required: Option<bool>,
        pub deprecated: Option<bool>,
        pub allow_empty_value: Option<bool>,
        pub style: Option<String>,
        pub explode: Option<bool>,
        pub allow_reserved: Option<bool>,
        pub schema: Option<MaybeRef<Schema>>,
        pub example: Option<Value>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub examples: Map<String, MaybeRef<Example>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub content: Map<String, MediaTypeContent>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

impl Parameter {
    pub fn new(name: String, r#in: ParameterIn) -> Self {
        Self {
            name,
            r#in,
            description: Default::default(),
            required: Default::default(),
            deprecated: Default::default(),
            allow_empty_value: Default::default(),
            style: Default::default(),
            explode: Default::default(),
            allow_reserved: Default::default(),
            schema: Default::default(),
            example: Default::default(),
            examples: Default::default(),
            content: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct RequestBody {
        pub description: Option<String>,
        pub content: Map<String, MediaTypeContent>,
        pub required: Option<bool>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl RequestBody {
    pub fn new(content: Map<String, MediaTypeContent>) -> Self {
        Self {
            content,
            description: Default::default(),
            required: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct MediaTypeContent {
        pub schema: Option<MaybeRef<Schema>>,
        pub example: Option<Value>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub examples: Map<String, MaybeRef<Example>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub encoding: Map<String, Encoding>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Encoding {
        pub content_type: Option<String>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub headers: Map<String, MaybeRef<Header>>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Responses {
        #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
        pub responses: Map<String, MaybeRef<Response>>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Responses {
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty() && self.extensions.is_empty()
    }
    pub fn merge_with(&mut self, other: Responses) {
        for (status_code, response) in other.responses {
            match self.responses.entry(status_code) {
                Entry::Occupied(mut entry) => merge_response_into(entry.get_mut(), response),
                Entry::Vacant(entry) => {
                    entry.insert(response);
                }
            }
        }
        self.extensions.merge_with(other.extensions);
    }
    pub fn merge_iter(iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut iter = iter.into_iter();
        let mut responses = iter.next()?;
        for next in iter {
            responses.merge_with(next);
        }
        Some(responses)
    }
}

/// Merges one response into another.
///
/// This function currently simply panics as merging responses is not supported.
/// See issue #1 for more details.
pub fn merge_response_into(_a: &mut MaybeRef<Response>, _b: MaybeRef<Response>) {
    panic!("cannot merge responses");
}

/// Merges multiple responses into one or `None` if the iterator is empty.
///
/// This function currently simply panics for more than two responses as merging is not supported.
/// See issue #1 for more details.
pub fn merge_responses_iter(
    iter: impl IntoIterator<Item = MaybeRef<Response>>,
) -> Option<MaybeRef<Response>> {
    let mut iter = iter.into_iter();
    let mut response = iter.next()?;
    for next in iter {
        merge_response_into(&mut response, next);
    }
    Some(response)
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Response {
        pub description: String,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub headers: Map<String, MaybeRef<Header>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub content: Map<String, MediaTypeContent>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub links: Map<String, MaybeRef<Link>>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Callback {
        #[serde(flatten)]
        pub callbacks: Map<String, PathItem>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Example {
        pub summary: Option<String>,
        pub description: Option<String>,
        pub value: Option<Value>,
        pub external_value: Option<String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Link {
        pub operation_ref: Option<String>,
        pub operation_id: Option<String>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub parameters: Map<String, Value>,
        pub request_body: Option<Value>,
        pub description: Option<String>,
        pub server: Option<Server>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Header {
        pub description: Option<String>,
        pub required: Option<bool>,
        pub deprecated: Option<bool>,
        pub style: Option<String>,
        pub explode: Option<bool>,
        pub schema: Option<MaybeRef<Schema>>,
        pub example: Option<Value>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub examples: Map<String, MaybeRef<Example>>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub content: Map<String, MediaTypeContent>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    pub struct Tag {
        pub name: String,
        pub description: Option<String>,
        pub external_docs: Option<ExternalDocs>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Default::default(),
            external_docs: Default::default(),
            extensions: Default::default(),
        }
    }
}

macros::define_openapi_spec_object! {
    pub struct SecurityScheme {
        #[serde(flatten)]
        pub r#type: SecuritySchemeType,
        pub description: Option<String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

impl SecurityScheme {
    pub fn new(r#type: SecuritySchemeType) -> Self {
        Self {
            r#type,
            description: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
#[non_exhaustive]
#[expect(clippy::large_enum_variant)]
pub enum SecuritySchemeType {
    ApiKey(ApiKeySecurityScheme),
    Http(HttpSecurityScheme),
    MutualTLS,
    Oauth2(Oauth2SecurityScheme),
    OpenIdConnect(OpenIdConnectSecurityScheme),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "in", content = "name")]
#[non_exhaustive]
pub enum ApiKeySecurityScheme {
    Query(String),
    Header(String),
    Cookie(String),
}

macros::define_openapi_spec_object! {
    pub struct HttpSecurityScheme {
        pub scheme: String,
        pub bearer_format: Option<String>,
    }
}

impl HttpSecurityScheme {
    pub fn new(scheme: String) -> Self {
        Self {
            scheme,
            bearer_format: None,
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct Oauth2SecurityScheme {
        pub flows: OAuthFlows,
    }
}

macros::define_openapi_spec_object! {
    pub struct OpenIdConnectSecurityScheme {
        pub open_id_connect_url: String,
    }
}

impl OpenIdConnectSecurityScheme {
    pub fn new(open_id_connect_url: String) -> Self {
        Self {
            open_id_connect_url,
        }
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct OAuthFlows {
        pub implicit: Option<OAuthFlow>,
        pub password: Option<OAuthFlow>,
        pub client_credentials: Option<OAuthFlow>,
        pub authorization_code: Option<OAuthFlow>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct OAuthFlow {
        pub authorization_url: Option<String>,
        pub token_url: Option<String>,
        pub refresh_url: Option<String>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        pub scopes: Map<String, String>,

        #[serde(flatten)]
        pub extensions: Extensions,
    }
}

macros::define_openapi_spec_object! {
    #[derive(Default)]
    pub struct SecurityRequirement {
        #[serde(flatten)]
        pub schemes: Map<String, Vec<String>>,
    }
}

impl SecurityRequirement {
    pub fn is_empty(&self) -> bool {
        self.schemes.is_empty()
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Extensions(pub Map<String, Value>);

impl Extensions {
    pub fn merge_with(&mut self, other: Extensions) {
        merge_maps(&mut self.0, other.0, |value_mut, value| {
            if *value_mut != value {
                panic!("colliding extension values")
            }
        });
    }
}

impl Deref for Extensions {
    type Target = Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Extensions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for Extensions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&format!("x-{k}"), v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Extensions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = Map::<String, Value>::deserialize(deserializer)?;
        let map = map
            .into_iter()
            .filter(|(k, _)| k.starts_with("x-"))
            .map(|(mut k, v)| {
                k.replace_range(0..2, "");
                (k, v)
            })
            .collect();
        Ok(Self(map))
    }
}

#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, assert_json_snapshot};
    use serde_json::json;

    use super::*;

    #[test]
    pub fn extensions() {
        assert_json_snapshot!(Extensions(Map::from_iter([
            (String::from("foo"), json!("value")),
            (String::from("bar"), json!(42))
        ])));
        assert_debug_snapshot!(serde_json::from_value::<Extensions>(json!({
            "x-foo": 42,
            "bar": "nowhere",
            "x-foobar": "ok",
        })));
    }

    #[test]
    pub fn defaults() {
        assert_json_snapshot!(OpenApi::new(
            "3.0.0".to_owned(),
            Info::new("Title".to_owned())
        ));
        assert_json_snapshot!(Contact::default());
        assert_json_snapshot!(License::new("GPL".to_owned()));
        assert_json_snapshot!(Server::new("/url".to_owned()));
        assert_json_snapshot!(Components::default());
        assert_json_snapshot!(Paths::default());
        assert_json_snapshot!(PathItem::default());
        assert_json_snapshot!(Operation::default());
        assert_json_snapshot!(ExternalDocs::new("/url".to_owned()));
        assert_json_snapshot!(Parameter::new("name".to_owned(), ParameterIn::Cookie));
        assert_json_snapshot!(RequestBody::new(Map::default()));
        assert_json_snapshot!(MediaTypeContent::default());
        assert_json_snapshot!(Encoding::default());
        assert_json_snapshot!(Responses::default());
        assert_json_snapshot!(Response::default());
        assert_json_snapshot!(Callback::default());
        assert_json_snapshot!(Example::default());
        assert_json_snapshot!(Link::default());
        assert_json_snapshot!(Header::default());
        assert_json_snapshot!(Tag::new("name".to_owned()));
        assert_json_snapshot!(Ref::new("ref".to_owned()));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::MutualTLS));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::ApiKey(
            ApiKeySecurityScheme::Cookie("cookie_name".to_owned())
        )));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::Oauth2(
            Oauth2SecurityScheme::default()
        )));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::Oauth2(
            Oauth2SecurityScheme::default()
                .with_flows(OAuthFlows::default().with_implicit(OAuthFlow::default()))
        )));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::OpenIdConnect(
            OpenIdConnectSecurityScheme::new("/url".to_owned())
        )));
        assert_json_snapshot!(SecurityScheme::new(SecuritySchemeType::Http(
            HttpSecurityScheme::new("scheme".to_owned())
        )));
        assert_json_snapshot!(OAuthFlows::default());
        assert_json_snapshot!(OAuthFlow::default());
        assert_json_snapshot!(SecurityRequirement::default());
    }
}
