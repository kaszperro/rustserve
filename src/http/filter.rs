use crate::http::{Method, Request, Response, response::IntoResponse};

#[derive(Clone)]
pub struct Context<'a> {
    request: &'a Request,
    path_index: usize,
}

impl<'a> Context<'a> {
    pub fn new(request: &'a Request) -> Self {
        Context {
            request,
            path_index: 0,
        }
    }

    pub fn is_path_matched(&self) -> bool {
        self.path_index == self.request.path_segments().len()
    }

    pub(crate) fn next_segment(&mut self) -> Option<&str> {
        let res = self.request.path_segment(self.path_index);
        self.path_index += 1;
        res
    }
}

pub trait Filter: Sized + Send + Sync {
    type Extract;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract>;

    fn and<B: Filter>(self, other: B) -> And<Self, B> {
        And { a: self, b: other }
    }

    fn map<B, F: Fn(Self::Extract) -> B>(self, func: F) -> Map<Self, B, F> {
        Map { filter: self, func }
    }

    fn maybe<B: Filter>(self, other: B) -> Maybe<Self, B> {
        Maybe {
            filter: self,
            other,
        }
    }

    fn path(self, path: &str) -> And<Self, Path> {
        self.and(Path {
            path: path.to_string(),
        })
    }

    fn param<T: From<String> + Send + Sync>(self) -> And<Self, PathParam<T>> {
        self.and(PathParam::new())
    }

    fn or<B: Filter>(self, other: B) -> Or<Self, B> {
        Or { a: self, b: other }
    }
}

pub struct And<A: Filter, B: Filter> {
    a: A,
    b: B,
}

pub struct Or<A: Filter, B: Filter> {
    a: A,
    b: B,
}

pub struct Map<A: Filter, B, F: Fn(A::Extract) -> B> {
    filter: A,
    func: F,
}

pub struct Maybe<A: Filter, B: Filter> {
    filter: A,
    other: B,
}

pub struct Path {
    path: String,
}

pub struct End;

impl Filter for End {
    type Extract = ();

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        if ctx.is_path_matched() {
            Some(())
        } else {
            None
        }
    }
}

pub struct PathParam<T: From<String>> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<String>> PathParam<T> {
    pub fn new() -> Self {
        PathParam {
            _marker: std::marker::PhantomData,
        }
    }
}

impl Filter for Path {
    type Extract = ();

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        for segment in self.path.split('/') {
            if ctx.next_segment() != Some(segment) {
                return None;
            }
        }

        Some(())
    }
}

impl Filter for () {
    type Extract = ();

    fn filter(&self, _ctx: &mut Context) -> Option<Self::Extract> {
        Some(())
    }
}

pub trait OneTuple {
    type Extract;
    fn extract(self) -> Self::Extract;
}

impl<T> OneTuple for (T,) {
    type Extract = T;

    fn extract(self) -> Self::Extract {
        self.0
    }
}

impl<T: From<String> + Send + Sync> Filter for PathParam<T> {
    type Extract = (T,);

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        ctx.next_segment().map(|s| (T::from(s.to_string()),))
    }
}

impl<A: Filter, B: Filter> Filter for Maybe<A, B>
where
    A::Extract: Combiner<(Option<<B::Extract as OneTuple>::Extract>,)>,
    B::Extract: OneTuple,
{
    type Extract = <A::Extract as Combiner<(Option<<B::Extract as OneTuple>::Extract>,)>>::Extract;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        let a = self.filter.filter(ctx)?;

        let mut sub_ctx = ctx.clone();
        let b = (self.other.filter(&mut sub_ctx).map(|b| {
            *ctx = sub_ctx;
            b.extract()
        }),);

        Some(a.combine(b))
    }
}

impl<A: Filter, B: Filter> Filter for And<A, B>
where
    A::Extract: Combiner<B::Extract>,
{
    type Extract = <A::Extract as Combiner<B::Extract>>::Extract;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        let a = self.a.filter(ctx)?;
        let b = self.b.filter(ctx)?;
        Some(a.combine(b))
    }
}

pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A: IntoResponse, B: IntoResponse> IntoResponse for Either<A, B> {
    fn into_response(self) -> Response {
        match self {
            Either::A(a) => a.into_response(),
            Either::B(b) => b.into_response(),
        }
    }
}

impl<A: Filter, B: Filter> Filter for Or<A, B> {
    type Extract = Either<A::Extract, B::Extract>;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        let mut a_ctx = ctx.clone();
        if let Some(a) = self.a.filter(&mut a_ctx) {
            *ctx = a_ctx;
            Some(Either::A(a))
        } else {
            let mut b_ctx = ctx.clone();
            let res = self.b.filter(&mut b_ctx).map(|b| {
                *ctx = b_ctx;
                Either::B(b)
            });
            res
        }
    }
}

impl<A, B, F> Filter for Map<A, B, F>
where
    A: Filter,
    F: Fn(A::Extract) -> B + Send + Sync,
{
    type Extract = B;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        let a = self.filter.filter(ctx)?;
        Some((self.func)(a))
    }
}

pub trait Combiner<T> {
    type Extract;
    fn combine(self, other: T) -> Self::Extract;
}

impl<A> Combiner<()> for A {
    type Extract = A;

    fn combine(self, _other: ()) -> Self::Extract {
        self
    }
}

impl<T> Combiner<(T,)> for () {
    type Extract = (T,);

    fn combine(self, other: (T,)) -> Self::Extract {
        other
    }
}

