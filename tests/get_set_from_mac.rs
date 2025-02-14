use ieee_802_15_4_mac::{
    mac::MacCommander,
    pib::PibValue,
    sap::{get::GetRequest, set::SetRequest, Status},
};

#[test_log::test(tokio::test(unhandled_panic = "shutdown_runtime"))]
async fn get_set() {
    let runner = ieee_802_15_4_mac::test_helpers::run::run_mac_engine_simple();

    test_get(runner.commander).await;
    test_set(runner.commander).await;
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
