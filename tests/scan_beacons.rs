use std::fs::File;

use futures::FutureExt;
use ieee802154::mac::{command::Command, Frame, FrameContent, PanId, ShortAddress};
use lr_wpan_rs::{
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
    ChannelPage,
};

use test_log::test;

#[test(tokio::test(unhandled_panic = "shutdown_runtime", start_paused = true))]
async fn scan_passive() {
    let mut runner = lr_wpan_rs::test_helpers::run::run_mac_engine_multi(3);

    runner
        .aether
        .start_trace(File::create("scan_passive.pcap").unwrap());

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Passive, &[0, 1, 2], true).await;
    runner.aether.stop_trace();

    let mut messages = runner
        .aether
        .parse_trace(File::open("scan_passive.pcap").unwrap());

    assert!(notifications.is_empty());

    assert!(messages.all(|m| matches!(m.content, FrameContent::Beacon(_))));

    assert_eq!(scan_confirm.status, Status::Success);
    assert_eq!(scan_confirm.result_list_size, 2);
    assert!(scan_confirm.unscanned_channels.is_empty());
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list[0],
        PanDescriptor {
            coord_address: ieee802154::mac::Address::Short(PanId(0), ShortAddress(0)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: ieee802154::mac::beacon::SuperframeSpecification {
                beacon_order: ieee802154::mac::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: ieee802154::mac::beacon::SuperframeOrder::SuperframeOrder(10),
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
        scan_confirm.pan_descriptor_list[1],
        PanDescriptor {
            coord_address: ieee802154::mac::Address::Short(PanId(1), ShortAddress(1)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: ieee802154::mac::beacon::SuperframeSpecification {
                beacon_order: ieee802154::mac::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: ieee802154::mac::beacon::SuperframeOrder::SuperframeOrder(10),
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
    let mut runner = lr_wpan_rs::test_helpers::run::run_mac_engine_multi(3);

    runner
        .aether
        .start_trace(File::create("scan_active.pcap").unwrap());

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Active, &[0], true).await;
    runner.aether.stop_trace();

    let mut messages = runner
        .aether
        .parse_trace(File::open("scan_active.pcap").unwrap());

    assert!(notifications.is_empty());

    assert!(matches!(
        messages.next(),
        Some(Frame {
            content: FrameContent::Command(Command::BeaconRequest),
            ..
        })
    ));
    assert!(messages.all(|m| matches!(m.content, FrameContent::Beacon(_))));

    assert_eq!(scan_confirm.status, Status::Success);
    assert_eq!(scan_confirm.result_list_size, 2);
    assert!(scan_confirm.unscanned_channels.is_empty());
    pretty_assertions::assert_eq!(
        scan_confirm.pan_descriptor_list[0],
        PanDescriptor {
            coord_address: ieee802154::mac::Address::Short(PanId(0), ShortAddress(0)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: ieee802154::mac::beacon::SuperframeSpecification {
                beacon_order: ieee802154::mac::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: ieee802154::mac::beacon::SuperframeOrder::SuperframeOrder(10),
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
        scan_confirm.pan_descriptor_list[1],
        PanDescriptor {
            coord_address: ieee802154::mac::Address::Short(PanId(1), ShortAddress(1)),
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
            super_frame_spec: ieee802154::mac::beacon::SuperframeSpecification {
                beacon_order: ieee802154::mac::beacon::BeaconOrder::BeaconOrder(10),
                superframe_order: ieee802154::mac::beacon::SuperframeOrder::SuperframeOrder(10),
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
    let mut runner = lr_wpan_rs::test_helpers::run::run_mac_engine_multi(3);

    runner
        .aether
        .start_trace(File::create("scan_passive_no_auto.pcap").unwrap());

    start_beacon(runner.commanders[0], 0).await;
    start_beacon(runner.commanders[1], 1).await;

    let (scan_confirm, notifications) =
        perform_scan(runner.commanders[2], ScanType::Passive, &[0, 1, 2], false).await;
    runner.aether.stop_trace();

    let messages = runner
        .aether
        .parse_trace(File::open("scan_passive_no_auto.pcap").unwrap());

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
            beacon_order: ieee802154::mac::beacon::BeaconOrder::BeaconOrder(10),
            superframe_order: ieee802154::mac::beacon::SuperframeOrder::SuperframeOrder(10),
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
) -> (ScanConfirm, Vec<BeaconNotifyIndication>) {
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
        .request(ScanRequest {
            scan_type,
            scan_channels: channels.try_into().unwrap(),
            scan_duration: 14,
            channel_page: ChannelPage::Uwb,
            security_info: SecurityInfo::new_none_security(),
        })
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
