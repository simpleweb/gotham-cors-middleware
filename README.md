# Gotham CORS Middleware

This library is aimed to provide CORS functionality to [Gotham.rs](https://gotham.rs/) servers.

Currently this is a very simple implementation with limited customisability.

Requires rust 1.26 or later.

Usage:
```rust
extern crate gotham;
extern crate gotham_cors_middleware;

use gotham::pipeline::new_pipeline;
use gotham_cors_middleware::CORSMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;

pub fn router() -> Router {
    let (chain, pipeline) = single_pipeline(
        new_pipeline()
            .add(CORSMiddleware::default())
            .build(),
    );

    build_router(chain, pipeline, |route| {
     // Routes
    }
}
```

Roadmap:
- [x] Add integration tests
- [x] Add builder that would allow header customisation
- [x] Add documentation
- [ ] See how next version of Gotham requires changes to middeware structure
