use crate::error::Error;

pub type Future<T> = dyn futures::Future<Item = T, Error = Error>;
pub type Stream<T> =
    dyn futures::stream::Stream<Item = T, Error = Error> + Send;

pub trait PulseFuture<T> {
    fn into_box(
        self,
    ) -> Box<dyn futures::Future<Item = T, Error = Error> + Send>
    where
        Self: 'static + Send + Sized + futures::Future<Item = T, Error = Error>,
    {
        Box::new(self)
    }
}

impl<T, E, A, F> PulseFuture<T> for futures::MapErr<A, F>
where
    E: ::failure::Fail,
    A: futures::Future<Error = E>,
    F: FnOnce(A::Error) -> Error,
{
}

pub trait PulseStream<T> {
    fn into_box(
        self,
    ) -> Box<dyn futures::stream::Stream<Item = T, Error = Error> + Send>
    where
        Self: 'static
            + Send
            + Sized
            + futures::stream::Stream<Item = T, Error = Error>,
    {
        Box::new(self)
    }
}

impl<T, E, A, F> PulseStream<T> for futures::stream::MapErr<A, F>
where
    E: ::failure::Fail,
    A: futures::stream::Stream<Error = E>,
    F: FnOnce(A::Error) -> Error,
{
}
