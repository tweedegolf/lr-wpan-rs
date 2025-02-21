use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    allocation::{Allocated, Allocation},
    reqresp::{ReqResp, RequestFuture},
    sap::{
        ConfirmValue, DynamicRequest, Indication, IndicationValue, Request, RequestValue,
        ResponseValue,
    },
    time::Instant,
};

pub const CHANNEL_SIZE: usize = 4;

/// The main interface to the MAC layer. It can be used to make requests and receive indications
pub struct MacCommander {
    request_confirm_channel: ReqResp<RequestValue, ConfirmValue, CHANNEL_SIZE>,
    indication_response_channel: ReqResp<IndicationValue, ResponseValue, CHANNEL_SIZE>,
}

impl MacCommander {
    /// Create a new instance
    pub const fn new() -> Self {
        Self {
            request_confirm_channel: ReqResp::new(),
            indication_response_channel: ReqResp::new(),
        }
    }

    /// Make a request to the MAC layer. The typed confirm response is returned.
    /// This API is cancel-safe, though the request may not have been sent at the point of cancellation.
    #[must_use]
    pub async fn request<R: Request>(&self, request: R) -> R::Confirm {
        self.request_confirm_channel
            .request(request.into())
            .await
            .into()
    }

    /// Make a request to the MAC layer. The typed confirm response is returned.
    /// This API is cancel-safe, though the request may not have been sent at the point of cancellation.
    #[must_use]
    pub async fn request_with_allocation<'a, R: DynamicRequest>(
        &self,
        mut request: R,
        allocation: &'a mut [R::AllocationElement],
    ) -> Allocated<'a, R::Confirm>
    where
        R::Confirm: 'a,
    {
        unsafe {
            request.attach_allocation(Allocation {
                ptr: allocation.as_mut_ptr(),
                len: allocation.len(),
            });
        }
        // To make safety easier, shadow the reference so we can't touch it anymore
        #[expect(unused)]
        let allocation = ();

        let confirm = self
            .request_confirm_channel
            .request(request.into())
            .await
            .into();

        Allocated::new(confirm)
    }

    /// Wait until an indication is received. The indication must be responded to using the returned [IndicationResponder].
    /// This API is cancel-safe.
    pub async fn wait_for_indication(&self) -> IndicationResponder<'_, IndicationValue> {
        let (id, indication) = self.indication_response_channel.wait_for_request().await;
        IndicationResponder {
            commander: self,
            indication,
            id,
        }
    }

    /// Get the inverse of the commander where you can receive requests and send indications.
    pub(crate) fn get_handler(&self) -> MacHandler<'_> {
        MacHandler { commander: self }
    }
}

impl Default for MacCommander {
    fn default() -> Self {
        Self::new()
    }
}

pub type IndicateIndirectFuture<'a> =
    RequestFuture<'a, IndicationValue, ResponseValue, CHANNEL_SIZE>;

pub(crate) struct MacHandler<'a> {
    commander: &'a MacCommander,
}

impl<'a> MacHandler<'a> {
    #[allow(dead_code)]
    pub async fn indicate<I: Indication>(&self, indication: I) -> I::Response {
        self.commander
            .indication_response_channel
            .request(indication.into())
            .await
            .into()
    }

    /// Send an indication, but don't immediately wait on it.
    /// Instead the response wait is put in a buffer so it can be dealt with later.
    pub fn indicate_indirect<I: Indication>(&self, indication: I) -> IndicateIndirectFuture<'a> {
        self.commander
            .indication_response_channel
            .request(indication.into())
    }

    pub async fn wait_for_request(&self) -> RequestResponder<'_, RequestValue> {
        let (id, request) = self
            .commander
            .request_confirm_channel
            .wait_for_request()
            .await;
        RequestResponder {
            commander: self.commander,
            request,
            id,
        }
    }
}

pub struct IndicationResponder<'a, T> {
    commander: &'a MacCommander,
    /// The indication that was received
    pub indication: T,
    id: u32,
}

impl<'a> IndicationResponder<'a, IndicationValue> {
    pub fn into_concrete<U: Indication>(self) -> IndicationResponder<'a, U> {
        let Self {
            commander,
            indication,
            id,
        } = self;
        IndicationResponder {
            commander,
            indication: indication.into(),
            id,
        }
    }
}

impl<T: Indication> IndicationResponder<'_, T> {
    pub fn respond(self, response: T::Response) {
        self.commander
            .indication_response_channel
            .respond(self.id, response.into());
    }
}

