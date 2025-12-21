use crate::http::response::IntoResponse;

use super::{Method, Request, Response};

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, req: &Request) -> Response;
}

impl<F, R> Handler for F
where
    F: Fn(&Request) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    fn handle(&self, req: &Request) -> Response {
        (self)(req).into_response()
    }
}

pub trait RouteHandler: Send + Sync + 'static {
    fn handle(&self, prefix: &str, request: &Request) -> Option<Response>;
}

pub struct Router {
    prefix: String,
    routes: Vec<Box<dyn RouteHandler>>,
}

struct Route {
    path: String,
    method: Method,
    handler: Box<dyn Handler>,
}

impl RouteHandler for Route {
    fn handle(&self, prefix: &str, request: &Request) -> Option<Response> {
        let route_path = format!("{}{}", prefix, self.path);
        let is_matching = request.path() == route_path && self.method == *request.method();
        if !is_matching {
            return None;
        }
        Some(self.handler.handle(request))
    }
}

impl RouteHandler for Router {
    fn handle(&self, prefix: &str, request: &Request) -> Option<Response> {
        let prefix = format!("{}{}", prefix, self.prefix);
        self.routes
            .iter()
            .find_map(|route| route.handle(&prefix, request))
    }
}

impl Router {
    pub fn prefix(prefix: &str) -> Self {
        Router {
            prefix: prefix.to_string(),
            routes: Vec::new(),
        }
    }

    pub fn new() -> Self {
        Router::prefix("")
    }

    pub fn route<F, R>(mut self, path: &str, method: Method, handler: F) -> Self
    where
        F: Fn(&Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        self.routes.push(Box::new(Route {
            path: path.to_string(),
            method,
            handler: Box::new(handler),
        }));
        self
    }

    pub fn get<F, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(&Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        self.route(path, Method::Get, handler)
    }

    pub fn post<F, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(&Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        self.route(path, Method::Post, handler)
    }

    pub fn put<F, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(&Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        self.route(path, Method::Put, handler)
    }

    pub fn delete<F, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(&Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        self.route(path, Method::Delete, handler)
    }

    pub fn nested(mut self, router: Router) -> Self {
        self.routes.push(Box::new(router));
        self
    }

    pub(crate) fn handle(&self, request: &Request) -> Response {
        for route in &self.routes {
            if let Some(response) = route.handle(&self.prefix, request) {
                return response;
            }
        }
        Response::not_found()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
