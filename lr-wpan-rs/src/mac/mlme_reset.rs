use rand_core::RngCore;

use super::{MacConfig, MacError, commander::RequestResponder, state::MacState};
use crate::{
    consts::MAX_BEACON_PAYLOAD_LENGTH,
    phy::Phy,
    pib::{MacPib, MacPibWrite, SequenceNumber},
    sap::reset::{ResetConfirm, ResetRequest},
    time::DelayNsExt,
    wire::{
        ExtendedAddress, PanId, ShortAddress,
        beacon::{BeaconOrder, SuperframeOrder},
    },
};

pub async fn process_reset_request<P: Phy, Rng: RngCore, Delay: DelayNsExt>(
    phy: &mut P,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'_>,
    config: &mut MacConfig<Rng, Delay>,
    responder: RequestResponder<'_, ResetRequest>,
) {
    let result: Result<(), MacError<P::Error>> = async {
        if responder.request.set_default_pib {
            phy.reset().await?;

            *mac_pib = MacPib {
                pib_write: MacPibWrite {
                    associated_pan_coord: false,
                    association_permit: false,
                    auto_request: true,
                    batt_life_ext: false,
                    beacon_payload: [0; MAX_BEACON_PAYLOAD_LENGTH],
                    beacon_payload_length: 0,
                    beacon_order: BeaconOrder::OnDemand,
                    bsn: SequenceNumber::new(config.rng.next_u32() as u8),
                    coord_extended_address: ExtendedAddress::BROADCAST,
                    coord_short_address: ShortAddress::BROADCAST,
                    dsn: SequenceNumber::new(config.rng.next_u32() as u8),
                    gts_permit: true,
                    max_be: 5,
                    max_csma_backoffs: 4,
                    max_frame_retries: 3,
                    min_be: 3,
                    pan_id: PanId::broadcast(),
                    promiscuous_mode: false,
                    response_wait_time: 32,
                    rx_on_when_idle: false,
                    security_enabled: false,
                    short_address: ShortAddress::BROADCAST,
                    transaction_persistence_time: 0x01F4,
                    tx_control_active_duration: P::MODULATION.tx_control_active_duration(),
                    tx_control_pause_duration: P::MODULATION.tx_control_pause_duration(),
                    tx_total_duration: 0,
                },
                extended_address: config.extended_address,
                beacon_tx_time: 0,
                lifs_period: 40,
                sifs_period: 12,
                ranging_supported: true,
                superframe_order: SuperframeOrder::Inactive,
                sync_symbol_offset: 0,
                timestamp_supported: true,
            };
        }

        *mac_state = MacState::new(config);

        Ok(())
    }
    .await;

    responder.respond(ResetConfirm {
        status: match result {
            Ok(()) => crate::sap::Status::Success,
            Err(e) => e.into(),
        },
    });
}
