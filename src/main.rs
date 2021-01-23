//use err_tools::*;
use hyper::body::{Bytes, HttpBody};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
//use std::convert::Infallible;
use std::net::SocketAddr;

async fn handle(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    println!("Signal recieved");
    let (parts, mut bod) = req.into_parts();

    let d = match bod.data().await {
        Some(Ok(d)) => d,
        Some(Err(e)) => {
            println!("Error d = {:?}", e);
            Bytes::from("")
        }

        None => {
            println!("Nothing doing");
            Bytes::from("")
        }
    };
    let q = parts.uri.query().unwrap_or("No Query");

    let r = format!(
        "Hello from '{}' you said: '{}' ",
        q,
        std::str::from_utf8(&d.slice(..))?
    );

    Ok(Response::new(Body::from(r)))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9056));

    let make_service =
        make_service_fn(|_conn| async { Ok::<_, anyhow::Error>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
