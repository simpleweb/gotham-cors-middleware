//! Library aimed at providing CORS functionality
//! for Gotham based servers.
//!
//! Currently a very basic implementation with
//! limited customisability.
#[macro_use]
extern crate gotham_derive;

use futures::prelude::*;
use gotham::{
    handler::HandlerFuture,
    hyper::{
        header::{
            HeaderMap, HeaderValue, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE,
            AUTHORIZATION, CONTENT_TYPE, ORIGIN,
        },
        Method,
    },
    middleware::Middleware,
    state::{FromState, State},
};
use std::{option::Option, pin::Pin};

/// Struct to perform the necessary CORS
/// functionality needed. Allows some
/// customisation through use of the
/// new() function.
///
/// Example of use:
/// ```rust
/// extern crate gotham;
/// extern crate gotham_cors_middleware;
///
/// use gotham::pipeline::new_pipeline;
/// use gotham_cors_middleware::CORSMiddleware;
/// use gotham::pipeline::single::single_pipeline;
/// use gotham::router::builder::*;
/// use gotham::router::Router;
///
/// pub fn router() -> Router {
///     let (chain, pipeline) = single_pipeline(
///         new_pipeline()
///             .add(CORSMiddleware::default())
///             .build()
///     );
///
///     build_router(chain, pipeline, |route| {
///         // Routes
///     })
/// }
/// ```
#[derive(Clone, NewMiddleware, Debug, PartialEq)]
pub struct CORSMiddleware {
    methods: Vec<Method>,
    origin: Option<String>,
    max_age: u32,
}

impl CORSMiddleware {
    /// Create a new CORSMiddleware with custom methods,
    /// origin and max_age properties.
    ///
    /// Expects methods to be a Vec of hyper::Method enum
    /// values, origin to be an Option containing a String
    /// (so allows for None values - which defaults to
    /// returning the sender origin on request or returning
    /// a string of "*" - see the call function source) and
    /// max age to be a u32 value.
    ///
    /// Example of use:
    /// ```rust
    /// extern crate gotham;
    /// extern crate gotham_cors_middleware;
    ///
    /// use gotham::{
    ///     pipeline::{new_pipeline, single::single_pipeline},
    ///     router::{Router, builder::*},
    ///     hyper::Method,
    /// };
    /// use gotham_cors_middleware::CORSMiddleware;
    ///
    /// fn create_custom_middleware() -> CORSMiddleware {
    ///     let methods = vec![Method::DELETE, Method::GET, Method::HEAD, Method::OPTIONS];
    ///
    ///     let max_age = 1000;
    ///
    ///     let origin = Some("http://www.example.com".to_string());
    ///
    ///     CORSMiddleware::new(methods, origin, max_age)
    /// }
    ///
    /// pub fn router() -> Router {
    ///     let (chain, pipeline) = single_pipeline(
    ///         new_pipeline()
    ///             .add(create_custom_middleware())
    ///             .build()
    ///     );
    ///
    ///     build_router(chain, pipeline, |route| {
    ///         // Routes
    ///     })
    /// }
    /// ```
    pub fn new(methods: Vec<Method>, origin: Option<String>, max_age: u32) -> CORSMiddleware {
        CORSMiddleware {
            methods,
            origin,
            max_age,
        }
    }

    /// Creates a new CORSMiddleware with what is currently
    /// the "default" values for methods/origin/max_age.
    ///
    /// This is based off the values that were used previously
    /// before they were customisable. If you need different
    /// values, use the new() function.
    pub fn default() -> CORSMiddleware {
        let methods = vec![
            Method::DELETE,
            Method::GET,
            Method::HEAD,
            Method::OPTIONS,
            Method::PATCH,
            Method::POST,
            Method::PUT,
        ];

        let origin = None;
        let max_age = 86400;

        CORSMiddleware::new(methods, origin, max_age)
    }
}

