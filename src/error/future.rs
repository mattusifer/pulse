pub trait PulseFuture<T> {
    fn into_box(self) -> Box<dyn futures::Future<Output = T> + Send>
    where
        Self: 'static + Send + Sized + futures::Future<Output = T>,
    {
        Box::new(self)
    }
}

pub trait PulseStream<T> {
    fn into_box(self) -> Box<dyn futures::stream::Stream<Item = T> + Send>
    where
        Self: 'static + Send + Sized + futures::stream::Stream<Item = T>,
    {
        Box::new(self)
    }
}
