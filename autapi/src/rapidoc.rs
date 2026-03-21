use std::pin::Pin;

use axum::{
    handler::Handler,
    response::{Html, IntoResponse},
};

use crate::{request::Request, response::Response};

#[derive(Clone)]
pub struct RapiDoc {
    pub spec_url: String,
}

impl<S> Handler<(), S> for RapiDoc {
    type Future = Pin<Box<dyn Future<Output = Response> + Send + 'static>>;

    fn call(self, _: Request, _: S) -> Self::Future {
        Box::pin(async move {
            Html(format!(
                r#"
<!doctype html>
<html>
    <head>
        <meta charset="utf-8">
        <script type="module" src="https://unpkg.com/rapidoc/dist/rapidoc-min.js"></script>
    </head>
    <body>
        <rapi-doc spec-url = "{}" show-components = "true"> </rapi-doc>
    </body>
</html>"#,
                self.spec_url
            ))
            .into_response()
        })
    }
}
