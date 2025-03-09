use std::sync::Arc;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::http::{Method, Request, Response, StatusCode};

use crate::server_state::ServerState;

pub async fn serve(req: Request<Incoming>, state: Arc<ServerState>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match req.method() {
        /*
        Path management
        /web: Serve the web interface
        /favicon.ico: Serve the favicon
        /: Serve the main page
        /oauth2: Serve the OAuth2 interface

        
         */
        &Method::GET => {
            //TEST stuff
            state.add_state(req.headers().get("user-agent").unwrap().to_str().unwrap());
            state.lru.lock().unwrap().put("Hello wolrd".to_string(), Arc::new("Hello world".to_string().into_bytes()));
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .body("Hi world!".into())
                .unwrap());
        }

        // Catch-all 404.
        _ => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Not Found".into())
                .unwrap();
            return Ok(response);
        }
    };
    
}
