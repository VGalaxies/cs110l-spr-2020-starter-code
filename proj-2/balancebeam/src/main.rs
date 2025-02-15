mod request;
mod response;

use clap::Parser;
use rand::{Rng, SeedableRng};
use std::io::ErrorKind;
// use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{delay_for, Duration, Instant};

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Parser, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        help = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, help = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        help = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
        long,
        help = "Path to send request to for active health checks",
        default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        help = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
#[derive(Clone)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
    upstream_states: Vec<bool>,
    client_requests_map: HashMap<String, (Instant, usize)>,
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Construct Proxy State
    let mut state_ori = ProxyState {
        upstream_addresses: options.upstream,
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
        upstream_states: vec![],
        client_requests_map: HashMap::new(),
    };
    state_ori.upstream_states = vec![true; state_ori.upstream_addresses.len()];
    let state = Arc::new(Mutex::new(state_ori));

    // Spawn active health check
    let state_cloned = state.clone();
    task::spawn(async move {
        active_health_check(state_cloned).await;
    });

    // Handle incoming connections
    while let Some(stream) = listener.incoming().next().await {
        match stream {
            Ok(stream) => {
                let state_cloned = state.clone();
                task::spawn(async move {
                    handle_connection(stream, state_cloned).await;
                });
            }
            Err(err) => {
                log::error!("Connection failed: {}", err);
            }
        }
    }
}

async fn active_health_check(state: Arc<Mutex<ProxyState>>) {
    let interval;
    let path;
    let upstream_addresses;

    {
        let state_ref = state.lock().await;
        interval = state_ref.active_health_check_interval;
        path = state_ref.active_health_check_path.clone();
        upstream_addresses = state_ref.upstream_addresses.clone();
    } // release lock

    loop {
        delay_for(Duration::from_secs(interval as u64)).await;

        for upstream_idx in 0..upstream_addresses.len() {
            let mut upstream_state = false;
            let upstream_ip = &upstream_addresses[upstream_idx];

            match TcpStream::connect(upstream_ip).await {
                Ok(mut stream) => {
                    let request = http::Request::builder()
                        .method(http::Method::GET)
                        .uri(path.clone())
                        .header("Host", upstream_ip)
                        .body(Vec::new())
                        .unwrap();
                    if let Err(error) = request::write_to_stream(&request, &mut stream).await {
                        log::error!(
                            "Failed to send request (active health check) to upstream {}: {:?}",
                            upstream_ip,
                            error
                        );
                        // do nothing
                    }
                    match response::read_from_stream(&mut stream, request.method()).await {
                        Ok(response) => {
                            upstream_state = response.status() == http::StatusCode::OK;
                        }
                        Err(error) => {
                            log::error!(
                                "Error reading response (active health check) from server: {:?}",
                                error
                            );
                            // do nothing
                        }
                    }
                }
                Err(error) => {
                    log::error!(
                        "Error establishing connection with server {}: {:?}",
                        upstream_ip,
                        error
                    );
                    // do nothing
                }
            }

            log::info!(
                "Active health check result: upstream {} -> {}",
                upstream_ip,
                upstream_state
            );

            {
                let mut state_ref = state.lock().await;
                state_ref.upstream_states[upstream_idx] = upstream_state;
            } // release lock
        }
    }
}

async fn connect_to_upstream(state: Arc<Mutex<ProxyState>>) -> Result<TcpStream, std::io::Error> {
    // TODO: implement failover (milestone 3)
    let mut state_ref = state.lock().await;
    if !state_ref.upstream_states.contains(&true) {
        log::error!("Failed to connect: all upstreams are dead");
        return Err(std::io::Error::new(ErrorKind::Other, "oh no!"));
    }

    let mut rng = rand::rngs::StdRng::from_entropy();
    loop {
        let upstream_idx = rng.gen_range(0, state_ref.upstream_addresses.len());
        if !state_ref.upstream_states[upstream_idx] {
            continue;
        }
        let upstream_ip = &state_ref.upstream_addresses[upstream_idx];

        match TcpStream::connect(upstream_ip).await {
            Ok(stream) => {
                return Ok(stream);
            }
            Err(err) => {
                log::warn!("Failed to connect to upstream {}: {:?}", upstream_ip, err);
                state_ref.upstream_states[upstream_idx] = false;
                continue;
            }
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {:?}", error);
        return;
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: Arc<Mutex<ProxyState>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let state_cloned = state.clone();
    let mut upstream_conn = match connect_to_upstream(state_cloned).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = upstream_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {:?}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };

        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Rate limiting
        {
            let state_cloned = state.clone();
            let mut state_ref = state_cloned.lock().await;
            let limits = state_ref.max_requests_per_minute;
            let map_cloned = state_ref.client_requests_map.clone();
            if limits != 0 {
                match map_cloned.get(&client_ip) {
                    None => {
                        state_ref.client_requests_map.insert(client_ip.clone(), (Instant::now(), 1));
                    }
                    Some((when, counter)) => {
                        let mut count = counter.clone();
                        if when.elapsed().as_secs() >= 60 {
                            count = 0;
                        }

                        if count >= limits {
                            let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
                            send_response(&mut client_conn, &response).await;
                            log::debug!("Forwarded response `TOO_MANY_REQUESTS` to client");
                            continue;
                        } else {
                            count += 1;
                        }

                        state_ref.client_requests_map.insert(client_ip.clone(), (*when, count));
                    }
                }
            }
        }

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!(
                "Failed to send request to upstream {}: {:?}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await
        {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}
