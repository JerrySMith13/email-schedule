//! Simple HTTPS echo service based on hyper_util and rustls
//!
//! First parameter is the mandatory port to use.
//! Certificate and private key are hardcoded to sample files.
//! hyper will automatically use HTTP/2 if a client starts talking HTTP/2,
//! otherwise HTTP/1.1 will be used.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs, io};

use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

mod server;
mod server_state;

fn main() {
    // Serve an echo service over HTTPS, with proper error handling.
    if let Err(e) = run_server() {
        eprintln!("FAILED: {}", e);
        std::process::exit(1);
    }
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

fn get_absolute_path(relative_path: &str) -> io::Result<PathBuf> {
    let mut path = env::current_dir()?;
    path.push(relative_path);
    Ok(path)
}
async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Set a process wide default crypto provider.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // First parameter is port number (optional, defaults to 1337)
    let port = match env::args().nth(1) {
        Some(ref p) => p.parse()?,
        None => 1337,
    };
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);

    println!("Loading certificate and private key...");
    let certs_path = get_absolute_path("encrypt-files/sample.pem")?;
    let key_path = get_absolute_path("encrypt-files/sample.rsa")?;

    // Load public certificate.
    let certs = load_certs(certs_path.to_str().unwrap())?;
    // Load private key.
    let key = load_private_key(key_path.to_str().unwrap())?;
    println!("Loaded certificate and private key!");
    println!("Loading server state...");
    let state = server_state::ServerState::new();
    let state_arc = Arc::new(state);
    let state_arc_clone = state_arc.clone();
    tokio::spawn(server_state::ServerState::maintenance_thread(state_arc_clone));
    println!("Loaded server state!");
    


    println!("Starting to serve on https://{}", addr);

    // Create a TCP listener via tokio.
    let incoming = TcpListener::bind(&addr).await?;

    // the graceful watcher
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    // when this signal completes, start shutdown
    let mut signal = std::pin::pin!(shutdown_signal());
    let graceful_state_arc = state_arc.clone();

    // Build TLS configuration.
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| error(e.to_string()))?;
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    
    let service = service_fn( move |req| {
        // Clone state for each request
        let state = state_arc.clone();
        // Pass state to serve function
        server::serve(req, state)
    });

    let service_arc = Arc::new(service.clone());

    loop {
        tokio::select! {
            Ok((tcp_stream, _remote_addr)) = incoming.accept() => {
                let tls_acceptor = tls_acceptor.clone();
            let serve = service_arc.clone();
            tokio::spawn(async move {
                let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                    Ok(tls_stream) => tls_stream,
                    Err(err) => {
                        eprintln!("failed to perform tls handshake: {err:#}");
                        return;
                    }
                };
                
                if let Err(err) = Builder::new(TokioExecutor::new())
                    .serve_connection(TokioIo::new(tls_stream), Arc::clone(&serve))
                    .await
                {
                    eprintln!("failed to serve connection: {err:#}");
                }
            });
        }

         _ = &mut signal => {
            // shutdown signal received
            eprintln!("Shutting down");
            graceful_state_arc.stop_maintenance();
            eprintln!("Waiting for all connections to close");
            break;
        }

        }
    }
    tokio::select! {
        _ = graceful.shutdown() => {
            eprintln!("all connections gracefully closed");
            Ok(())
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            eprintln!("timed out wait for all connections to close");
            Ok(())
        }
    }
    
}

// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<CertificateDer<'static>>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}
