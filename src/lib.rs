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
use unicase::Ascii;

#[derive(Clone, NewMiddleware)]
pub struct CORSMiddleware;

impl Middleware for CORSMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let f = chain(state).map(|(state, response)| {
            let origin = {
                let origin_raw = Headers::borrow_from(&state).get::<Origin>().clone();
                let ori = match origin_raw {
                    Some(o) => o.to_string(),
                    None => "*".to_string(),
                };

                ori
            };

            let mut headers = Headers::new();

            headers.set(AccessControlAllowCredentials);
            headers.set(AccessControlAllowHeaders(vec![
                Ascii::new("Authorization".to_string()),
                Ascii::new("Content-Type".to_string()),
            ]));
            headers.set(AccessControlAllowOrigin::Value(origin));
            headers.set(AccessControlAllowMethods(vec![
                Method::Delete,
                Method::Get,
                Method::Head,
                Method::Options,
                Method::Patch,
                Method::Post,
                Method::Put,
            ]));
            headers.set(AccessControlMaxAge(86400));

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
    use gotham::router::Router;
    use gotham::router::builder::*;
    use gotham::test::TestServer;
    use hyper::Method::Options;
    use hyper::{Get, Head};
    use hyper::StatusCode;

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

    fn router() -> Router {
        let (chain, pipeline) = single_pipeline(
            new_pipeline()
            .add(CORSMiddleware)
            .build(),
        );

        build_router(chain,pipeline, |route| {
            route.request(vec![Get, Head, Options], "/").to(handler);
        })
    }

    #[test]
    fn test_headers_set() {
        let test_server = TestServer::new(router()).unwrap();

        let response = test_server
            .client()
            .get("https://example.com/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let headers = response.headers();
        assert_eq!(headers.get::<AccessControlAllowOrigin>().unwrap().to_string(), "*".to_string()); 
        assert_eq!(headers.get::<AccessControlMaxAge>().unwrap().to_string(), "86400".to_string()); 

    }
}
