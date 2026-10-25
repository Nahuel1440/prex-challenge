//! Use cases that orchestrate the domain over the in-memory store.

mod client_service;
mod client_store;
mod client_view;
mod error;

pub use client_service::{ClientRegistration, ClientService};
pub use client_store::ClientStore;
pub use client_view::ClientView;
pub use error::AppError;
