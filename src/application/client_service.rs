//! Client service.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::sync::Mutex;

use super::cut_off_file_name::cut_off_file_name;
use super::{AppError, ClientStore, ClientView, CutOffFileWriter};
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
#[derive(Debug)]
pub struct ClientService<W> {
    store: Mutex<ClientStore>,
    writer: W,
}

impl<W: CutOffFileWriter> ClientService<W> {
    /// A service with no clients; the first cut-off file gets the number `next_file_counter`.
    pub fn new(writer: W, next_file_counter: u64) -> Self {
        Self {
            store: Mutex::new(ClientStore::new(next_file_counter)),
            writer,
        }
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

    /// A copy of the client taken under the lock, so a concurrent movement never tears it.
    ///
    /// Fails with `ClientNotFound` if the id was never registered.
    pub async fn find(&self, client_id: ClientId) -> Result<ClientView, AppError> {
        self.store
            .lock()
            .await
            .find(client_id)
            .map(ClientView::from)
            .ok_or(AppError::ClientNotFound(client_id))
    }

    /// Stores every balance in the cut-off file of `today` (UTC) and resets them to zero.
    ///
    /// Runs under the lock, with no timeout around the write. If the write fails, balances
    /// and file counter stay as they were, so a retry uses the same file name.
    pub async fn store_balances(&self, today: NaiveDate) -> Result<String, AppError> {
        let mut store = self.store.lock().await;
        let file_name = cut_off_file_name(today, store.next_file_counter());
        let lines = store.cut_off_lines();

        self.writer
            .write(&file_name, &lines)
            .await
            .map_err(|cause| {
                tracing::error!(%cause, %file_name, "cut-off file could not be written");
                AppError::CutOffFailed
            })?;
        store.complete_cut_off();

        Ok(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::fake_cut_off_file_writer::{FakeCutOffFileWriter, RecordedWrite};
    use crate::application::{AppError, CutOffLine};
    use crate::domain::ClientId;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;

    const CONCURRENT_REGISTRATIONS: usize = 50;
    const CONCURRENT_MOVEMENTS: usize = 100;

    type TestService = ClientService<FakeCutOffFileWriter>;

    fn service() -> TestService {
        service_starting_at_file_counter(1)
    }

    fn service_starting_at_file_counter(next_file_counter: u64) -> TestService {
        ClientService::new(FakeCutOffFileWriter::default(), next_file_counter)
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 10, 24)
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
        let service = service();

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
        let service = service();
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
        let service = service();
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
        let service = service();
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
        let service = Arc::new(service());

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

    fn line(client_id: u64, text: &str) -> CutOffLine {
        CutOffLine {
            client_id: ClientId::new(client_id),
            balance: balance(text),
        }
    }

    async fn registered_client(service: &TestService, document: &str) -> ClientId {
        service
            .register(registration_with_document(document), today())
            .await
            .unwrap()
    }

    async fn balance_of(service: &TestService, client_id: ClientId) -> Decimal {
        service.find(client_id).await.unwrap().balance
    }

    #[tokio::test]
    async fn credit_then_debit_return_the_new_balance() {
        let service = service();
        let client_id = registered_client(&service, "1").await;

        let after_credit = service.credit(client_id, amount("100.50")).await;
        let after_debit = service.debit(client_id, amount("30")).await;

        assert_eq!(after_credit.unwrap(), balance("100.50"));
        assert_eq!(after_debit.unwrap(), balance("70.50"));
    }

    #[tokio::test]
    async fn credit_unknown_client_fails_with_client_not_found() {
        let service = service();
        let unknown = ClientId::new(42);

        let error = service.credit(unknown, amount("1")).await;

        assert_eq!(error.unwrap_err(), AppError::ClientNotFound(unknown));
    }

    #[tokio::test]
    async fn debit_unknown_client_fails_with_client_not_found() {
        let service = service();
        let unknown = ClientId::new(42);

        let error = service.debit(unknown, amount("1")).await;

        assert_eq!(error.unwrap_err(), AppError::ClientNotFound(unknown));
    }

    #[tokio::test]
    async fn credit_overflow_returns_balance_overflow_and_balance_unchanged() {
        let service = service();
        let client_id = registered_client(&service, "1").await;
        let near_max = Amount::try_from(Decimal::MAX).unwrap();
        service.credit(client_id, near_max).await.unwrap();

        let error = service.credit(client_id, amount("1")).await;
        let balance_after = service.debit(client_id, amount("1")).await.unwrap();

        assert_eq!(error.unwrap_err(), AppError::BalanceOverflow);
        assert_eq!(balance_after, Decimal::MAX - Decimal::ONE);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_credits_on_one_client_sum_exactly() {
        let service = Arc::new(service());
        let client_id = registered_client(&service, "1").await;

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

    #[tokio::test]
    async fn find_returns_a_copy_of_the_normalized_client() {
        let service = service();
        let client_id = service
            .register(registration_with_document("12.345.678"), today())
            .await
            .unwrap();

        let view = service.find(client_id).await.unwrap();

        assert_eq!(
            view,
            ClientView {
                client_id,
                client_name: "Ana Pérez".to_owned(),
                birth_date: NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(),
                document_number: "12345678".to_owned(),
                country: "AR".to_owned(),
                balance: Decimal::ZERO,
            }
        );
    }

    #[tokio::test]
    async fn find_reflects_the_balance_after_credit_and_debit() {
        let service = service();
        let client_id = registered_client(&service, "1").await;
        service.credit(client_id, amount("100.50")).await.unwrap();
        service.debit(client_id, amount("30")).await.unwrap();

        let view = service.find(client_id).await.unwrap();

        assert_eq!(view.balance, balance("70.50"));
    }

    #[tokio::test]
    async fn find_unknown_client_fails_with_client_not_found() {
        let service = service();
        let unknown = ClientId::new(42);

        let error = service.find(unknown).await;

        assert_eq!(error.unwrap_err(), AppError::ClientNotFound(unknown));
    }

    #[tokio::test]
    async fn store_balances_names_the_file_from_the_utc_date_and_counter() {
        let service = service();

        let file_name = service.store_balances(date(2026, 10, 24)).await;

        assert_eq!(file_name.unwrap(), "24102026_1.DAT");
        assert_eq!(
            service.writer.recorded_writes()[0].file_name,
            "24102026_1.DAT"
        );
    }

    #[tokio::test]
    async fn store_balances_writes_one_line_per_client_sorted_by_client_id() {
        const CLIENTS: u64 = 20;
        let service = service();
        for number in 1..=CLIENTS {
            let client_id = registered_client(&service, &number.to_string()).await;
            service
                .credit(client_id, amount(&number.to_string()))
                .await
                .unwrap();
        }

        service.store_balances(today()).await.unwrap();

        let expected: Vec<CutOffLine> = (1..=CLIENTS)
            .map(|number| line(number, &number.to_string()))
            .collect();
        assert_eq!(service.writer.recorded_writes()[0].lines, expected);
    }

    #[tokio::test]
    async fn store_balances_resets_every_balance_to_zero_after_a_successful_write() {
        let service = service();
        let creditor = registered_client(&service, "1").await;
        let debtor = registered_client(&service, "2").await;
        service.credit(creditor, amount("500.12")).await.unwrap();
        service.debit(debtor, amount("3000")).await.unwrap();

        service.store_balances(today()).await.unwrap();

        assert_eq!(
            service.writer.recorded_writes()[0].lines,
            [line(1, "500.12"), line(2, "-3000")]
        );
        assert_eq!(balance_of(&service, creditor).await, Decimal::ZERO);
        assert_eq!(balance_of(&service, debtor).await, Decimal::ZERO);
    }

    #[tokio::test]
    async fn store_balances_increments_the_counter_after_a_successful_write() {
        let service = service();

        let first = service.store_balances(today()).await.unwrap();
        let second = service.store_balances(today()).await.unwrap();

        assert_eq!(first, "24102026_1.DAT");
        assert_eq!(second, "24102026_2.DAT");
    }

    #[tokio::test]
    async fn store_balances_with_no_clients_writes_an_empty_file() {
        let service = service();

        service.store_balances(today()).await.unwrap();

        assert_eq!(
            service.writer.recorded_writes(),
            [RecordedWrite {
                file_name: "24102026_1.DAT".to_owned(),
                lines: Vec::new(),
            }]
        );
    }

    #[tokio::test]
    async fn store_balances_failure_returns_cut_off_failed_and_keeps_every_balance() {
        let service = service();
        let client_id = registered_client(&service, "1").await;
        service.credit(client_id, amount("500.12")).await.unwrap();
        service.writer.fail_writes();

        let error = service.store_balances(today()).await;

        assert_eq!(error.unwrap_err(), AppError::CutOffFailed);
        assert_eq!(balance_of(&service, client_id).await, balance("500.12"));
    }

    #[tokio::test]
    async fn store_balances_failure_keeps_the_counter_so_the_next_success_reuses_it() {
        let service = service();
        service.writer.fail_writes();
        service.store_balances(today()).await.unwrap_err();
        service.writer.succeed_writes();

        let file_name = service.store_balances(today()).await;

        assert_eq!(file_name.unwrap(), "24102026_1.DAT");
    }

    #[tokio::test]
    async fn store_balances_follows_the_date_across_a_day_change() {
        let service = service();

        let last_day = service.store_balances(date(2026, 12, 31)).await;
        let first_day = service.store_balances(date(2027, 1, 1)).await;

        assert_eq!(last_day.unwrap(), "31122026_1.DAT");
        assert_eq!(first_day.unwrap(), "01012027_2.DAT");
    }

    #[tokio::test]
    async fn store_balances_starts_at_the_counter_given_at_construction() {
        let service = service_starting_at_file_counter(8);

        let file_name = service.store_balances(today()).await;

        assert_eq!(file_name.unwrap(), "24102026_8.DAT");
    }
}
