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

