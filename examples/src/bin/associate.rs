#![no_std]
#![no_main]

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, peripherals};
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::Vec;
use lr_wpan_rs::allocation::Allocation;
use lr_wpan_rs::mac::{MacCommander, MacConfig};
use lr_wpan_rs::pib::PibValue;
use lr_wpan_rs::sap::associate::AssociateRequest;
use lr_wpan_rs::sap::get::GetRequest;
use lr_wpan_rs::sap::reset::ResetRequest;
use lr_wpan_rs::sap::scan::ScanRequest;
use lr_wpan_rs::sap::set::SetRequest;
use lr_wpan_rs::sap::SecurityInfo;
use lr_wpan_rs::wire::command::{AssociationStatus, CapabilityInformation};
use lr_wpan_rs::wire::{ExtendedAddress, ShortAddress};
use lr_wpan_rs::ChannelPage;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => embassy_stm32::rng::InterruptHandler<peripherals::RNG>;
});

static COMMANDER: MacCommander = MacCommander::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    info!("Hello World!");

    let p = embassy_stm32::init(Default::default());

    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);

    let spi = Spi::new_blocking(p.SPI3, p.PC10, p.PC12, p.PC11, spi_config);
    let cs = Output::new(p.PC13, Level::High, Speed::VeryHigh);
    let irq = ExtiInput::new(p.PC0, p.EXTI0, embassy_stm32::gpio::Pull::None);

    let radio = lr_wpan_rs_dw1000::DW1000Phy::new(
        unwrap!(ExclusiveDevice::new(spi, cs, embassy_time::Delay)),
        irq,
        embassy_time::Delay,
    )
    .await;

    let rng = Rng(embassy_stm32::rng::Rng::new(p.RNG, Irqs));

    lr_wpan_rs::mac::run_mac_engine(
        radio.unwrap(),
        &COMMANDER,
        MacConfig {
            extended_address: ExtendedAddress(0),
            rng,
            delay: embassy_time::Delay,
        },
    )
    .await;
}

#[embassy_executor::task]
async fn associate() {
    // Reset the device
    COMMANDER
        .request(ResetRequest {
            set_default_pib: true,
        })
        .await
        .status
        .unwrap();

    // Set macAutoRequest so we get a list of scanned beacons instead of indications
    COMMANDER
        .request(SetRequest {
            pib_attribute: PibValue::MAC_AUTO_REQUEST,
            pib_attribute_value: PibValue::MacAutoRequest(true),
        })
        .await
        .status
        .unwrap();

    // Scan for the PAN the coordinator is running
    let mut scan_allocation = [None; 1];
    let scan_confirm = COMMANDER
        .request_with_allocation(
            ScanRequest {
                scan_type: lr_wpan_rs::sap::scan::ScanType::Active,
                scan_channels: Vec::from_slice(&[0]).unwrap(),
                pan_descriptor_list: Allocation::new(),
                scan_duration: 14,
                channel_page: ChannelPage::Uwb,
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
    let associate_confirm = COMMANDER
        .request(AssociateRequest {
            channel_number: 0,
            channel_page: ChannelPage::Uwb,
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
        COMMANDER
            .request(GetRequest {
                pib_attribute: PibValue::MAC_SHORT_ADDRESS
            })
            .await
            .value,
        PibValue::MacShortAddress(ShortAddress(1))
    );
    assert_eq!(
        COMMANDER
            .request(GetRequest {
                pib_attribute: PibValue::MAC_COORD_SHORT_ADDRESS
            })
            .await
            .value,
        PibValue::MacCoordShortAddress(ShortAddress(0))
    );
    assert_eq!(
        COMMANDER
            .request(GetRequest {
                pib_attribute: PibValue::MAC_COORD_EXTENDED_ADDRESS
            })
            .await
            .value,
        PibValue::MacCoordExtendedAddress(ExtendedAddress::broadcast())
    );
    assert_eq!(
        COMMANDER
            .request(GetRequest {
                pib_attribute: PibValue::PHY_CURRENT_CHANNEL
            })
            .await
            .value,
        PibValue::PhyCurrentChannel(0)
    );
    assert_eq!(
        COMMANDER
            .request(GetRequest {
                pib_attribute: PibValue::PHY_CURRENT_PAGE
            })
            .await
            .value,
        PibValue::PhyCurrentPage(ChannelPage::Mhz868_915_2450)
    );
}

struct Rng(embassy_stm32::rng::Rng<'static, peripherals::RNG>);

impl rand_core::RngCore for Rng {
    fn next_u32(&mut self) -> u32 {
        use rand_core_06::RngCore;
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        use rand_core_06::RngCore;
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        use rand_core_06::RngCore;
        self.0.fill_bytes(dst)
    }
}
