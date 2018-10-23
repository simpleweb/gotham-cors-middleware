//! Library aimed at providing CORS functionality
//! for Gotham based servers.
//!
//! Currently a very basic implementation with
//! limited customisability.
#[macro_use]
extern crate gotham_derive;

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate unicase;

use futures::Future;
use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::{FromState, State};
use hyper::header::{
    AccessControlAllowCredentials, AccessControlAllowHeaders, AccessControlAllowMethods,
    AccessControlAllowOrigin, AccessControlMaxAge, Headers, Origin,
};
use hyper::Method;
use std::option::Option;
use unicase::Ascii;

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
    /// extern crate hyper;
    ///
    /// use gotham::pipeline::new_pipeline;
    /// use gotham_cors_middleware::CORSMiddleware;
    /// use gotham::pipeline::single::single_pipeline;
    /// use gotham::router::builder::*;
    /// use gotham::router::Router;
    /// use hyper::Method;
    ///
    /// fn create_custom_middleware() -> CORSMiddleware {
    ///     let methods = vec![Method::Delete, Method::Get, Method::Head, Method::Options];
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
            Method::Delete,
            Method::Get,
            Method::Head,
            Method::Options,
            Method::Patch,
            Method::Post,
            Method::Put,
        ];

        let origin = None;
        let max_age = 86400;

        CORSMiddleware::new(methods, origin, max_age)
    }
}

impl Middleware for CORSMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let settings = self.clone();
        let f = chain(state).map(|(state, response)| {
            let origin: String;
            if settings.origin.is_none() {
                let origin_raw = Headers::borrow_from(&state).get::<Origin>().clone();
                let ori = match origin_raw {
                    Some(o) => o.to_string(),
                    None => "*".to_string(),
                };

                origin = ori;
            } else {
                origin = settings.origin.unwrap();
            };

            let mut headers = Headers::new();

            headers.set(AccessControlAllowCredentials);
            headers.set(AccessControlAllowHeaders(vec![
                Ascii::new("Authorization".to_string()),
                Ascii::new("Content-Type".to_string()),
            ]));
            headers.set(AccessControlAllowOrigin::Value(origin));
            headers.set(AccessControlAllowMethods(settings.methods));
            headers.set(AccessControlMaxAge(settings.max_age));

            let res = response.with_headers(headers);

            (state, res)
        });

        Box::new(f)
    }
}

#[cfg(test)]
mod tests {
    extern crate mime;

    use super::*;

    use futures::future;
    use gotham::http::response::create_response;
    use gotham::pipeline::new_pipeline;
    use gotham::pipeline::single::single_pipeline;
    use gotham::router::builder::*;
    use gotham::router::Router;
    use gotham::test::TestServer;
    use hyper::Method::Options;
    use hyper::StatusCode;
    use hyper::{Get, Head};

    // Since we cannot construct 'State' ourselves, we need to test via an 'actual' app
    fn handler(state: State) -> Box<HandlerFuture> {
        let body = "Hello World".to_string();

        let response = create_response(
            &state,
            StatusCode::Ok,
            Some((body.into_bytes(), mime::TEXT_PLAIN)),
        );

        Box::new(future::ok((state, response)))
    }

    fn default_router() -> Router {
        let (chain, pipeline) =
            single_pipeline(new_pipeline().add(CORSMiddleware::default()).build());

        build_router(chain, pipeline, |route| {
            route.request(vec![Get, Head, Options], "/").to(handler);
        })
    }

    fn custom_router() -> Router {
        let methods = vec![Method::Delete, Method::Get, Method::Head, Method::Options];

        let max_age = 1000;

        let origin = Some("http://www.example.com".to_string());

        let (chain, pipeline) = single_pipeline(
            new_pipeline()
                .add(CORSMiddleware::new(methods, origin, max_age))
                .build(),
        );

        build_router(chain, pipeline, |route| {
            route.request(vec![Get, Head, Options], "/").to(handler);
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

        assert_eq!(response.status(), StatusCode::Ok);
        let headers = response.headers();
        assert_eq!(
            headers
                .get::<AccessControlAllowOrigin>()
                .unwrap()
                .to_string(),
            "*".to_string()
        );
        assert_eq!(
            headers.get::<AccessControlMaxAge>().unwrap().to_string(),
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

        assert_eq!(response.status(), StatusCode::Ok);
        let headers = response.headers();
        assert_eq!(
            headers
                .get::<AccessControlAllowOrigin>()
                .unwrap()
                .to_string(),
            "http://www.example.com".to_string()
        );
        assert_eq!(
            headers.get::<AccessControlMaxAge>().unwrap().to_string(),
            "1000".to_string()
        );
    }

    #[test]
    fn test_new_cors_middleware() {
        let methods = vec![Method::Delete, Method::Get, Method::Head, Method::Options];

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
            Method::Delete,
            Method::Get,
            Method::Head,
            Method::Options,
            Method::Patch,
            Method::Post,
            Method::Put,
        ];

        assert_eq!(test.methods, methods);

        assert_eq!(test.max_age, 86400);

        assert_eq!(test.origin, None);
    }
}
