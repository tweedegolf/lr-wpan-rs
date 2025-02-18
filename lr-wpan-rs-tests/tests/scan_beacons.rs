use futures::FutureExt;
use lr_wpan_rs::{
    allocation::{Allocated, Allocation},
    mac::MacCommander,
    pib::PibValue,
    sap::{
        beacon_notify::BeaconNotifyIndication,
        reset::ResetRequest,
        scan::{ScanConfirm, ScanRequest, ScanType},
        set::SetRequest,
        start::StartRequest,
        IndicationValue, PanDescriptor, SecurityInfo, Status,
    },
    time::Instant,
    wire::{command::Command, Frame, FrameContent, PanId, ShortAddress},
    ChannelPage,
};
use test_log::test;

#[test(tokio::test(unhandled_panic = "shutdown_runtime", start_paused = true))]
async fn scan_passive() {
    let mut runner = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    runner.aether.start_trace("scan_passive");

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Passive, &[0, 1, 2], true).await;
    let trace = runner.aether.stop_trace();

    let mut messages = runner.aether.parse_trace(trace);

    assert!(notifications.is_empty());

    assert!(messages.all(|m| matches!(m.content, FrameContent::Beacon(_))));

    assert_eq!(scan_confirm.status, Status::Success);
    assert_eq!(scan_confirm.result_list_size, 2);
    assert!(scan_confirm.unscanned_channels.is_empty());
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list().nth(0).unwrap(),
        &PanDescriptor {
            coord_address: lr_wpan_rs::wire::Address::Short(PanId(0), ShortAddress(0)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(10),
                final_cap_slot: 0,
                battery_life_extension: false,
                pan_coordinator: true,
                association_permit: false
            },
            gts_permit: false,
            link_quality: 255,
            timestamp: Instant::from_ticks(9830400000),
            security_status: None,
            security_info: SecurityInfo::new_none_security(),
            code_list: ()
        }
    );
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list().nth(1).unwrap(),
        &PanDescriptor {
            coord_address: lr_wpan_rs::wire::Address::Short(PanId(1), ShortAddress(1)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(10),
                final_cap_slot: 0,
                battery_life_extension: false,
                pan_coordinator: true,
                association_permit: false
            },
            gts_permit: false,
            link_quality: 255,
            timestamp: Instant::from_ticks(9830400000),
            security_status: None,
            security_info: SecurityInfo::new_none_security(),
            code_list: ()
        }
    );
}

#[test(tokio::test(unhandled_panic = "shutdown_runtime", start_paused = true))]
async fn scan_active() {
    let mut runner = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    runner.aether.start_trace("scan_active");

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Active, &[0], true).await;
    let trace = runner.aether.stop_trace();

    let mut messages = runner.aether.parse_trace(trace);

    assert!(notifications.is_empty());

    let first_message = messages.next();
    assert!(
        matches!(
            first_message,
            Some(Frame {
                content: FrameContent::Command(Command::BeaconRequest),
                ..
            })
        ),
        "{first_message:?}"
    );
    assert!(messages.all(|m| matches!(m.content, FrameContent::Beacon(_))));

    assert_eq!(scan_confirm.status, Status::Success);
    assert_eq!(scan_confirm.result_list_size, 2);
    assert!(scan_confirm.unscanned_channels.is_empty());
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list().nth(0).unwrap(),
        &PanDescriptor {
            coord_address: lr_wpan_rs::wire::Address::Short(PanId(0), ShortAddress(0)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(10),
                final_cap_slot: 0,
                battery_life_extension: false,
                pan_coordinator: true,
                association_permit: false
            },
            gts_permit: false,
            link_quality: 255,
            timestamp: Instant::from_ticks(9830400000),
            security_status: None,
            security_info: SecurityInfo::new_none_security(),
            code_list: ()
        }
    );
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list().nth(1).unwrap(),
        &PanDescriptor {
            coord_address: lr_wpan_rs::wire::Address::Short(PanId(1), ShortAddress(1)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(10),
                final_cap_slot: 0,
                battery_life_extension: false,
                pan_coordinator: true,
                association_permit: false
            },
            gts_permit: false,
            link_quality: 255,
            timestamp: Instant::from_ticks(9830400000),
            security_status: None,
            security_info: SecurityInfo::new_none_security(),
            code_list: ()
        }
    );
}

