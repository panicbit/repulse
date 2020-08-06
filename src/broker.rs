use anyhow::*;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use futures::{pin_mut, channel::{oneshot, mpsc}};
use tokio::io;
use crate::frame::Frame;
use future::{BoxFuture, Abortable};

type FrameTransmitter = mpsc::Sender<(Frame, oneshot::Sender<Result<()>>)>;
type FrameReceiver = mpsc::Receiver<(Frame, oneshot::Sender<Result<()>>)>;
pub type SendFrame = Box<dyn Fn(Frame) -> BoxFuture<'static, Result<()>> + Send>;

pub(crate) fn start_broker<S, OnFrame>(stream: S, on_frame: OnFrame) -> (SendFrame, AbortHandle)
where
    S: AsyncRead + AsyncWrite + Send + 'static,
    OnFrame: FnMut(Result<Frame>) + Send + 'static,
{
    let (reader, writer) = io::split(stream);
    let (frame_tx, frame_rx) = mpsc::channel(1024);

    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    {
        let read_loop = read_loop(reader, on_frame, abort_handle.clone());
        let read_loop = Abortable::new(read_loop, abort_registration.read_loop);
        tokio::spawn(read_loop);
    }

    {
        let write_loop = write_loop(writer, frame_rx, abort_handle.clone());
        let write_loop = Abortable::new(write_loop, abort_registration.write_loop);
        tokio::spawn(write_loop);
    }

    let send_frame = create_frame_sender(frame_tx);

    (Box::new(send_frame), abort_handle)
}

async fn read_loop<R, OnFrame>(reader: R, mut on_frame: OnFrame, abort_handle: AbortHandle)
where
    R: AsyncRead + Unpin,
    OnFrame: FnMut(Result<Frame>) + Send + 'static,
{
    let mut frames = Frame::stream(reader);

    while let Some(frame) = frames.next().await {
        let frame = frame.context("Failed to read frame");
        let error_occurred = frame.is_err();

        on_frame(frame);

        if error_occurred {
            abort_handle.abort();
            return;
        }
    }

    abort_handle.abort();
}

async fn write_loop<W>(
    mut writer: W,
    frame_rx: FrameReceiver,
    abort_handle: AbortHandle,
)
where
    W: AsyncWrite + Unpin,
{
    pin_mut!(frame_rx);

    while let Some((frame, response_tx)) = frame_rx.next().await {
        let result = frame.write_to(&mut writer).await
            .context("Failed to send frame");
        let error_occurred = result.is_err();

        response_tx.send(result).ok();

        if error_occurred {
            abort_handle.abort();
            return;
        }
    }

    abort_handle.abort();
}

fn create_frame_sender(frame_tx: FrameTransmitter) -> impl Fn(Frame) -> BoxFuture<'static, Result<()>> {
    move |frame| {
        let (result_tx, result_rx) = oneshot::channel();

        let queue_result = frame_tx.clone().try_send((frame, result_tx));

        async move {
            queue_result
                .context("Send queue is full")?;

            if let Ok(write_result) = result_rx.await {
                write_result
                    .context("Failed to write frame")?;
            }

            Ok(())
        }
        .boxed()
    }
}

#[derive(Clone, Debug)]
pub struct AbortHandle {
    read_loop: future::AbortHandle,
    write_loop: future::AbortHandle,
}

impl AbortHandle {
    pub fn abort(&self) {
        self.read_loop.abort();
        self.write_loop.abort();
    }

    fn new_pair() -> (Self, AbortRegistration) {
        let (read_loop_abort_handle, read_loop_abort_registration) = future::AbortHandle::new_pair();
        let (write_loop_abort_handle, write_loop_abort_registration) = future::AbortHandle::new_pair();

        let abort_handle = Self {
            read_loop: read_loop_abort_handle,
            write_loop: write_loop_abort_handle,
        };

        let abort_registration = AbortRegistration {
            read_loop: read_loop_abort_registration,
            write_loop: write_loop_abort_registration,
        };

        (abort_handle, abort_registration)
    }
}

struct AbortRegistration {
    read_loop: future::AbortRegistration,
    write_loop: future::AbortRegistration,
}
