use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

use crate::http::filter::Context;
use crate::http::response::IntoResponse;
use crate::http::{Filter, Response};

use super::Method;

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    path_segments: Vec<String>,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl Request {
    pub fn new(
        method: Method,
        path: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Self {
        let path_segments = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let headers = headers
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();

        Request {
            method,
            path_segments,
            headers,
            body,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> String {
        self.path_segments.join("/")
    }

    pub fn path_segments(&self) -> &Vec<String> {
        &self.path_segments
    }

    pub fn path_segment(&self, index: usize) -> Option<&str> {
        self.path_segments.get(index).map(|s| s.as_str())
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    pub(crate) fn parse(stream: &TcpStream) -> Result<Self, ParseError> {
        let mut buf_reader = BufReader::new(stream);
        let mut lines: Vec<String> = Vec::new();

        for line in buf_reader.by_ref().lines() {
            let line = line.map_err(|_| ParseError::IoError)?;
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        let first_line = lines.first().ok_or(ParseError::MalformedRequest)?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();

        let method_str = *parts.get(0).ok_or(ParseError::MalformedRequest)?;
        let path = *parts.get(1).ok_or(ParseError::MalformedRequest)?;

        let path_segments = path.split('/').map(|s| s.to_string()).collect();

        let method: Method = method_str
            .parse()
            .map_err(|_| ParseError::UnrecognizedMethod)?;

        let mut headers: HashMap<String, String> = HashMap::new();
        for line in lines.iter().skip(1) {
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_lowercase(), value.trim().to_string());
            }
        }

        let body = if let Some(content_length) = headers.get("content-length") {
            let length: usize = content_length
                .parse()
                .map_err(|_| ParseError::InvalidContentLength)?;

            let mut buffer = vec![0u8; length];
            buf_reader
                .read_exact(&mut buffer)
                .map_err(|_| ParseError::IoError)?;

            Some(buffer)
        } else {
            None
        };

        Ok(Request {
            method,
            path_segments,
            headers,
            body,
        })
    }
}

#[derive(Debug)]
pub enum ParseError {
    IoError,
    MalformedRequest,
    UnrecognizedMethod,
    InvalidContentLength,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::IoError => write!(f, "I/O error"),
            ParseError::MalformedRequest => write!(f, "malformed request"),
            ParseError::UnrecognizedMethod => write!(f, "unrecognized method"),
            ParseError::InvalidContentLength => write!(f, "invalid content-length"),
        }
    }
}

impl std::error::Error for ParseError {}

pub trait RequestHandler: Send + Sync {
    fn handle(&self, req: &Request) -> Response;
}

impl<A: Filter> RequestHandler for A
where
    A::Extract: IntoResponse,
{
    fn handle(&self, req: &Request) -> Response {
        let mut ctx = Context::new(req);
        let res = self.filter(&mut ctx);

        if !ctx.is_path_matched() {
            return Response::not_found();
        }

        res.map(|r| r.into_response())
            .unwrap_or(Response::not_found())
    }
}
