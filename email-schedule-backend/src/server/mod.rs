
use std::io::Read;
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::http::{Method, Request, Response, StatusCode};

use crate::CacheRef;

pub async fn serve(req: Request<Incoming>, cache: CacheRef) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut response = Response::new(Full::default());
    match req.method() {
        // Help route.
        &Method::GET => {
            match req.uri().path() {

                "/sign-up" => {
                    
                }

                "/favicon.ico" => {
                    
                }

                _ => {
                    *response.status_mut() = StatusCode::NOT_FOUND;
                }


            }
        }
        // Catch-all 404.
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(response)
}
