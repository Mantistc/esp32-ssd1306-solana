use embedded_svc::wifi::Configuration;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    wifi::{BlockingWifi, ClientConfiguration, EspWifi},
};
use log::info;

pub fn wifi(modem: Modem, ssid: &str, password: &str) -> BlockingWifi<EspWifi<'static>> {
    let sysloop = EspSystemEventLoop::take().expect("failed sysloop ownership take");
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), None).unwrap();
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop).unwrap();

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))
        .unwrap();

    info!("Starting wifi...");

    wifi.start().unwrap();

    info!("Connecting wifi...");

    // let networks = wifi.scan().unwrap();
    // for network in networks {
    //     info!("SSID: {}, RSSI: {}", network.ssid, network.signal_strength);
    // }

    let ap_infos = wifi.scan().unwrap();

    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .expect("Could not parse the given SSID into WiFi config"),
        password: password
            .try_into()
            .expect("Could not parse the given password into WiFi config"),
        channel,
        ..Default::default()
    }))
    .unwrap();

    info!("Connecting wifi...");

    match wifi.connect() {
        Ok(value) => {
            info!("success");
            value
        }
        Err(err) => {
            info!("Error: {}", err);
            panic!("Error on connect: {}", err);
        }
    };

    info!("Waiting for DHCP lease...");

    wifi.wait_netif_up().unwrap();

    let ip_info = wifi.wifi().sta_netif().get_ip_info().unwrap();

    info!("Wifi DHCP info: {:?}", ip_info);
    wifi
}
