//! Credits that race with cut-offs are neither lost nor counted twice.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use chrono::NaiveDate;
use prex_processor::{
    application::{ClientRegistration, ClientService},
    domain::{Amount, ClientId},
    infrastructure::FsCutOffFileWriter,
};
use rust_decimal::Decimal;

const CLIENT_COUNT: u64 = 5;
const CREDITS_PER_CLIENT: u64 = 100;
const CUT_OFF_COUNT: usize = 20;
const CREDIT_AMOUNT: Decimal = Decimal::ONE;

type SharedService = Arc<ClientService<FsCutOffFileWriter>>;

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
            service
                .credit(client_id, Amount::try_from(CREDIT_AMOUNT).unwrap())
                .await
                .unwrap();
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

/// Sum of the balances of every line of every cut-off file in `dir`.
fn total_in_cut_off_files(dir: &Path) -> Decimal {
    fs::read_dir(dir)
        .unwrap()
        .flat_map(|entry| {
            let content = fs::read_to_string(entry.unwrap().path()).unwrap();
            content.lines().map(balance_of_line).collect::<Vec<_>>()
        })
        .sum()
}

/// The balance of a line `ID BALANCE`.
fn balance_of_line(line: &str) -> Decimal {
    let (_client_id, balance) = line.split_once(' ').unwrap();

    balance.parse().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_credits_during_cut_offs_are_neither_lost_nor_double_counted() {
    let dir = tempfile::tempdir().unwrap();
    let service: SharedService = Arc::new(ClientService::new(
        FsCutOffFileWriter::new(dir.path().to_owned()),
        1,
    ));
    let client_ids = register_clients(&service).await;

    let mut tasks: Vec<_> = client_ids
        .iter()
        .map(|&client_id| credit_client_repeatedly(&service, client_id))
        .collect();
    tasks.push(run_cut_offs(&service));
    for task in tasks {
        task.await.unwrap();
    }

    let mut balances_now = Decimal::ZERO;
    for client_id in client_ids {
        balances_now += service.find(client_id).await.unwrap().balance;
    }
    let total_credited = CREDIT_AMOUNT * Decimal::from(CLIENT_COUNT * CREDITS_PER_CLIENT);
    assert_eq!(
        total_in_cut_off_files(dir.path()) + balances_now,
        total_credited
    );
}
