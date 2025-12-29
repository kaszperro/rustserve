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
        let b = (self.other.filter(ctx).map(|b| b.extract()),);
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

impl<A: Filter, B: Filter> Filter for Or<A, B>
where
    A::Extract: IntoResponse,
    B::Extract: IntoResponse,
{
    type Extract = Response;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        if let Some(a) = self.a.filter(&mut ctx.clone()) {
            Some(a.into_response())
        } else {
            self.b.filter(&mut ctx.clone()).map(|b| b.into_response())
        }
    }
}

impl<A, B, F> Filter for Map<A, B, F>
where
    A: Filter,
    F: Fn(A::Extract) -> B + Send + Sync + 'static,
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
