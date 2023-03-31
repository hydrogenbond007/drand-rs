//! # drand-core
//!
//! drand-core is a library to retrieve public randomness generated by drand beacons. It features an HTTP client, and verification method.
//!
//! The format specification is at [drand.love/docs/specification](https://drand.love/docs/specification/). drand was designed in [Scalable Bias-Resistant Distributed Randomness](https://eprint.iacr.org/2016/1067.pdf).
//!
//! The reference interroperable Go implementation is available at [drand/drand](https://github.com/drand/drand).
//!
//! ## Usage
//!
//! ```rust
//! use drand_core::http_chain_client::HttpChainClient;
//!
//! #[tokio::main]
//! async fn main() {
//!   // Create a new client
//!   let client: HttpChainClient = "https://drand.cloudflare.com".try_into().unwrap();
//!   
//!   // Get the latest beacon. By default, it verifies its signature against the chain info.
//!   let beacon = client.latest().await.unwrap();
//!   
//!   // Print the beacon
//!   println!("{:?}", beacon);
//! }
//! ```

pub mod beacon;
mod bls_signatures;
pub mod chain;
pub mod http_chain_client;
