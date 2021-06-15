use anyhow::Result;
use fastly::http::{header, Method, StatusCode};
use fastly::{Request, Response};
use ipnet::{Ipv4Net, Ipv6Net};
use regex::Regex;
use std::net::{Ipv4Addr, Ipv6Addr};

const BACKEND_NAME: &str = "fastlyapi";

#[derive(serde::Deserialize, Debug)]
struct FastlyIpList {
    addresses: Vec<String>,
    ipv6_addresses: Vec<String>,
}

#[derive(serde::Serialize)]
struct IpCheckResult {
    is_fastly_ip: bool,
}

#[fastly::main]
fn main(req: Request) -> Result<Response> {
    // We can filter requests that have unexpected methods.
    const VALID_METHODS: [Method; 1] = [Method::GET];
    if !(VALID_METHODS.contains(req.get_method())) {
        return Ok(
            Response::from_status(StatusCode::NOT_FOUND).with_body("Only GET method is allowed")
        );
    }

    // Regular expression to check if a string is a IPv6 or IPv4 address
    let re_ip46 = Regex::new("^/((?:[0-9]{1,3}.){3}[0-9]{1,3}|(([a-f0-9:]+:+)+[a-f0-9]+))$")?;

    let path = req.get_path();
    if !re_ip46.is_match(path) {
        return Ok(
            Response::from_status(StatusCode::NOT_FOUND).with_body("Valid IP address not found")
        );
    }

    // Save the IP request from client
    let ip_addr = path[1..].to_owned();

    // Get IP list info
    let req_backend = Request::get("https://dummy/public-ip-list")
        .with_ttl(60 * 60 * 24 * 7)
        .with_header(header::HOST, "api.fastly.com");
    let mut resp = req_backend.send(BACKEND_NAME)?;

    let ip_list = resp.take_body_json::<FastlyIpList>()?;

    if let Ok(ipv4) = ip_addr.parse::<Ipv4Addr>() {
        for ipv4net in ip_list.addresses {
            let net: Ipv4Net = ipv4net.parse()?;
            if net.contains(&ipv4) {
                return Ok(Response::from_status(StatusCode::OK)
                    .with_body_json(&IpCheckResult { is_fastly_ip: true })?);
            }
        }
    } else if let Ok(ipv6) = ip_addr.parse::<Ipv6Addr>() {
        for ipv6net in ip_list.ipv6_addresses {
            let net: Ipv6Net = ipv6net.parse()?;
            if net.contains(&ipv6) {
                return Ok(Response::from_status(StatusCode::OK)
                    .with_body_json(&IpCheckResult { is_fastly_ip: true })?);
            }
        }
    } else {
        return Ok(
            Response::from_status(StatusCode::NOT_FOUND).with_body("Valid IP address not found")
        );
    }

    return Ok(
        Response::from_status(StatusCode::OK).with_body_json(&IpCheckResult {
            is_fastly_ip: false,
        })?,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipv4contains() {
        let net: Ipv4Net = "151.101.0.0/16".parse().unwrap();
        let ip_yes: Ipv4Addr = "151.101.0.1".parse().unwrap();
        let ip_no: Ipv4Addr = "151.102.0.0".parse().unwrap();

        assert!(net.contains(&ip_yes));
        assert!(!net.contains(&ip_no));
    }

    #[test]
    fn ipv6contains() {
        let net: Ipv6Net = "2a04:4e40::/32".parse().unwrap();
        let ip_yes: Ipv6Addr = "2a04:4e40:7f6:9100:f8c9:7bf6:e4f5:c5f3".parse().unwrap();
        let ip_no: Ipv6Addr = "240d:1a:7f6:9100:f8c9:7bf6:e4f5:c5f3".parse().unwrap();

        assert!(net.contains(&ip_yes));
        assert!(!net.contains(&ip_no));
    }
}
