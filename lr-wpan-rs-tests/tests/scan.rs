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
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        command::Command,
        Address, Frame, FrameContent, PanId, ShortAddress,
    },
    ChannelPage,
};

#[test_log::test]
fn scan_passive() {
    let (commanders, mut aether, mut runner) = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    aether.start_trace("scan_passive");

    // Start the beacons
    runner.attach_test_task(start_beacon(commanders[0], 0, true));
    runner.attach_test_task(start_beacon(commanders[1], 1, false));

    runner.attach_test_task(async {
        // Perform the scan, passively
        let (scan_confirm, notifications) =
            perform_scan(commanders[2], ScanType::Passive, &[0, 1, 2], true).await;

        // Scan needs to be successful
        assert_eq!(scan_confirm.status, Status::Success);
        // We should've scanned one device since we've done a passive scan
        assert_eq!(scan_confirm.result_list_size, 1);
        // All channels should be scanned
        assert!(scan_confirm.unscanned_channels.is_empty());

        // Auto request was true, so we should've gotten zero beacon notifications
        assert!(notifications.is_empty());

        let trace = aether.stop_trace();
        // All the messages in the aether should be beacons
        let mut messages = aether.parse_trace(trace);
        assert!(messages.all(|m| matches!(m.content, FrameContent::Beacon(_))));

        pretty_assertions::assert_eq!(
            scan_confirm.pan_descriptor_list().nth(0).unwrap(),
            &PanDescriptor {
                coord_address: lr_wpan_rs::wire::Address::Short(PanId(0), ShortAddress(0)),
                channel_number: 0,
                channel_page: ChannelPage::Uwb,
                super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                    beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                    superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(
                        10
                    ),
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
        assert_eq!(scan_confirm.pan_descriptor_list().nth(1), None);
    });

    runner.run();
}

#[test_log::test]
fn scan_active() {
    let (commanders, mut aether, mut runner) = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    aether.start_trace("scan_active");

    // Start the beacons
    runner.attach_test_task(start_beacon(commanders[0], 0, true));
    runner.attach_test_task(start_beacon(commanders[1], 1, false));

    runner.attach_test_task(async {
        // Perform the scan, actively
        let (mut scan_confirm, notifications) =
            perform_scan(commanders[2], ScanType::Active, &[0], true).await;

        // Scan needs to be successful
        assert_eq!(scan_confirm.status, Status::Success);
        // We should've scanned two devices since we've done an active scan
        assert_eq!(scan_confirm.result_list_size, 2);
        // All channels should be scanned
        assert!(scan_confirm.unscanned_channels.is_empty());

        // Auto request was true, so we should've gotten zero beacon notifications
        assert!(notifications.is_empty());

        let trace = aether.stop_trace();

        let mut messages = aether.parse_trace(trace);

        // We expect a beacon request and then only beacons
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

        pretty_assertions::assert_eq!(
            scan_confirm
                .pan_descriptor_list()
                .find(|pd| pd.coord_address == Address::Short(PanId(0), ShortAddress(0)))
                .unwrap(),
            &PanDescriptor {
                coord_address: lr_wpan_rs::wire::Address::Short(PanId(0), ShortAddress(0)),
                channel_number: 0,
                channel_page: ChannelPage::Uwb,
                super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                    beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(10),
                    superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(
                        10
                    ),
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

        let non_beacon_pan = scan_confirm
            .pan_descriptor_list_mut()
            .find(|pd| pd.coord_address == Address::Short(PanId(1), ShortAddress(1)))
            .unwrap();
        // We don't want to test the timestamp since that changes (even in simulation)
        non_beacon_pan.timestamp = Instant::from_seconds(0);

        pretty_assertions::assert_eq!(
            non_beacon_pan,
            &PanDescriptor {
                coord_address: lr_wpan_rs::wire::Address::Short(PanId(1), ShortAddress(1)),
                channel_number: 0,
                channel_page: ChannelPage::Uwb,
                super_frame_spec: lr_wpan_rs::wire::beacon::SuperframeSpecification {
                    beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::OnDemand,
                    superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::Inactive,
                    final_cap_slot: 0,
                    battery_life_extension: false,
                    pan_coordinator: true,
                    association_permit: false
                },
                gts_permit: false,
                link_quality: 255,
                timestamp: Instant::from_ticks(0),
                security_status: None,
                security_info: SecurityInfo::new_none_security(),
                code_list: ()
            }
        );
    });

    runner.run();
}

#[test_log::test]
fn scan_passive_no_auto_request() {
    // Goal is to scan without auto request which sends out the data as indications
    // The indications should be the same as what's being sent out on the aether

    let (commanders, mut aether, mut runner) = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    aether.start_trace("scan_passive_no_auto");

    // Start the beacons
    runner.attach_test_task(start_beacon(commanders[0], 0, true));
    runner.attach_test_task(start_beacon(commanders[1], 1, true));

    runner.attach_test_task(async {
        // Do the scan, passively, without auto request
        let (scan_confirm, notifications) =
            perform_scan(commanders[2], ScanType::Passive, &[0, 1, 2], false).await;

        // Scan must have succeeded
        assert_eq!(scan_confirm.status, Status::Success);
        // No list, since we should've gotten the info as indications
        assert_eq!(scan_confirm.result_list_size, 0);
        assert_eq!(scan_confirm.pan_descriptor_list().count(), 0);
        // All channels should have been scanned
        assert!(scan_confirm.unscanned_channels.is_empty());
        // Notifications must NOT be empty
        assert!(!notifications.is_empty());

        let trace = aether.stop_trace();

        // The notifications should follow the messages on the aether
        let messages = aether.parse_trace(trace);
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
    });

    runner.run();
}

// // TODO: A test with auto request enabled and more PANs being scanned than can fit in the allocation

async fn start_beacon(commander: &MacCommander, id: u16, emit_beacons: bool) {
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
            beacon_order: if emit_beacons {
                BeaconOrder::BeaconOrder(10)
            } else {
                BeaconOrder::OnDemand
            },
            superframe_order: if emit_beacons {
                SuperframeOrder::SuperframeOrder(10)
            } else {
                SuperframeOrder::Inactive
            },
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
        .await
        .status
        .unwrap();

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
