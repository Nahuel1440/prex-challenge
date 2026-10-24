//! Client store.

use std::collections::HashMap;

use super::{AppError, CutOffLine};
use crate::domain::{Client, ClientId, DocumentNumber, NewClient};

/// First id given to a client.
const FIRST_CLIENT_ID: u64 = 1;

/// Every registered client, the document index and the id and file counters.
///
/// Not thread safe by itself: `ClientService` keeps it behind one lock so that the
/// document check and the insert happen together.
#[derive(Debug)]
pub struct ClientStore {
    clients: HashMap<ClientId, Client>,
    document_index: HashMap<DocumentNumber, ClientId>,
    next_client_id: u64,
    next_file_counter: u64,
}

impl ClientStore {
    /// An empty store whose first cut-off file gets the number `next_file_counter`.
    pub fn new(next_file_counter: u64) -> Self {
        Self {
            clients: HashMap::new(),
            document_index: HashMap::new(),
            next_client_id: FIRST_CLIENT_ID,
            next_file_counter,
        }
    }

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

    /// The client with this id, open for a balance change.
    pub fn client_mut(&mut self, client_id: ClientId) -> Result<&mut Client, AppError> {
        self.clients
            .get_mut(&client_id)
            .ok_or(AppError::ClientNotFound(client_id))
    }

    /// The client with this id, if any.
    pub fn find(&self, client_id: ClientId) -> Option<&Client> {
        self.clients.get(&client_id)
    }

    /// Number of the next cut-off file.
    pub fn next_file_counter(&self) -> u64 {
        self.next_file_counter
    }

    /// The balance of every client, sorted by client id.
    pub fn cut_off_lines(&self) -> Vec<CutOffLine> {
        let mut lines: Vec<CutOffLine> = self
            .clients
            .values()
            .map(|client| CutOffLine {
                client_id: client.id,
                balance: client.balance,
            })
            .collect();
        lines.sort_by_key(|line| line.client_id.value());

        lines
    }

    /// Closes a cut-off whose file was written: every balance back to zero, next file number.
    pub fn complete_cut_off(&mut self) {
        self.clients.values_mut().for_each(Client::reset_balance);
        self.next_file_counter += 1;
    }
}
