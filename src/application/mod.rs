//! Use cases that orchestrate the domain over the in-memory store.

mod client_service;
mod client_store;
mod error;

pub use client_service::{ClientRegistration, ClientService};
pub use client_store::ClientStore;
pub use error::AppError;
