use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use http_body_util::Full;
use tokio::io::AsyncRead;

pub struct FileBody {
    file: tokio::fs::File,
    buf: Vec<u8>,
    remaining: u64,
}

impl FileBody {
    fn new(file: tokio::fs::File, size: u64) -> Self {
        Self {
            file,
            buf: vec![0u8; 8192],
            remaining: size,
        }
    }
}

impl Body for FileBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.remaining == 0 {
            return Poll::Ready(None);
        }

        let this = &mut *self;
        let to_read = std::cmp::min(this.buf.len() as u64, this.remaining) as usize;
        let mut read_buf = tokio::io::ReadBuf::new(&mut this.buf[..to_read]);

        match Pin::new(&mut this.file).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let n = read_buf.filled().len();
                if n == 0 {
                    if this.remaining > 0 {
                        Poll::Ready(Some(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "file truncated during streaming",
                        ))))
                    } else {
                        Poll::Ready(None)
                    }
                } else {
                    this.remaining -= n as u64;
                    Poll::Ready(Some(Ok(Frame::data(Bytes::copy_from_slice(
                        &this.buf[..n],
                    )))))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.remaining)
    }

    fn is_end_stream(&self) -> bool {
        self.remaining == 0
    }
}

pub enum ResponseBody {
    Full(Full<Bytes>),
    File(FileBody),
}

impl ResponseBody {
    pub fn empty() -> Self {
        Self::Full(Full::new(Bytes::new()))
    }

    pub fn text(s: String) -> Self {
        Self::Full(Full::new(Bytes::from(s)))
    }

    pub fn stream(file: tokio::fs::File, size: u64) -> Self {
        Self::File(FileBody::new(file, size))
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            ResponseBody::Full(full) => Pin::new(full)
                .poll_frame(cx)
                .map(|opt| opt.map(|r| r.map_err(|e| match e {}))),
            ResponseBody::File(file) => Pin::new(file).poll_frame(cx),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            ResponseBody::Full(f) => f.size_hint(),
            ResponseBody::File(f) => f.size_hint(),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            ResponseBody::Full(f) => f.is_end_stream(),
            ResponseBody::File(f) => f.is_end_stream(),
        }
    }
}