impl<A, B> Combiner<(B,)> for (A,) {
    type Extract = (A, B);

    fn combine(self, other: (B,)) -> Self::Extract {
        (self.0, other.0)
    }
}

impl<A, B, C> Combiner<(C,)> for (A, B) {
    type Extract = (A, B, C);

    fn combine(self, other: (C,)) -> Self::Extract {
        (self.0, self.1, other.0)
    }
}

pub struct Header {
    name: &'static str,
}

impl Filter for Header {
    type Extract = (String,);

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        ctx.request.header(self.name).map(|s| (s.to_owned(),))
    }
}

pub fn header(name: &'static str) -> Header {
    Header { name }
}

pub fn get(path: &str) -> impl Filter<Extract = ()> {
    Method::Get.path(path)
}

pub fn post(path: &str) -> impl Filter<Extract = ()> {
    Method::Post.path(path)
}

pub fn path(path: &str) -> impl Filter<Extract = ()> {
    Path {
        path: path.to_string(),
    }
}

pub fn end() -> End {
    End
}

pub fn param<T: From<String> + Send + Sync>() -> impl Filter<Extract = (T,)> {
    PathParam::new()
}

impl Filter for Method {
    type Extract = ();

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        if ctx.request.method() == self {
            Some(())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mock_req(method: Method, path: &str) -> Request {
        Request::new(method, path, HashMap::new(), None)
    }

    #[test]
    fn test_path_filter() {
        let filter = path("hello/world");
        let req = mock_req(Method::Get, "/hello/world");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_some());
        assert!(ctx.is_path_matched());

        let req = mock_req(Method::Get, "/hello/other");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_none());
    }

    #[test]
    fn test_method_filter() {
        let filter = get("test");
        let req = mock_req(Method::Get, "/test");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_some());

        let req = mock_req(Method::Post, "/test");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_none());
    }

    #[test]
    fn test_param_filter() {
        let filter = path("user").and(param::<String>());
        let req = mock_req(Method::Get, "/user/alice");
        let mut ctx = Context::new(&req);
        let res = filter.filter(&mut ctx);
        assert_eq!(res, Some(("alice".to_string(),)));
        assert!(ctx.is_path_matched());
    }

    #[test]
    fn test_header_filter() {
        let filter = header("x-api-key");
        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "secret".to_string());
        let req = Request::new(Method::Get, "/", headers, None);
        let mut ctx = Context::new(&req);
        assert_eq!(filter.filter(&mut ctx), Some(("secret".to_string(),)));
    }

    #[test]
    fn test_and_filter() {
        let filter = get("hello").and(header("user-agent"));
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "rust-test".to_string());
        let req = Request::new(Method::Get, "/hello", headers, None);
        let mut ctx = Context::new(&req);
        assert_eq!(filter.filter(&mut ctx), Some(("rust-test".to_string(),)));
    }

    #[test]
    fn test_or_filter() {
        let filter = path("a").or(path("b"));

        let req = mock_req(Method::Get, "/a");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_some());

        let req = mock_req(Method::Get, "/b");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_some());

        let req = mock_req(Method::Get, "/c");
        let mut ctx = Context::new(&req);
        assert!(filter.filter(&mut ctx).is_none());
    }

    #[test]
    fn test_map_filter() {
        let filter = path("val").and(param::<String>()).map(|(s,)| s.len());
        let req = mock_req(Method::Get, "/val/hello");
        let mut ctx = Context::new(&req);
        assert_eq!(filter.filter(&mut ctx), Some(5));
    }

    #[test]
    fn test_maybe_filter() {
        let filter = path("test").maybe(param::<String>());

        let req = mock_req(Method::Get, "/test/val");
        let mut ctx = Context::new(&req);
        assert_eq!(filter.filter(&mut ctx), Some((Some("val".to_string()),)));

        let req = mock_req(Method::Get, "/test");
        let mut ctx = Context::new(&req);
        assert_eq!(filter.filter(&mut ctx), Some((None,)));
    }

    #[test]
    fn test_overlapping_paths_ordered() {
        let filter = path("api/a/b").or(path("api/a"));

        let req = mock_req(Method::Get, "/api/a/b");
        let mut ctx = Context::new(&req);
        let res = filter.filter(&mut ctx);
        assert!(matches!(res, Some(Either::A(_))));
        assert!(ctx.is_path_matched());

        let req = mock_req(Method::Get, "/api/a");
        let mut ctx = Context::new(&req);
        let res = filter.filter(&mut ctx);
        assert!(matches!(res, Some(Either::B(_))));
        assert!(ctx.is_path_matched());
    }

    #[test]
    fn test_overlapping_paths_unordered_fixed() {
        // Without end(), this would match the first branch for /api/a/b and then fail is_path_matched.
        // With end(), path("api/a").and(end()) fails for /api/a/b, so it tries the next branch.
        let filter = path("api/a").and(end()).or(path("api/a/b").and(end()));

        let req = mock_req(Method::Get, "/api/a/b");
        let mut ctx = Context::new(&req);
        let res = filter.filter(&mut ctx);
        // Should match branch B now because branch A failed due to end()
        assert!(matches!(res, Some(Either::B(_))));
        assert!(ctx.is_path_matched());

        let req = mock_req(Method::Get, "/api/a");
        let mut ctx = Context::new(&req);
        let res = filter.filter(&mut ctx);
        // Should match branch A
        assert!(matches!(res, Some(Either::A(_))));
        assert!(ctx.is_path_matched());
    }
}
