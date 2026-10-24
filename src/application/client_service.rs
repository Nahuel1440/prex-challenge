//! Client service.

use chrono::NaiveDate;
use tokio::sync::Mutex;

use super::{AppError, ClientStore};
use crate::domain::{ClientId, NewClient};

/// The unvalidated data to register a client, as the issuer sent it.
#[derive(Debug, Clone)]
pub struct ClientRegistration {
    /// Raw `client_name`.
    pub client_name: String,
    /// Raw `birth_date`.
    pub birth_date: NaiveDate,
    /// Raw `document_number`.
    pub document_number: String,
    /// Raw `country`.
    pub country: String,
}

/// Runs the client use cases over one shared store.
#[derive(Debug, Default)]
pub struct ClientService {
    store: Mutex<ClientStore>,
}

impl ClientService {
    /// A service with no clients.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the registration (against the UTC date `today`) and stores the client.
    ///
    /// The duplicate check and the insert run under one lock.
    pub async fn register(
        &self,
        registration: ClientRegistration,
        today: NaiveDate,
    ) -> Result<ClientId, AppError> {
        let new_client = NewClient::parse(
            &registration.client_name,
            registration.birth_date,
            &registration.document_number,
            &registration.country,
            today,
        )?;

        self.store.lock().await.register(new_client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppError;
    use crate::domain::ClientId;
    use chrono::NaiveDate;
    use std::sync::Arc;

    const CONCURRENT_REGISTRATIONS: usize = 50;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    fn registration_with_document(document_number: &str) -> ClientRegistration {
        ClientRegistration {
            client_name: "Ana Pérez".to_owned(),
            birth_date: NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(),
            document_number: document_number.to_owned(),
            country: "ar".to_owned(),
        }
    }

    #[tokio::test]
    async fn register_assigns_consecutive_ids_1_2_3() {
        let service = ClientService::new();

        let first = service
            .register(registration_with_document("1"), today())
            .await;
        let second = service
            .register(registration_with_document("2"), today())
            .await;
        let third = service
            .register(registration_with_document("3"), today())
            .await;

        assert_eq!(
            [first.unwrap(), second.unwrap(), third.unwrap()],
            [ClientId::new(1), ClientId::new(2), ClientId::new(3)]
        );
    }

    #[tokio::test]
    async fn register_same_document_twice_fails_with_document_already_registered() {
        let service = ClientService::new();
        service
            .register(registration_with_document("123"), today())
            .await
            .unwrap();

        let error = service
            .register(registration_with_document("123"), today())
            .await;

        assert_eq!(error.unwrap_err(), AppError::DocumentAlreadyRegistered);
    }

    #[tokio::test]
    async fn register_rejects_same_document_in_a_different_format() {
        let service = ClientService::new();
        let first_id = service
            .register(registration_with_document("12.345.678"), today())
            .await
            .unwrap();

        let error = service
            .register(registration_with_document("12345678"), today())
            .await;

        assert_eq!(error.unwrap_err(), AppError::DocumentAlreadyRegistered);
        let store = service.store.lock().await;
        let stored = store.find(first_id).unwrap();
        assert_eq!(stored.profile.document_number.as_str(), "12345678");
        assert_eq!(stored.profile.country.as_str(), "AR");
    }

    #[tokio::test]
    async fn register_with_invalid_fields_returns_invalid_request_and_consumes_no_id() {
        let service = ClientService::new();
        let invalid = ClientRegistration {
            client_name: "  ".to_owned(),
            ..registration_with_document("1")
        };

        let error = service.register(invalid, today()).await.unwrap_err();
        let next_id = service
            .register(registration_with_document("1"), today())
            .await;

        assert!(matches!(error, AppError::InvalidRequest(_)));
        assert_eq!(next_id.unwrap(), ClientId::new(1));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_registrations_with_same_document_yield_exactly_one_success() {
        let service = Arc::new(ClientService::new());

        let tasks: Vec<_> = (0..CONCURRENT_REGISTRATIONS)
            .map(|_| {
                let service = Arc::clone(&service);
                tokio::spawn(async move {
                    service
                        .register(registration_with_document("12.345.678"), today())
                        .await
                })
            })
            .collect();
        let mut successes = 0;
        for task in tasks {
            successes += usize::from(task.await.unwrap().is_ok());
        }

        assert_eq!(successes, 1);
    }
}
