use std::time::Duration;

use lr_wpan_rs::{
    pib::PibValue,
    sap::{reset::ResetRequest, set::SetRequest, start::StartRequest, SecurityInfo, Status},
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        FrameType, PanId, ShortAddress,
    },
    ChannelPage,
};

#[test_log::test(tokio::test(unhandled_panic = "shutdown_runtime", start_paused = true))]
async fn test_beacons_simple_pancoordinator() {
    let mut runner = lr_wpan_rs::test_helpers::run::run_mac_engine_simple();

    runner.aether.start_trace("beacons_after_start");

    let reset_response = runner
        .commander
        .request(ResetRequest {
            set_default_pib: true,
        })
        .await;
    assert_eq!(reset_response.status, Status::Success);

    let set_response = runner
        .commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_SHORT_ADDRESS,
            pib_attribute_value: PibValue::MacShortAddress(ShortAddress(0)),
        })
        .await;
    assert_eq!(set_response.status, Status::Success);

    let start_response = runner
        .commander
        .request(StartRequest {
            pan_id: PanId(1234),
            channel_number: 5,
            channel_page: ChannelPage::Uwb,
            start_time: 0,
            beacon_order: lr_wpan_rs::wire::beacon::BeaconOrder::BeaconOrder(14),
            superframe_order: lr_wpan_rs::wire::beacon::SuperframeOrder::SuperframeOrder(14),
            pan_coordinator: true,
            battery_life_extension: false,
            coord_realignment: false,
            coord_realign_security_info: SecurityInfo::new_none_security(),
            beacon_security_info: SecurityInfo::new_none_security(),
        })
        .await;
    assert_eq!(start_response.status, Status::Success);

    tokio::time::sleep(Duration::from_secs(10)).await;

    let trace = runner.aether.stop_trace();

    let mut seq: Option<u8> = None;
    for frame in runner.aether.parse_trace(trace) {
        println!("{frame:?}");
        assert_eq!(frame.header.frame_type, FrameType::Beacon);
        assert_eq!(
            frame.header.source,
            Some(lr_wpan_rs::wire::Address::Short(
                PanId(1234),
                ShortAddress(0)
            ))
        );

        if let Some(seq) = seq {
            assert_eq!(frame.header.seq, seq.wrapping_add(1));
        }
        seq = Some(frame.header.seq);

        match frame.content {
            lr_wpan_rs::wire::FrameContent::Beacon(beacon) => {
                assert_eq!(
                    beacon.superframe_spec.beacon_order,
                    BeaconOrder::BeaconOrder(14)
                );
                assert_eq!(
                    beacon.superframe_spec.superframe_order,
                    SuperframeOrder::SuperframeOrder(14)
                );
                assert!(beacon.superframe_spec.pan_coordinator)
            }
            _ => panic!("Wrong type"),
        }
    }
}
