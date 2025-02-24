use lr_wpan_rs::{
    mac::MacCommander,
    pib::PibValue,
    sap::{get::GetRequest, set::SetRequest, Status},
};

#[test_log::test]
fn get_set() {
    let (commanders, _, mut runner) = lr_wpan_rs_tests::run::run_mac_engine_multi(3);

    runner.attach_test_task(async {
        test_get(commanders[0]).await;
        test_set(commanders[0]).await;
    });

    runner.run();
}

async fn test_get(commander: &MacCommander) {
    let response = commander
        .request(GetRequest {
            pib_attribute: PibValue::MAC_AUTO_REQUEST,
        })
        .await;

    assert_eq!(response.pib_attribute, PibValue::MAC_AUTO_REQUEST);
    assert_eq!(response.status, Status::Success);
    assert!(matches!(response.value, PibValue::MacAutoRequest(_)));

    let response = commander
        .request(GetRequest {
            pib_attribute: "phyDoesNotExist",
        })
        .await;

    assert_eq!(response.pib_attribute, "phyDoesNotExist");
    assert_eq!(response.status, Status::UnsupportedAttribute);
    assert!(matches!(response.value, PibValue::None));
}

async fn test_set(commander: &MacCommander) {
    let response = commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_BATT_LIFE_EXT_PERIODS,
            pib_attribute_value: PibValue::MacBattLifeExtPeriods(8),
        })
        .await;

    assert_eq!(response.pib_attribute, PibValue::MAC_BATT_LIFE_EXT_PERIODS);
    assert_eq!(response.status, Status::Success);

    let response = commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_BATT_LIFE_EXT_PERIODS,
            pib_attribute_value: PibValue::MacBattLifeExtPeriods(0), // Below allowed range
        })
        .await;

    assert_eq!(response.pib_attribute, PibValue::MAC_BATT_LIFE_EXT_PERIODS);
    assert_eq!(response.status, Status::InvalidParameter);

    let response = commander
        .request(SetRequest {
            pib_attribute: PibValue::MAC_TIMESTAMP_SUPPORTED, // Read only
            pib_attribute_value: PibValue::MacTimestampSupported(false),
        })
        .await;

    assert_eq!(response.pib_attribute, PibValue::MAC_TIMESTAMP_SUPPORTED);
    assert_eq!(response.status, Status::ReadOnly);

    let response = commander
        .request(SetRequest {
            pib_attribute: "phyDoesNotExist",
            pib_attribute_value: PibValue::None,
        })
        .await;

    assert_eq!(response.pib_attribute, "phyDoesNotExist");
    assert_eq!(response.status, Status::UnsupportedAttribute);
}