pub struct RequestResponder<'a, T> {
    commander: &'a MacCommander,
    /// The request that was received
    pub request: T,
    id: u32,
}

impl<'a> RequestResponder<'a, RequestValue> {
    pub fn into_concrete<U: DynamicRequest>(self) -> RequestResponder<'a, U> {
        let Self {
            commander,
            request,
            id,
        } = self;
        RequestResponder {
            commander,
            request: request.into(),
            id,
        }
    }
}

impl<T: DynamicRequest> RequestResponder<'_, T> {
    pub fn respond(self, response: T::Confirm) {
        self.commander
            .request_confirm_channel
            .respond(self.id, response.into());
    }
}

const INDIRECT_INDICATION_COLLECTION_SIZE: usize = 4;

pub struct IndirectIndicationCollection<'a> {
    futures: [IndirectIndicationCollectionSlot<'a>; INDIRECT_INDICATION_COLLECTION_SIZE],
}

struct IndirectIndicationCollectionSlot<'a> {
    future: Option<IndicateIndirectFuture<'a>>,
    expire_time: Instant,
}

impl<'a> IndirectIndicationCollectionSlot<'a> {
    fn project_future(self: Pin<&mut Self>) -> Option<Pin<&mut IndicateIndirectFuture<'a>>> {
        // Safety: The inner future remains just as pinned as before
        unsafe {
            self.get_unchecked_mut()
                .future
                .as_mut()
                .map(|f| Pin::new_unchecked(f))
        }
    }

    fn is_empty(self: Pin<&mut Self>) -> bool {
        self.future.is_none()
    }

    fn fill(mut self: Pin<&mut Self>, future: IndicateIndirectFuture<'a>, deadline: Instant) {
        if !self.as_mut().is_empty() {
            panic!("Cannot fill a non-empty slot");
        }

        self.set(Self {
            future: Some(future),
            expire_time: deadline,
        });
    }

    fn check_expired(mut self: Pin<&mut Self>, current_time: Instant) {
        if !self.as_mut().is_empty() {
            if current_time > self.expire_time {
                self.set(Self {
                    future: None,
                    expire_time: Instant::from_ticks(0),
                });
            }
        }
    }

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<ResponseValue> {
        match self.project_future() {
            Some(future) => future.poll(cx),
            None => Poll::Pending,
        }
    }
}

impl<'a> IndirectIndicationCollection<'a> {
    pub fn new() -> Self {
        Self {
            futures: [const {
                IndirectIndicationCollectionSlot {
                    future: None,
                    expire_time: Instant::from_ticks(0),
                }
            }; INDIRECT_INDICATION_COLLECTION_SIZE],
        }
    }

    fn project_future(
        self: Pin<&mut Self>,
        index: usize,
    ) -> Pin<&mut IndirectIndicationCollectionSlot<'a>> {
        // This is okay because `futures` is pinned when `self` is.
        unsafe { self.map_unchecked_mut(|s| &mut s.futures[index]) }
    }

    /// Push an [IndicateIndirectFuture] onto the collection.
    /// If the collection is full, the function panics.
    pub fn push(
        mut self: Pin<&mut Self>,
        future: IndicateIndirectFuture<'a>,
        expire_time: Instant,
    ) {
        for index in 0..INDIRECT_INDICATION_COLLECTION_SIZE {
            let mut future_slot = self.as_mut().project_future(index);
            if future_slot.as_mut().is_empty() {
                future_slot.fill(future, expire_time);
                return;
            }
        }

        panic!("`push` called on IndirectIndicationCollection while it's at capacity");
    }

    /// Wait on an outstanding indication to be answered.
    ///
    /// This function is cancel-safe.
    pub async fn wait(mut self: Pin<&mut Self>, current_time: Instant) -> ResponseValue {
        // Check for expiry. If this future is long lived it's not super accurate, but that should be fine
        for index in 0..INDIRECT_INDICATION_COLLECTION_SIZE {
            let future_slot = self.as_mut().project_future(index);
            future_slot.check_expired(current_time);
        }

        core::future::poll_fn(|cx| {
            for index in 0..INDIRECT_INDICATION_COLLECTION_SIZE {
                let future_slot = self.as_mut().project_future(index);
                match future_slot.poll(cx) {
                    Poll::Ready(response) => return Poll::Ready(response),
                    Poll::Pending => continue,
                }
            }

            Poll::Pending
        })
        .await
    }
}
