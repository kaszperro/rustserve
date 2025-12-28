use crate::http::{Method, Request};

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

    fn path(self, path: &str) -> Path<Self> {
        Path {
            filter: self,
            path: path.to_string(),
        }
    }
}

pub struct And<A: Filter, B: Filter> {
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

pub struct Path<A: Filter> {
    filter: A,
    path: String,
}

impl<A: Filter> Filter for Path<A> {
    type Extract = A::Extract;

    fn filter(&self, ctx: &mut Context) -> Option<Self::Extract> {
        let filter = self.filter.filter(ctx)?;

        for segment in self.path.split('/') {
            if ctx.request.path_segment(ctx.path_index) != Some(segment) {
                return None;
            }
            ctx.path_index += 1;
        }

        Some(filter)
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
        filter: (),
        path: path.to_string(),
    }
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
