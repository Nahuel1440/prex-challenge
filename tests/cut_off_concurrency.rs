//! Credits that race with cut-offs are neither lost nor counted twice.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::NaiveDate;
use prex_processor::{
    application::{ClientRegistration, ClientService, CutOffFileWriter, CutOffLine},
    domain::{Amount, ClientId},
    infrastructure::FsCutOffFileWriter,
};
use rust_decimal::Decimal;
use tokio::sync::Notify;

const CLIENT_COUNT: u64 = 5;
const CREDITS_PER_CLIENT: u64 = 100;
/// Cut-offs after the first one, which is held in flight.
const CUT_OFF_COUNT: usize = 19;
const CREDIT_AMOUNT: Decimal = Decimal::ONE;

type SharedService = Arc<ClientService<GatedWriter>>;

/// Holds the first write until the test lets it go, so credits arrive while it is in flight.
struct GatedWriter {
    first_write_started: Arc<Notify>,
    first_write_released: Arc<Notify>,
    first_write_taken: AtomicBool,
    inner: FsCutOffFileWriter,
}

impl CutOffFileWriter for GatedWriter {
    async fn write(&self, file_name: &str, lines: &[CutOffLine]) -> std::io::Result<()> {
        if !self.first_write_taken.swap(true, Ordering::SeqCst) {
            self.first_write_started.notify_one();
            self.first_write_released.notified().await;
        }

        self.inner.write(file_name, lines).await
    }
}

fn credit_amount() -> Amount {
    Amount::try_from(CREDIT_AMOUNT).unwrap()
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
}

async fn register_clients(service: &SharedService) -> Vec<ClientId> {
    let mut client_ids = Vec::new();
    for number in 0..CLIENT_COUNT {
        let registration = ClientRegistration {
            client_name: format!("Client {number}"),
            birth_date: NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(),
            document_number: format!("1000000{number}"),
            country: "AR".to_owned(),
        };
        client_ids.push(service.register(registration, today()).await.unwrap());
    }

    client_ids
}

fn credit_client_repeatedly(
    service: &SharedService,
    client_id: ClientId,
) -> tokio::task::JoinHandle<()> {
    let service = service.clone();

    tokio::spawn(async move {
        for _ in 0..CREDITS_PER_CLIENT {
            service.credit(client_id, credit_amount()).await.unwrap();
            tokio::task::yield_now().await;
        }
    })
}

fn run_cut_offs(service: &SharedService) -> tokio::task::JoinHandle<()> {
    let service = service.clone();

    tokio::spawn(async move {
        for _ in 0..CUT_OFF_COUNT {
            service.store_balances(today()).await.unwrap();
            tokio::task::yield_now().await;
        }
    })
}

/// The sum of the balances of each cut-off file in `dir`.
fn totals_of_cut_off_files(dir: &Path) -> Vec<Decimal> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let content = fs::read_to_string(entry.unwrap().path()).unwrap();
            content.lines().map(balance_of_line).sum()
        })
        .collect()
}

/// The balance of a line `ID BALANCE`.
fn balance_of_line(line: &str) -> Decimal {
    let (_client_id, balance) = line.split_once(' ').unwrap();

    balance.parse().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_credits_during_cut_offs_are_neither_lost_nor_double_counted() {
    let dir = tempfile::tempdir().unwrap();
    let first_write_started = Arc::new(Notify::new());
    let first_write_released = Arc::new(Notify::new());
    let service: SharedService = Arc::new(ClientService::new(
        GatedWriter {
            first_write_started: first_write_started.clone(),
            first_write_released: first_write_released.clone(),
            first_write_taken: AtomicBool::new(false),
            inner: FsCutOffFileWriter::new(dir.path().to_owned()),
        },
        1,
    ));
    let client_ids = register_clients(&service).await;
    for &client_id in &client_ids {
        service.credit(client_id, credit_amount()).await.unwrap();
    }

    let held_cut_off = tokio::spawn({
        let service = service.clone();
        async move { service.store_balances(today()).await.unwrap() }
    });
    first_write_started.notified().await;
    let mut tasks: Vec<_> = client_ids
        .iter()
        .map(|&client_id| credit_client_repeatedly(&service, client_id))
        .collect();
    first_write_released.notify_one();
    held_cut_off.await.unwrap();
    tasks.push(run_cut_offs(&service));
    for task in tasks {
        task.await.unwrap();
    }

    let mut balances_now = Decimal::ZERO;
    for client_id in client_ids {
        balances_now += service.find(client_id).await.unwrap().balance;
    }
    let file_totals = totals_of_cut_off_files(dir.path());
    let total_credited = CREDIT_AMOUNT * Decimal::from(CLIENT_COUNT * (1 + CREDITS_PER_CLIENT));
    assert_eq!(
        file_totals.iter().sum::<Decimal>() + balances_now,
        total_credited
    );
    assert!(file_totals.iter().any(|total| *total > Decimal::ZERO));
}
