use crate::{
    reqresp::ReqResp,
    sap::{ConfirmValue, Indication, IndicationValue, Request, RequestValue, ResponseValue},
};

/// The main interface to the MAC layer. It can be used to make requests and receive indications
pub struct MacCommander {
    request_confirm_channel: ReqResp<RequestValue, ConfirmValue, 4>,
    indication_response_channel: ReqResp<IndicationValue, ResponseValue, 4>,
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
    pub async fn request<R: Request>(&self, request: R) -> R::Confirm {
        self.request_confirm_channel
            .request(request.into())
            .await
            .into()
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

pub(crate) struct MacHandler<'a> {
    commander: &'a MacCommander,
}

impl MacHandler<'_> {
    #[allow(dead_code)]
    pub async fn indicate<I: Indication>(&self, indication: I) -> I::Response {
        self.commander
            .indication_response_channel
            .request(indication.into())
            .await
            .into()
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
    pub fn into_concrete<U: Request>(self) -> RequestResponder<'a, U> {
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

impl<T: Request> RequestResponder<'_, T> {
    pub fn respond(self, response: T::Confirm) {
        self.commander
            .request_confirm_channel
            .respond(self.id, response.into());
    }
}
