use core::str;
use embedded_svc::http::client::Client;
use esp_idf_svc::http::{
    client::{Configuration, EspHttpConnection},
    Method,
};
use serde::Serialize;
use serde_json::json;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

pub const LAMPORTS_PER_SOL: u32 = 1_000_000_000;

pub struct Http {
    sol_endpoint: String,
    http_client: Arc<Mutex<Client<EspHttpConnection>>>,
}

unsafe impl Send for Http {}

impl Http {
    pub fn init(endpoint: &str) -> Result<Self, Box<dyn Error>> {
        let connection = EspHttpConnection::new(&Configuration {
            timeout: Some(std::time::Duration::from_secs(30)),
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;
        let client = Client::wrap(connection);
        Ok(Self {
            sol_endpoint: endpoint.to_string(),
            http_client: Arc::new(Mutex::new(client)),
        })
    }

    pub fn http_request(
        &mut self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        payload: Option<&str>,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        let client = &mut self.http_client.lock().unwrap();
        let mut request = client.request(method, uri, &headers)?;
        if let Some(payload_str) = payload {
            request.write(payload_str.as_bytes())?;
        };
        let response = request.submit()?;
        let status = response.status();

        println!("Response code: {}\n", status);
        if !(200..=299).contains(&status) {
            return Err(format!("HTTP Error: Status code {}", status).into());
        }

        // read the response body in chunks
        let mut buf = [0_u8; 256]; // buffer for storing chunks
        let mut response_body = String::new(); // string to hold the full response
        let mut reader = response;
        loop {
            let size = reader.read(&mut buf)?; // read data into the buffer
            if size == 0 {
                break; // exit loop when no more data is available
            }
            response_body.push_str(str::from_utf8(&buf[..size])?); // append the chunk to the response body
        }
        println!("Raw response body: {}", response_body);
        // deserialize the response JSON
        let json_response: serde_json::Value = serde_json::from_str(&response_body)?;

        // result
        Ok(json_response.clone())
    }

    pub fn http_sol_request<Params>(
        &mut self,
        method: &str,
        params: Params,
    ) -> Result<serde_json::Value, Box<dyn Error>>
    where
        Params: Serialize,
    {
        let payload = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": [params]
        });

        let payload_str = serde_json::to_string(&payload)?;

        let headers = [
            ("Content-Type", "application/json"),
            ("Content-Length", &payload_str.len().to_string()),
        ];
        let endpoint = self.sol_endpoint.clone();
        let max_retries = 3;
        let mut attempts = 0;

        while attempts < max_retries {
            match self.http_request(Method::Post, &endpoint, &headers, Some(&payload_str)) {
                Ok(value) => return Ok(value["result"].clone()),
                Err(e) => {
                    attempts += 1;
                    println!("attempt {}/{} failed: {}", attempts, max_retries, e);
                    if attempts < max_retries {
                        std::thread::sleep(Duration::from_millis(1500));
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err("Unexpected failure after retries".into())
    }

    pub fn get_balance(&mut self, wallet: &str) -> Result<u64, Box<dyn Error>> {
        let method = "getBalance";
        match self.http_sol_request(method, wallet) {
            Ok(response) => {
                let balance = response["value"].as_u64().unwrap_or(0);
                Ok(balance)
            }
            Err(e) => {
                println!("Error occurred: {}", e);
                Ok(0)
            }
        }
    }

    pub fn get_tps(&mut self) -> Result<(u64, u64), Box<dyn Error>> {
        let method = "getRecentPerformanceSamples";

        match self.http_sol_request(method, 1) {
            Ok(rps) => {
                let rps_result = rps
                    .as_array()
                    .and_then(|array| array.get(0))
                    .ok_or("no performance samples found in the response")?;

                let num_tx = rps_result["numTransactions"].as_u64().unwrap_or(0);
                let slot = rps_result["slot"].as_u64().unwrap_or(0);
                let total_tx = num_tx / 60;
                Ok((slot, total_tx))
            }
            Err(e) => {
                println!("Error occurred: {}", e);
                Ok((0, 0))
            }
        }
    }

    pub fn get_solana_price(&mut self) -> Result<f64, Box<dyn Error>> {
        let headers = [("accept", "application/json")];
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
        match self.http_request(Method::Get, &url, &headers, None) {
            Ok(response) => {
                let sol_price = response["solana"]["usd"].as_f64().unwrap_or(0.0);
                Ok(sol_price)
            }
            Err(e) => {
                println!("Error occurred: {}", e);
                Ok(0.0)
            }
        }
    }

    pub fn utc_offset_time(&mut self) -> Result<(String, String), Box<dyn std::error::Error>> {
        let headers = [("accept", "application/json")];
        let url = "https://timeapi.io/api/time/current/zone?timeZone=America/Bogota";

        let max_retries = 3;
        let mut attempts = 0;

        while attempts < max_retries {
            match self.http_request(Method::Get, &url, &headers, None) {
                Ok(response) => {
                    let year = response["year"].as_i64().unwrap_or(0);
                    let month = response["month"].as_i64().unwrap_or(0);
                    let day = response["day"].as_i64().unwrap_or(0);
                    let hour = response["hour"].as_i64().unwrap_or(0);
                    let minute = response["minute"].as_i64().unwrap_or(0);
                    let seconds = response["seconds"].as_i64().unwrap_or(0);

                    let date_string = format!("{}-{:02}-{:02}", year, month, day);
                    let time_string = format!("{:02}:{:02}:{:02}", hour, minute, seconds);
                    return Ok((time_string, date_string));
                }
                Err(e) => {
                    attempts += 1;
                    println!("Attempt {}/{} failed: {}", attempts, max_retries, e);

                    if attempts < max_retries {
                        println!("retrying in 1 second...");
                        std::thread::sleep(Duration::from_millis(1500));
                    } else {
                        println!("all attempts failed.");
                        return Err(e);
                    }
                }
            }
        }

        Err("Unexpected failure after retries".into())
    }
}
