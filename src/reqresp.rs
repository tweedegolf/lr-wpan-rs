use core::sync::atomic::{AtomicU32, Ordering};

use embassy_futures::join::join;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use maitake_sync::WaitMap;

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

    pub async fn request(&self, request: Request) -> Response {
        let current_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let (response, _) = join(
            self.responses.wait(current_id),
            self.requests.send((current_id, request)),
        )
        .await;

        response.expect("Always succeeds because we use a unique ID")
    }

    pub async fn wait_for_request(&self) -> (u32, Request) {
        self.requests.receive().await
    }

    pub fn respond(&self, id: u32, response: Response) {
        self.responses.wake(&id, response);
    }
}

#[cfg(test)]
mod tests {
    use embassy_futures::join::join_array;

    use super::*;

    #[tokio::test]
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

    #[tokio::test]
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
