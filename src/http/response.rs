use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for &Response {
    fn into_response(self) -> Response {
        self.clone()
    }
}

impl<S: AsRef<str>> IntoResponse for (u16, S) {
    fn into_response(self) -> Response {
        Response::new(self.0).body(self.1.as_ref().as_bytes().to_vec())
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl Response {
    pub fn new(status_code: u16) -> Self {
        Response {
            status_code,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn ok<B: Into<Vec<u8>>>(body: B) -> Self {
        let body = body.into();
        Response::new(200).body(body)
    }

    pub fn json<S: AsRef<str>>(json: S) -> Self {
        Response::ok(json.as_ref().as_bytes().to_vec()).header("Content-Type", "application/json")
    }

    pub fn html<S: AsRef<str>>(html: S) -> Self {
        Response::ok(html.as_ref().as_bytes().to_vec())
            .header("Content-Type", "text/html; charset=utf-8")
    }

    pub fn created() -> Self {
        Response::new(201)
    }

    pub fn no_content() -> Self {
        Response::new(204)
    }

    pub fn bad_request() -> Self {
        Response::new(400)
    }

    pub fn not_found() -> Self {
        Response::new(404)
    }

    pub fn internal_error() -> Self {
        Response::new(500)
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body<B: Into<Vec<u8>>>(mut self, body: B) -> Self {
        self.body = Some(body.into());
        self
    }

    fn status_text(&self) -> &'static str {
        match self.status_code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }

    pub(crate) fn write_to_stream(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        write!(
            stream,
            "HTTP/1.1 {} {}\r\n",
            self.status_code,
            self.status_text()
        )?;

        for (key, value) in &self.headers {
            write!(stream, "{}: {}\r\n", key, value)?;
        }

        if let Some(ref body) = self.body {
            if !self.headers.contains_key("Content-Length") {
                write!(stream, "Content-Length: {}\r\n", body.len())?;
            }
        } else {
            write!(stream, "Content-Length: 0\r\n")?;
        }

        write!(stream, "\r\n")?;

        if let Some(ref body) = self.body {
            stream.write_all(body)?;
        }

        stream.flush()
    }
}
