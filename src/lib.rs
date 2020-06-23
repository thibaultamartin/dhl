use crate::errors::ClientError::*;
use crate::errors::ClientResult;

use chrono::{DateTime,NaiveDateTime};
use chrono::offset::Utc;
use regex::Regex;
use serde::{Serialize, Deserialize, de};
use std::convert::TryFrom;
use surf::http;

mod errors;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Service {
    Freight,
    Express,
    ParcelDE,
    ParcelNL,
    ParcelPL,
    DSC,
    DGF,
    Ecommerce,
}

impl TryFrom<&str> for Service {
    type Error = &'static str;

    fn try_from(service: &str) -> Result<Self, Self::Error> {
        match service.trim() {
            "freight" => Ok(Service::Freight),
            "express" => Ok(Service::Express),
            "parcel-de" => Ok(Service::ParcelDE),
            "parcel-nl" => Ok(Service::ParcelNL),
            "dsc" => Ok(Service::DSC),
            "dgf" => Ok(Service::DGF),
            "ecommerce" => Ok(Service::Ecommerce),
            _ => Err("Not a valid service"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusCode {
    PreTransit,
    Transit,
    Delivered,
    Failure,
    Unknown,
}

impl TryFrom<&str> for StatusCode {
    type Error = &'static str;

    fn try_from(status_code: &str) -> Result<Self, Self::Error> {
        match status_code.trim() {
            "pre-transit" => Ok(StatusCode::PreTransit),
            "transit" => Ok(StatusCode::Transit),
            "delivered" => Ok(StatusCode::Delivered),
            "failure" => Ok(StatusCode::Failure),
            "unknown" => Ok(StatusCode::Unknown),
            _ => Err("Not a valid status code")
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub address: Address,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
    pub address_locality: Option<String>,
    pub street_address: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentEvent {
    #[serde(deserialize_with = "deserialize_dhl_datetime")]
    pub timestamp: DateTime<Utc>,
    pub location: Option<Place>,
    #[serde(deserialize_with = "deserialize_status_code")]
    #[serde(default)] // this fellow allows serde not to panick if the field is missing
    pub status_code: Option<StatusCode>,
    pub description: Option<String>,
    pub remark: Option<String>,
    pub next_steps: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentDetails {
    pub carrier: Option<Organization>,
    pub product: Option<Product>,
    pub receiver: Option<Person>,
    pub sender: Option<Person>,
    pub proof_of_delivery: ProofOfDelivery,
    pub total_number_of_pieces: u32,
    pub piece_ids: Vec<String>,
}


#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub description: String,
    pub product_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOfDelivery {
    pub document_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub family_name: String,
    pub given_name: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub description: String,
    pub organization_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    pub id: String,
    #[serde(deserialize_with = "deserialize_service")]
    pub service: Service,
    pub origin: Option<Place>,
    pub destination: Option<Place>,
    pub status: ShipmentEvent,
    #[serde(deserialize_with = "deserialize_dhl_date")]
    pub estimated_time_of_delivery: DateTime<Utc>,
    pub estimated_time_of_delivery_remark: Option<String>,
    pub details: ShipmentDetails,
    pub events: Vec<ShipmentEvent>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub shipments: Vec<Shipment>,
    pub possible_additional_shipments_url: Vec<String>,
}

fn deserialize_service<'de ,D>(deserializer: D) -> Result<Service, D::Error>
where
    D: de::Deserializer<'de>
{
    let service_str = String::deserialize(deserializer)?;

    Service::try_from(service_str.as_ref()).map_err(de::Error::custom)
}

fn deserialize_status_code<'de, D>(deserializer: D) -> Result<Option<StatusCode>, D::Error>
where
    D: de::Deserializer<'de>
{
    let status_code_str = String::deserialize(deserializer)?;

    if let Ok(code) = StatusCode::try_from(status_code_str.as_ref()) {
        return Ok(Some(code))
    } else {
        return Err(de::Error::custom("Could not parse status code"))
    }
}

fn deserialize_dhl_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>
{
    let date_str = String::deserialize(deserializer)?;

    let naive_date = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%dT%H:%M:%S");
    if let Ok(naive_date) = naive_date {
        return Ok(DateTime::<Utc>::from_utc(naive_date, Utc))
    }

    Err(de::Error::custom("Could not parse date"))
}

fn deserialize_dhl_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where D: de::Deserializer<'de>
{
    let mut date_str = String::deserialize(deserializer)?;
    date_str = format!("{} 00:00:01", date_str);

    let naive_date = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S");
    if let Ok(naive_date) = naive_date {
        return Ok(DateTime::<Utc>::from_utc(naive_date, Utc))
    }

    Err(de::Error::custom("Could not parse date"))
}

#[derive(Clone)]
pub struct TrackingNumber {
    tracking_number: String,
}

impl TryFrom<&str> for TrackingNumber {
    type Error = &'static str;

    fn try_from(tracking_number: &str) -> Result<Self, Self::Error> {
        let re = Regex::new(r"(^(\d{10})$)|(^(000|JJD01|JJD00|JVGL)\d+$)|(^(GM|LX|RX|[a-zA-Z]{5})\d+$)|(^(\d{10,39})$)|(^(3S|JVGL|JJD)[a-zA-Z0-9]+$)|(^\d{7}$)|(^\d[a-zA-Z]{2}\d{4,6}$)|(^[a-zA-Z]{3,4}\d+$)|(^\d{3}-\d{8}$)|(^[a-zA-Z]{2,3}-[a-zA-Z]{2,3}-\d{7}$)|(^\d{4}-\d{5}$)|(^\d{9,10}|\d{14}$)").unwrap();

        if !re.is_match(tracking_number) { return Err("Tracking Number did not match DHL format") }

        Ok(TrackingNumber { tracking_number: tracking_number.trim().to_string() })
    }
}

pub struct Client {
    api_key: String,
}

impl Client {
    pub fn new(api_key: &str) -> Client {
        Client { api_key: api_key.to_string() }
    }

    pub async fn get_shipments(&self, tracking_number: TrackingNumber) ->  ClientResult<Response> {
        let mut uri = "https://api-eu.dhl.com/track/shipments?trackingNumber=".to_string();
        uri.push_str(&tracking_number.tracking_number);

        let mut response = surf::get(uri)
            .set_header("Accept", "application/json")
            .set_header("DHL-API-KEY", &self.api_key)
            .await?;

        match response.status() {
            http::StatusCode::OK => {},
            http::StatusCode::UNAUTHORIZED => return Err(Unauthorized),
            http::StatusCode::NOT_FOUND => return Err(ParcelNotFound),
            _ => return Err(ServerError),
        }

        //println!("Response: {:?}", &response.body_string().await?);
        let res: Response = response.body_json().await?;
        Ok(res)
    }
}
