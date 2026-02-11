mod domain;
mod http;
mod multi_exp;
mod route;

use domain::AppState;
use http::{Error as HttpError, HttpRequest, ResponseBody, send_binary, send_html};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Mutex;

fn main() -> std::io::Result<()> {
    let app_state = Rc::new(Mutex::new(AppState::new()));
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, app_state.clone()) {
                    eprintln!("Connection handler error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("TCP accept error: {:?}", e);
            }
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, state: Rc<Mutex<AppState>>) -> Result<(), HttpError> {
    let request = match HttpRequest::from_tcp_stream(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            let status_line = format!("HTTP/1.1 {} Bad Request", e.status_code());
            send_html(&mut stream, &status_line, &e.to_string());
            return Ok(());
        }
    };
    let mut state_guard = state
        .lock()
        .map_err(|e| HttpError::InternalServerError(format!("Mutex poison: {}", e)))?;

    let result = route::route(&request, &mut state_guard);

    drop(state_guard);
    match result {
        Ok(response) => {
            let status_line = format!("HTTP/1.1 {}", response.status);
            match response.body {
                ResponseBody::Html(html) => {
                    send_html(&mut stream, &status_line, &html);
                }
                ResponseBody::Binary(data, content_type) => {
                    send_binary(&mut stream, &status_line, content_type, &data);
                }
            }
        }
        Err(e) => {
            let status_line = format!("HTTP/1.1 {}", e.status_code());
            send_html(&mut stream, &status_line, &e.to_string());
        }
    }

    Ok(())
}
