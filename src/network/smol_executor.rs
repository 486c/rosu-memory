use std::net::{Shutdown, TcpListener, TcpStream};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compat::Compat;
use eyre::Error;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use smol::{future, io, prelude::*, Async};

pub struct SmolListener<'a> {
    incoming: Pin<Box<dyn Stream<Item = io::Result<Async<TcpStream>>> + Send + 'a>>,
}

impl<'a> SmolListener<'a> {
    pub fn new(listener: &'a Async<TcpListener>) -> Self {
        Self {
            incoming: Box::pin(listener.incoming()),
        }
    }
}

impl hyper::server::accept::Accept for SmolListener<'_> {
    type Conn = Compat<Async<TcpStream>>;
    type Error = Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let stream = smol::ready!(self.incoming.as_mut().poll_next(cx)).unwrap()?;

        Poll::Ready(Some(Ok(Compat::new(stream))))
    }
}

#[derive(Clone)]
pub struct SmolExecutor;

impl<F: Future + Send + 'static> hyper::rt::Executor<F> for SmolExecutor {
    fn execute(&self, fut: F) {
        smol::block_on(async { drop(fut.await) });
    }
}
