use std::{mem::MaybeUninit, task::Poll};

use pin_project_lite::pin_project;

pin_project! {
    pub struct SmolIo<T> {
        #[pin]
        inner: T
    }
}

impl<T> SmolIo<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner
        }
    }
}

impl<T> hyper::rt::Read for SmolIo<T>
where
    T: smol::io::AsyncRead
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let buf_mut = unsafe { buf.as_mut() };
        let mut tmp_buf = [0u8; 1024];

        let remaining = buf_mut.len().min(1024);

        let n = {
            match smol::io::AsyncRead::poll_read(
                self.project().inner, 
                cx, 
                &mut tmp_buf[..remaining]
            ) {
                Poll::Ready(n) => n?,
                Poll::Pending => return std::task::Poll::Pending,
            }
        };

        let tmp_buf = tmp_buf.map(MaybeUninit::new);
        buf_mut[..n].copy_from_slice(&tmp_buf[..n]);

        unsafe {
            buf.advance(n)
        };

        Poll::Ready(Ok(()))
    }
}

impl<T> hyper::rt::Write for SmolIo<T> 
where
    T: smol::io::AsyncWrite 
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let n = {
            match smol::io::AsyncWrite::poll_write(
                self.project().inner, 
                cx, 
                buf
            ) {
                Poll::Ready(n) => n?,
                Poll::Pending => return std::task::Poll::Pending,
            }
        };

        Poll::Ready(Ok(n))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>, 
        cx: &mut std::task::Context<'_>
    ) -> Poll<Result<(), std::io::Error>> {
        smol::io::AsyncWrite::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        smol::io::AsyncWrite::poll_close(self.project().inner, cx)
    }
}

impl<T> smol::io::AsyncRead for SmolIo<T>
where
    T: hyper::rt::Read 
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut read_buf = hyper::rt::ReadBuf::new(buf);

        match hyper::rt::Read::poll_read(
            self.project().inner, 
            cx, 
            read_buf.unfilled()
        ) {
            Poll::Ready(n) => n?,
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(Ok(read_buf.filled().len()))
    }
}


impl<T> smol::io::AsyncWrite for SmolIo<T>
where
    T: hyper::rt::Write
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        hyper::rt::Write::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>, 
        cx: &mut std::task::Context<'_>
    ) -> Poll<std::io::Result<()>> {
        hyper::rt::Write::poll_flush(self.project().inner, cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>, 
        cx: &mut std::task::Context<'_>
    ) -> Poll<std::io::Result<()>> {
        hyper::rt::Write::poll_shutdown(self.project().inner, cx)
    }
}
