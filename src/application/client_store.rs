//! Client store.

use std::collections::HashMap;

use super::AppError;
use crate::domain::{Client, ClientId, DocumentNumber, NewClient};

/// First id given to a client.
const FIRST_CLIENT_ID: u64 = 1;

/// Every registered client, the document index and the id counter.
///
/// Not thread safe by itself: `ClientService` keeps it behind one lock so that the
/// document check and the insert happen together.
#[derive(Debug)]
pub struct ClientStore {
    clients: HashMap<ClientId, Client>,
    document_index: HashMap<DocumentNumber, ClientId>,
    next_client_id: u64,
}

impl Default for ClientStore {
    fn default() -> Self {
        Self {
            clients: HashMap::new(),
            document_index: HashMap::new(),
            next_client_id: FIRST_CLIENT_ID,
        }
    }
}

impl ClientStore {
    /// Stores a new client with the next id, unless its document is already registered.
    pub fn register(&mut self, new_client: NewClient) -> Result<ClientId, AppError> {
        if self
            .document_index
            .contains_key(&new_client.document_number)
        {
            return Err(AppError::DocumentAlreadyRegistered);
        }

        let client_id = ClientId::new(self.next_client_id);
        self.next_client_id += 1;
        self.document_index
            .insert(new_client.document_number.clone(), client_id);
        self.clients
            .insert(client_id, Client::open(client_id, new_client));

        Ok(client_id)
    }

    /// The client with this id, if any.
    pub fn find(&self, client_id: ClientId) -> Option<&Client> {
        self.clients.get(&client_id)
    }
}
