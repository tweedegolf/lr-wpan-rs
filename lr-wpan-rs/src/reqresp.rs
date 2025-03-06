use core::{
    future::Future,
    sync::atomic::{AtomicU32, Ordering},
    task::Poll,
};

use embassy_futures::join::{Join, join};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, SendFuture},
};
use maitake_sync::{WaitMap, wait_map::Wait};

pub struct ReqResp<Request, Response, const N: usize> {
    requests: Channel<CriticalSectionRawMutex, (u32, Request), N>,
    responses: WaitMap<u32, Response>,
    next_id: AtomicU32,
}

impl<Request, Response, const N: usize> ReqResp<Request, Response, N> {
    pub const fn new() -> Self {
        Self {
            requests: Channel::new(),
            responses: WaitMap::new(),
            next_id: AtomicU32::new(0),
        }
    }

    pub fn request(&self, request: Request) -> RequestFuture<Request, Response, N> {
        let current_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        RequestFuture {
            inner: join(
                self.responses.wait(current_id),
                self.requests.send((current_id, request)),
            ),
        }
    }

    pub async fn wait_for_request(&self) -> (u32, Request) {
        self.requests.receive().await
    }

    pub fn respond(&self, id: u32, response: Response) {
        self.responses.wake(&id, response);
    }
}

pub struct RequestFuture<'a, Request, Response, const N: usize> {
    inner:
        Join<Wait<'a, u32, Response>, SendFuture<'a, CriticalSectionRawMutex, (u32, Request), N>>,
}

impl<Request, Response, const N: usize> Future for RequestFuture<'_, Request, Response, N> {
    type Output = Response;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        // Safety: Inner is just as pinned as the outer, so it should be safe to project
        let inner = unsafe { core::pin::Pin::new_unchecked(&mut self.get_unchecked_mut().inner) };

        match inner.poll(cx) {
            Poll::Ready((response, _)) => {
                Poll::Ready(response.expect("Always succeeds because we use a unique ID"))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use embassy_futures::join::join_array;

    use super::*;

    #[futures_test::test]
    async fn test_echo_single() {
        const MAX_VAL: u32 = 10000;
        let channel = ReqResp::<_, _, 4>::new();

        let requester = async {
            for i in 0..=MAX_VAL {
                assert_eq!(channel.request(i).await, i);
            }
        };

        let responder = async {
            loop {
                let (id, request) = channel.wait_for_request().await;
                channel.respond(id, request);

                if request == MAX_VAL {
                    break;
                }
            }
        };

        join(requester, responder).await;
    }

    #[futures_test::test]
    #[expect(clippy::identity_op, reason = "better code layout")]
    async fn test_echo_multi() {
        const MAX_VAL: u32 = 8 * 10 - 1;
        let channel = ReqResp::<_, _, 4>::new();

        let requester = async {
            for i in (0..=MAX_VAL).step_by(8) {
                let result = join_array([
                    channel.request(i + 0),
                    channel.request(i + 1),
                    channel.request(i + 2),
                    channel.request(i + 3),
                    channel.request(i + 4),
                    channel.request(i + 5),
                    channel.request(i + 6),
                    channel.request(i + 7),
                ])
                .await;

                for (index, r) in result.into_iter().enumerate() {
                    assert_eq!(r, i + index as u32);
                }
            }
        };

        let responder = async {
            loop {
                let (id, request) = channel.wait_for_request().await;
                channel.respond(id, request);

                if request == MAX_VAL {
                    break;
                }
            }
        };

        join(requester, responder).await;
    }
}
