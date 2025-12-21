use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

use super::Method;

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    path: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl Request {
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
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
            path: path.to_string(),
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