#[test(tokio::test(unhandled_panic = "shutdown_runtime", start_paused = true))]
async fn scan_passive_no_auto_request() {
    let mut runner = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    runner.aether.start_trace("scan_passive_no_auto");

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Passive, &[0, 1, 2], false).await;
    let trace = runner.aether.stop_trace();

    let messages = runner.aether.parse_trace(trace);

    assert!(!notifications.is_empty());

    for (message, notification) in messages.zip(notifications) {
        match message.content {
            FrameContent::Beacon(beacon) => {
                assert_eq!(beacon.pending_address, notification.address_list);
                assert_eq!(
                    beacon.superframe_spec,
                    notification.pan_descriptor.super_frame_spec
                );
                assert_eq!(message.payload, &notification.sdu[..]);
                assert_eq!(
                    message.header.source,
                    Some(notification.pan_descriptor.coord_address)
                );
                assert_eq!(message.header.seq, notification.beacon_sequence_number);
            }
            _ => unimplemented!("Not seen in test"),
        }
    }

    assert_eq!(scan_confirm.status, Status::Success);
    assert_eq!(scan_confirm.result_list_size, 0);
    assert!(scan_confirm.unscanned_channels.is_empty());
}

async fn start_beacon(commander: &MacCommander, id: u16) {
    let reset_response = commander
        .request(ResetRequest {
            set_default_pib: true,
        })
        .await;
    assert_eq!(reset_response.status, Status::Success);

    let set_response = commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_SHORT_ADDRESS,
            pib_attribute_value: PibValue::MacShortAddress(ShortAddress(id)),
        })
        .await;
    assert_eq!(set_response.status, Status::Success);

    let start_response = commander
        .request(StartRequest {
            pan_id: PanId(id),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            start_time: 0,
            beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
            superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(10),
            pan_coordinator: true,
            battery_life_extension: false,
            coord_realignment: false,
            coord_realign_security_info: SecurityInfo::new_none_security(),
            beacon_security_info: SecurityInfo::new_none_security(),
        })
        .await;
    assert_eq!(start_response.status, Status::Success);
}

async fn perform_scan(
    commander: &MacCommander,
    scan_type: ScanType,
    channels: &[u8],
    auto_request: bool,
) -> (Allocated<'static, ScanConfirm>, Vec<BeaconNotifyIndication>) {
    let reset_response = commander
        .request(ResetRequest {
            set_default_pib: true,
        })
        .await;
    assert_eq!(reset_response.status, Status::Success);

    commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_AUTO_REQUEST,
            pib_attribute_value: PibValue::MacAutoRequest(auto_request),
        })
        .await;

    let mut wait = core::pin::pin!(commander.wait_for_indication().fuse());

    let mut request = core::pin::pin!(commander
        .request_with_allocation(
            ScanRequest {
                scan_type,
                scan_channels: channels.try_into().unwrap(),
                scan_duration: 14,
                channel_page: ChannelPage::Uwb,
                security_info: SecurityInfo::new_none_security(),
                pan_descriptor_list: Allocation::new(),
            },
            vec![None; 16].leak()
        )
        .fuse());

    let mut beacon_notifications = Vec::new();

    loop {
        futures::select_biased! {
            responder = wait => {
                match responder.indication {
                    IndicationValue::BeaconNotify(_) => {
                        let responder = responder.into_concrete::<BeaconNotifyIndication>();
                        beacon_notifications.push(responder.indication.clone());
                        responder.respond(());
                    },
                    _ => unimplemented!("Not sent in this test"),
                };

                wait.set(commander.wait_for_indication().fuse());
            }
            confirm = request => {
                return (confirm, beacon_notifications);
            }
        }
    }
}
