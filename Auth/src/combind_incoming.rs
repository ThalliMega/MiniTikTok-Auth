use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use tonic::transport::server::TcpIncoming;

pub struct CombinedIncoming {
    pub a: TcpIncoming,
    pub b: TcpIncoming,
}

impl Stream for CombinedIncoming {
    type Item = <TcpIncoming as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(value) = Pin::new(&mut self.a).poll_next(cx) {
            return Poll::Ready(value);
        }

        if let Poll::Ready(value) = Pin::new(&mut self.b).poll_next(cx) {
            return Poll::Ready(value);
        }

        Poll::Pending
    }
}
