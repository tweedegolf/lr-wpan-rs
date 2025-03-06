use heapless::Vec;
use log::info;
use lr_wpan_rs::{
    allocation::Allocation,
    mac::MacCommander,
    pib::PibValue,
    sap::{
        associate::{AssociateIndication, AssociateRequest, AssociateResponse},
        get::GetRequest,
        reset::ResetRequest,
        scan::ScanRequest,
        set::SetRequest,
        start::StartRequest,
        IndicationValue, SecurityInfo,
    },
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        command::{AssociationStatus, CapabilityInformation},
        ExtendedAddress, PanId, ShortAddress,
    },
    ChannelPage,
};

#[test_log::test]
fn associate() {
    let (commanders, _, mut runner) = lr_wpan_rs_tests::run::create_test_runner(2);

    let pan_coordinator = commanders[0];
    let device = commanders[1];

    let (ready_sender, ready_receiver) = async_channel::bounded(1);
    runner.attach_test_task(run_pan_coordinator(pan_coordinator, ready_sender));

    // Run the device
    runner.attach_test_task(async move {
        // Reset the device
        device
            .request(ResetRequest {
                set_default_pib: true,
            })
            .await
            .status
            .unwrap();

        // Set macAutoRequest so we get a list of scanned beacons instead of indications
        device
            .request(SetRequest {
                pib_attribute: PibValue::MAC_AUTO_REQUEST,
                pib_attribute_value: PibValue::MacAutoRequest(true),
            })
            .await
            .status
            .unwrap();

        // Wait until coordinator is ready
        let _ = ready_receiver.recv().await;

        // Scan for the PAN the coordinator is running
        let mut scan_allocation = [None; 1];
        let scan_confirm = device
            .request_with_allocation(
                ScanRequest {
                    scan_type: lr_wpan_rs::sap::scan::ScanType::Active,
                    scan_channels: Vec::from_slice(&[0]).unwrap(),
                    pan_descriptor_list: Allocation::new(),
                    scan_duration: 14,
                    channel_page: ChannelPage::Mhz868_915_2450,
                    security_info: SecurityInfo::new_none_security(),
                },
                &mut scan_allocation,
            )
            .await;

        let scanned_coordinator = scan_confirm
            .pan_descriptor_list()
            .next()
            .expect("One PAN must have been found");

        // We've found the PAN, now associate with it
        let associate_confirm = device
            .request(AssociateRequest {
                channel_number: 0,
                channel_page: ChannelPage::Mhz868_915_2450,
                coord_address: scanned_coordinator.coord_address,
                capability_information: CapabilityInformation {
                    full_function_device: true,
                    mains_power: true,
                    idle_receive: true,
                    frame_protection: false,
                    allocate_address: true,
                },
                security_info: SecurityInfo::new_none_security(),
            })
            .await;

        // Now assert we got the answer we expect
        assert_eq!(associate_confirm.status, Ok(AssociationStatus::Successful));
        assert_eq!(associate_confirm.assoc_short_address, ShortAddress(1));

        // Assert the pib is as expected
        assert_eq!(
            device
                .request(GetRequest {
                    pib_attribute: PibValue::MAC_SHORT_ADDRESS
                })
                .await
                .value,
            PibValue::MacShortAddress(ShortAddress(1))
        );
        assert_eq!(
            device
                .request(GetRequest {
                    pib_attribute: PibValue::MAC_COORD_SHORT_ADDRESS
                })
                .await
                .value,
            PibValue::MacCoordShortAddress(ShortAddress(0))
        );
        assert_eq!(
            device
                .request(GetRequest {
                    pib_attribute: PibValue::MAC_COORD_EXTENDED_ADDRESS
                })
                .await
                .value,
            PibValue::MacCoordExtendedAddress(ExtendedAddress::broadcast())
        );
        assert_eq!(
            device
                .request(GetRequest {
                    pib_attribute: PibValue::PHY_CURRENT_CHANNEL
                })
                .await
                .value,
            PibValue::PhyCurrentChannel(0)
        );
        assert_eq!(
            device
                .request(GetRequest {
                    pib_attribute: PibValue::PHY_CURRENT_PAGE
                })
                .await
                .value,
            PibValue::PhyCurrentPage(ChannelPage::Mhz868_915_2450)
        );
    });

    runner.run();
}

async fn run_pan_coordinator(
    pan_coordinator: &MacCommander,
    ready_sender: async_channel::Sender<()>,
) {
    // Reset the coordinator
    pan_coordinator
        .request(ResetRequest {
            set_default_pib: true,
        })
        .await
        .status
        .unwrap();

    // Self assign the short address
    pan_coordinator
        .request(SetRequest {
            pib_attribute: PibValue::MAC_SHORT_ADDRESS,
            pib_attribute_value: PibValue::MacShortAddress(lr_wpan_rs::wire::ShortAddress(0)),
        })
        .await
        .status
        .unwrap();

    // We are open for association
    pan_coordinator
        .request(SetRequest {
            pib_attribute: PibValue::MAC_ASSOCIATION_PERMIT,
            pib_attribute_value: PibValue::MacAssociationPermit(true),
        })
        .await
        .status
        .unwrap();

    // Start the PAN without beacons enabled
    pan_coordinator
        .request(StartRequest {
            pan_id: PanId(0),
            channel_number: 0,
            channel_page: ChannelPage::Mhz868_915_2450,
            start_time: 0,
            beacon_order: BeaconOrder::OnDemand,
            superframe_order: SuperframeOrder::Inactive,
            pan_coordinator: true,
            battery_life_extension: false,
            coord_realignment: false,
            coord_realign_security_info: SecurityInfo::new_none_security(),
            beacon_security_info: SecurityInfo::new_none_security(),
        })
        .await
        .status
        .unwrap();

    // We've done our setup
    ready_sender.send(()).await.unwrap();

    // Wait for the association indication and respond/accept it
    let indication_responder = pan_coordinator.wait_for_indication().await;
    match indication_responder.indication {
        IndicationValue::Associate(_) => {
            let responder = indication_responder.into_concrete::<AssociateIndication>();

            info!("Got an associate indication: {:?}", responder.indication);

            let request_device_address = responder.indication.device_address;

            responder.respond(AssociateResponse {
                device_address: request_device_address,
                assoc_short_address: ShortAddress(1),
                status: lr_wpan_rs::wire::command::AssociationStatus::Successful,
                security_info: SecurityInfo::new_none_security(),
            });
        }
        indication => panic!("Got an unexpected indication: {indication:?}"),
    }

    info!("Running PAN coordinator is done");
}
