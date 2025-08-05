use fimo_std::{
    context::{ContextBuilder, Error},
    log_trace,
    tasks::BlockingContext,
    tracing::{Config, Level, StdErrLogger, ThreadAccess},
};
use std::{future::Future, pin::Pin, task::Poll};

#[test]
fn block_on_futures() -> Result<(), Error> {
    let logger = StdErrLogger::new();
    ContextBuilder::new()
        .with_tracing_config(
            Config::default()
                .with_max_level(Level::Trace)
                .with_subscribers(&[logger.as_subscriber()]),
        )
        .enter(|context| {
            unsafe { context.enable_cleanup() };
            let _access = ThreadAccess::new();

            let fut = new_nested()?;
            let blocking = BlockingContext::new()?;
            let (a, b) = blocking.block_on(fut);

            assert_eq!(a, LOOP_1);
            assert_eq!(b, LOOP_2);

            Ok(())
        })?
}

const LOOP_1: usize = 5;
const LOOP_2: usize = 10;

fn new_nested() -> Result<impl Future<Output = (usize, usize)>, Error> {
    let a = fimo_std::tasks::Future::new(LoopFuture::<LOOP_1>::new()).enqueue()?;
    let b = fimo_std::tasks::Future::new(LoopFuture::<LOOP_2>::new()).enqueue()?;
    Ok(async move {
        log_trace!("Poll start");
        let a = a.await;
        log_trace!("A finished");
        let b = b.await;
        log_trace!("B finished");
        (a, b)
    })
}

struct LoopFuture<const N: usize> {
    i: usize,
}

impl<const N: usize> LoopFuture<N> {
    fn new() -> Self {
        Self { i: 0 }
    }
}

impl<const N: usize> Future for LoopFuture<N> {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { Pin::into_inner_unchecked(self) };
        log_trace!("Iteration i='{}', data=`{:p}`", inner.i, inner);

        inner.i += 1;
        if inner.i < N {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(inner.i)
        }
    }
}
