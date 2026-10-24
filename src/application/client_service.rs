//! Client service.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::sync::Mutex;

use super::{AppError, ClientStore};
use crate::domain::{Amount, ClientId, NewClient};

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

    /// Adds `amount` to the client balance and returns the new balance.
    pub async fn credit(&self, client_id: ClientId, amount: Amount) -> Result<Decimal, AppError> {
        let mut store = self.store.lock().await;
        let client = store.client_mut(client_id)?;
        client.credit(amount)?;

        Ok(client.balance)
    }

    /// Subtracts `amount` from the client balance and returns the new balance.
    pub async fn debit(&self, client_id: ClientId, amount: Amount) -> Result<Decimal, AppError> {
        let mut store = self.store.lock().await;
        let client = store.client_mut(client_id)?;
        client.debit(amount)?;

        Ok(client.balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppError;
    use crate::domain::ClientId;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;

    const CONCURRENT_REGISTRATIONS: usize = 50;
    const CONCURRENT_MOVEMENTS: usize = 100;

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

    fn amount(text: &str) -> Amount {
        Amount::try_from(Decimal::from_str(text).unwrap()).unwrap()
    }

    fn balance(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    async fn registered_client(service: &ClientService) -> ClientId {
        service
            .register(registration_with_document("1"), today())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn credit_then_debit_return_the_new_balance() {
        let service = ClientService::new();
        let client_id = registered_client(&service).await;

        let after_credit = service.credit(client_id, amount("100.50")).await;
        let after_debit = service.debit(client_id, amount("30")).await;

        assert_eq!(after_credit.unwrap(), balance("100.50"));
        assert_eq!(after_debit.unwrap(), balance("70.50"));
    }

    #[tokio::test]
    async fn credit_unknown_client_fails_with_client_not_found() {
        let service = ClientService::new();
        let unknown = ClientId::new(42);

        let error = service.credit(unknown, amount("1")).await;

        assert_eq!(error.unwrap_err(), AppError::ClientNotFound(unknown));
    }

    #[tokio::test]
    async fn debit_unknown_client_fails_with_client_not_found() {
        let service = ClientService::new();
        let unknown = ClientId::new(42);

        let error = service.debit(unknown, amount("1")).await;

        assert_eq!(error.unwrap_err(), AppError::ClientNotFound(unknown));
    }

    #[tokio::test]
    async fn credit_overflow_returns_balance_overflow_and_balance_unchanged() {
        let service = ClientService::new();
        let client_id = registered_client(&service).await;
        let near_max = Amount::try_from(Decimal::MAX).unwrap();
        service.credit(client_id, near_max).await.unwrap();

        let error = service.credit(client_id, amount("1")).await;
        let balance_after = service.debit(client_id, amount("1")).await.unwrap();

        assert_eq!(error.unwrap_err(), AppError::BalanceOverflow);
        assert_eq!(balance_after, Decimal::MAX - Decimal::ONE);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_credits_on_one_client_sum_exactly() {
        let service = Arc::new(ClientService::new());
        let client_id = registered_client(&service).await;

        let tasks: Vec<_> = (0..CONCURRENT_MOVEMENTS)
            .map(|_| {
                let service = Arc::clone(&service);
                tokio::spawn(async move { service.credit(client_id, amount("0.01")).await })
            })
            .collect();
        for task in tasks {
            task.await.unwrap().unwrap();
        }

        let final_balance = service.debit(client_id, amount("0.01")).await.unwrap();
        assert_eq!(final_balance, balance("0.99"));
    }
}
