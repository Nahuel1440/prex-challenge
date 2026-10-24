//! Use cases that orchestrate the domain over the in-memory store.

mod client_service;
mod client_store;
mod client_view;
mod cut_off_file_writer;
mod error;

pub use client_service::{ClientRegistration, ClientService};
pub use client_store::ClientStore;
pub use client_view::ClientView;
pub use cut_off_file_writer::{CutOffFileWriter, CutOffLine};
pub use error::AppError;