impl Middleware for CORSMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>>,
    {
        let f = chain(state).and_then(|(state, mut response)| {
            let origin = match self.origin {
                Some(o) => o,
                None => {
                    let origin_raw = HeaderMap::borrow_from(&state).get(ORIGIN).clone();
                    match origin_raw {
                        Some(o) => o.to_str().unwrap().to_string(),
                        None => "*".to_string(),
                    }
                }
            };

            let methods = self
                .methods
                .iter()
                .map(|m| String::from(m.as_str()))
                .collect::<Vec<String>>()
                .join(", ");

            let headers = vec![AUTHORIZATION, CONTENT_TYPE]
                .iter()
                .map(|m| String::from(m.as_str()))
                .collect::<Vec<String>>()
                .join(", ");

            response.headers_mut().insert(
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_str("true").unwrap(),
            );

            response.headers_mut().insert(
                ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_str(&origin).unwrap(),
            );

            response.headers_mut().insert(
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_str(&headers).unwrap(),
            );

            response.headers_mut().insert(
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_str(&methods).unwrap(),
            );

            response
                .headers_mut()
                .insert(ACCESS_CONTROL_MAX_AGE, HeaderValue::from(self.max_age));

            future::ok((state, response))
        });

        f.boxed()
    }
}

#[cfg(test)]
mod tests {
    extern crate mime;

    use super::*;

    use gotham::{
        handler::IntoResponse,
        helpers::http::response::create_response,
        hyper::{Method, StatusCode},
        pipeline::{new_pipeline, single::single_pipeline},
        router::{builder::*, Router},
        test::TestServer,
    };

    // Since we cannot construct 'State' ourselves, we need to test via an 'actual' app
    fn handler(state: State) -> (State, impl IntoResponse) {
        let body = "Hello World".to_string();

        let response = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, body.into_bytes());

        (state, response)
    }

    fn default_router() -> Router {
        let (chain, pipeline) =
            single_pipeline(new_pipeline().add(CORSMiddleware::default()).build());

        build_router(chain, pipeline, |route| {
            route
                .request(vec![Method::GET, Method::HEAD, Method::OPTIONS], "/")
                .to(handler);
        })
    }

    fn custom_router() -> Router {
        let methods = vec![Method::DELETE, Method::GET, Method::HEAD, Method::OPTIONS];

        let max_age = 1000;

        let origin = Some("http://www.example.com".to_string());

        let (chain, pipeline) = single_pipeline(
            new_pipeline()
                .add(CORSMiddleware::new(methods, origin, max_age))
                .build(),
        );

        build_router(chain, pipeline, |route| {
            route
                .request(vec![Method::GET, Method::HEAD, Method::OPTIONS], "/")
                .to(handler);
        })
    }

    #[test]
    fn test_headers_set() {
        let test_server = TestServer::new(default_router()).unwrap();

        let response = test_server
            .client()
            .get("https://example.com/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(
            headers
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            "*".to_string()
        );
        assert_eq!(
            headers
                .get(ACCESS_CONTROL_MAX_AGE)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            "86400".to_string()
        );
    }

    #[test]
    fn test_custom_headers_set() {
        let test_server = TestServer::new(custom_router()).unwrap();

        let response = test_server
            .client()
            .get("https://example.com/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(
            headers
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            "http://www.example.com".to_string()
        );
        assert_eq!(
            headers
                .get(ACCESS_CONTROL_MAX_AGE)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            "1000".to_string()
        );
    }

    #[test]
    fn test_new_cors_middleware() {
        let methods = vec![Method::DELETE, Method::GET, Method::HEAD, Method::OPTIONS];

        let max_age = 1000;

        let origin = Some("http://www.example.com".to_string());

        let test = CORSMiddleware::new(methods.clone(), origin.clone(), max_age.clone());

        let default = CORSMiddleware::default();

        assert_ne!(test, default);

        assert_eq!(test.origin, origin);
        assert_eq!(test.max_age, max_age);
        assert_eq!(test.methods, methods);
    }

    #[test]
    fn test_default_cors_middleware() {
        let test = CORSMiddleware::default();
        let methods = vec![
            Method::DELETE,
            Method::GET,
            Method::HEAD,
            Method::OPTIONS,
            Method::PATCH,
            Method::POST,
            Method::PUT,
        ];

        assert_eq!(test.methods, methods);

        assert_eq!(test.max_age, 86400);

        assert_eq!(test.origin, None);
    }
}
