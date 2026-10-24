# prex-challenge

A small payment processor in Rust (actix-web). An issuer registers clients, posts credits and debits, reads balances, and runs a cut-off that writes every balance to a file and resets them to 0.

Balances live in memory only. Read [Limitations](#limitations) before you rely on it.

## Quick start

Needs Rust 1.89 or newer.

```bash
cargo run --release
curl http://127.0.0.1:8080/health          # {"status":"ok"}
```

With Docker:

```bash
docker build -t prex-challenge .
docker run --rm -p 8080:8080 -v prex-cut-off:/data/cut_off_files prex-challenge
```

The image runs as a non-root user (uid 10001), listens on `0.0.0.0:8080` and writes cut-off files to the volume `/data/cut_off_files`. With a bind mount, the host folder must be writable by uid 10001.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | Interface to bind. The image sets `0.0.0.0`. |
| `PORT` | `8080` | TCP port. A value that is not a port number stops the start with exit code 1. |
| `CUT_OFF_DIR` | `./cut_off_files` | Folder for cut-off files. Created at start. The image sets `/data/cut_off_files`. |
| `RUST_LOG` | `info` | Log filter (`tracing`). Every request is logged. |

On a stop signal (SIGINT, SIGTERM) the server stops accepting requests and gives in-flight ones 10 seconds to finish.

## API

Paths are the ones in the challenge PDF. Every answer is JSON. Money is a JSON number, never a string.

```bash
H='Content-Type: application/json'

# Register a client. The document is normalized (dots, dashes, spaces removed, uppercase)
# and the country is uppercased.
curl -X POST localhost:8080/new_client -H "$H" \
  -d '{"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"12.345.678","country":"ar"}'
# 201 {"client_id":1}

curl -X POST localhost:8080/new_credit_transaction -H "$H" -d '{"client_id":1,"credit_amount":100.50}'
# 200 {"client_id":1,"balance":100.50}

curl -X POST localhost:8080/new_debit_transaction -H "$H" -d '{"client_id":1,"debit_amount":30}'
# 200 {"client_id":1,"balance":70.50}

curl 'localhost:8080/client_balance?user_id=1'
# 200 {"client_id":1,"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"12345678","country":"AR","balance":70.50}

curl -X POST localhost:8080/store_balances
# 200 {"file_name":"24102026_1.DAT"}   the file now holds the line "1 70.5"
```

| Endpoint | Success | Notes |
| --- | --- | --- |
| `POST /new_client` | 201 `{client_id}` | Ids are integers from 1, in registration order. |
| `POST /new_credit_transaction` | 200 `{client_id, balance}` | `credit_amount` greater than 0, at most 2 decimals. |
| `POST /new_debit_transaction` | 200 `{client_id, balance}` | `debit_amount` as above. The balance may go negative. |
| `GET /client_balance?user_id=N` | 200 profile and `balance` | `user_id` is the id from `new_client`. |
| `POST /store_balances` | 200 `{file_name}` | No body. Runs the cut-off. |
| `GET /health` | 200 `{status}` | Not in the PDF. For probes and CI. |

### Client rules

| Field | Rule |
| --- | --- |
| `client_name` | Trimmed, 1 to 100 characters, any Unicode. |
| `birth_date` | `YYYY-MM-DD`, not after today (UTC). |
| `document_number` | `.`, `-` and whitespace removed, uppercased, only `A-Z` and `0-9` left. Unique after that: `12.345.678` and `12345678` clash. |
| `country` | Trimmed, uppercased, must be an ISO 3166-1 alpha-2 code. |

### Error contract

Every error has the same body: `{"code": "CLIENT_NOT_FOUND", "message": "client 42 does not exist"}`. `code` is stable for machines. `message` is for people.

| Case | Status | `code` |
| --- | --- | --- |
| Malformed body, wrong field type, invalid client field, missing or non-integer `user_id` | 400 | `INVALID_REQUEST` |
| Amount is 0, negative or has more than 2 decimals | 400 | `INVALID_AMOUNT` |
| Client id does not exist | 404 | `CLIENT_NOT_FOUND` |
| Document already registered | 409 | `DOCUMENT_ALREADY_REGISTERED` |
| Movement would overflow the balance (beyond the `Decimal` range) | 422 | `BALANCE_OVERFLOW` |
| Cut-off file could not be written | 500 | `CUT_OFF_FAILED` |

Things that surprise people:

- `INVALID_AMOUNT` is checked before `CLIENT_NOT_FOUND`. A bad amount for an unknown client gives 400, not 404.
- Exponent notation is not an amount. `1e2` gives `INVALID_REQUEST`, and so does an amount sent as a JSON string (`"100.50"`).
- More than 2 decimals is rejected, never rounded.
- A 500 never shows paths or OS errors. The cause goes to the log.
- Unknown routes and wrong methods get Actix's default answer, not this contract.

## Run the checks

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --locked
```

End-to-end, against a running server (local binary or container):

| Tool | Command |
| --- | --- |
| Postman collection with newman | `npx --yes newman@6 run postman/prex-challenge.postman_collection.json --env-var base_url=http://127.0.0.1:8080` |
| Shell smoke test | `BASE_URL=http://127.0.0.1:8080 scripts/smoke.sh` |
| Cut-off file format check | `scripts/check_cut_off_files.sh <cut_off_dir>` |

`smoke.sh` runs the file check too when `CUT_OFF_DIR` points at the server's folder. For a server in a container, copy the files out with `docker cp` and run the check on them.

To use the collection in Postman, import `postman/prex-challenge.postman_collection.json` and run it with the Collection Runner. Folders go in order, one per endpoint, and later requests use `client_id` and `file_counter` saved by earlier ones. Change the `base_url` collection variable to target another host.

CI (GitHub Actions) runs on every pull request:

| Job | What it does |
| --- | --- |
| `check` | `fmt`, `clippy -D warnings`, `cargo test --locked` |
| `msrv` | `cargo test --locked` on Rust 1.89 |
| `e2e` | Release binary, then newman, `smoke.sh` and the file check |
| `docker` | `docker build` of the `Dockerfile` (no push) |

## Design

One crate, four layers as modules:

| Module | Owns |
| --- | --- |
| `domain` | Client, amount and field rules. Pure: no Actix, Tokio or files. |
| `application` | `ClientService` (use cases), `ClientStore` (the state), the `CutOffFileWriter` port, `AppError`. |
| `infrastructure` | `FsCutOffFileWriter` and the startup scan of the cut-off folder. |
| `http` | Handlers, request and response shapes, the mapping from `AppError` to status and `code`. |

`main` reads the settings, wires the layers and starts the server.

### State and concurrency

- One `tokio::sync::Mutex` guards the whole store: clients, the document index, the id counter and the file counter. Every use case takes it once.
- Reason: the cut-off needs one consistent snapshot, and "does the document exist? then insert" and the file counter become atomic without extra code. Two registrations with one document give one 201 and one 409.
- Cost: all operations take turns, even on different clients. A per-client lock or a read-write lock would add complexity with no throughput requirement to justify it.
- Tokio's mutex, not the standard one, because the cut-off holds the lock across an `.await` (the file write).

### Money

- Amounts are `rust_decimal::Decimal`, parsed from the JSON text straight into a `Decimal`. A float would silently change `12345678901234567.89`.
- At most 2 decimals, greater than 0. Credit and debit use checked arithmetic, so a result outside the `Decimal` range is `BALANCE_OVERFLOW`.
- A balance is the net position since the last cut-off, not available funds, so it may be negative.

### Cut-off

Everything below happens under the lock, so each movement lands in exactly one cut-off file, none lost and none counted twice:

1. Build the name `DDMMYYYY_N.DAT` from the UTC date and the file counter.
2. Collect one line per client, sorted by id: `{id} {balance}`. The id is a plain integer and the balance has no trailing zeros (`1 70.5`, `3 -3000`). No clients gives an empty file.
3. Write the file (next section).
4. Only on success: reset every balance to 0 and advance the counter. On failure nothing changes and the answer is `CUT_OFF_FAILED`.

The handler runs the cut-off in its own Tokio task and waits for it. If the caller disconnects, Actix drops the handler, but the task still finishes. Otherwise the file could be published and the balances never reset, so the next cut-off would count them again. A task that ends without a result (a panic) is reported as `CUT_OFF_FAILED` and logged.

There is no timeout around the write. A timeout cancels the future but not the blocking write, so the file could appear after a failure was reported, and its money would be counted twice. Cost: a hung disk blocks every operation while the lock is held.

### How the file is written

`FsCutOffFileWriter` never shows a half-written file and never overwrites one:

1. Write the content to `.tmp-<name>.tmp` in the same folder, created with `create_new`, and `fsync` it. That name can never match `DDMMYYYY_N.DAT`.
2. `hard_link` the temp file to the final name. A hard link fails with `AlreadyExists` if the name is taken. A `rename` would silently replace an existing file.
3. `fsync` the folder, so the new name is durable before the balances reset.
4. Remove the temp file. After the link the file is published, so a failed removal is only a warning and the write still returns success. Otherwise the balances would stay in memory although they are in a published file.
5. On any failure before the link, the temp file is removed. At start, temp files left by a crash are removed too.

This is atomic publication, on filesystems that support hard links. It is not a transaction between memory and disk: a crash after the link and before the reset loses the memory anyway (see Limitations).

### File name and counter

- The date is the UTC date, so the name does not depend on the server timezone.
- At start, the service scans `CUT_OFF_DIR` for names like `DDMMYYYY_N.DAT` (any date) and starts at the highest `N` plus 1, or at 1 for an empty folder. Temp files and other names are ignored.
- This is a small step away from the challenge text ("since the service started"). It protects old files after a restart.
- A failed cut-off does not use a number, so there is no gap in the sequence.

## Limitations

Accepted on purpose, to keep the scope to what the challenge asks.

- **State is lost on restart.** Clients, balances and the id counter live in memory. A stop loses every client and every movement since the last cut-off. The files are results; the service never reads them back (only their names, for the counter).
- **Ids repeat across runs.** `client_id` is an integer counter from 1 on every start, as the PDF types it. Ids in files of different runs can mean different clients.
- **Single instance.** A second instance would hold a different set of clients and balances.
- **Cut-offs block everything.** Every operation waits while the file is written, and the wait grows with the number of clients. There is no timeout (see Cut-off).
- **File line format is an assumption.** The PDF does not define padding, so ids are plain (`1`, not `01`) and balances are normalized (`70.5`, not `70.50`).
- **Counter continues from the folder.** See File name and counter.
- **UTC date.** An issuer in UTC-3 that calls after 21:00 local time gets the next day in the file name. The date is read when the request arrives, before it waits for the lock, so a cut-off queued behind a slow one across UTC midnight can carry the previous day.
- **A taken name blocks the cut-off.** If the next file name already exists on disk (for example a file copied in by hand), every retry fails with `CUT_OFF_FAILED` until an operator moves the file. A retry uses the same name by design, so nothing is overwritten. The log tells this case apart.
- **Uniqueness uses the document alone**, as the PDF says.
- **No authentication**, and no handling of unknown routes or methods beyond Actix defaults.

## Production next steps

| Step | Why |
| --- | --- |
| Journal: append each client and movement to a file and `fsync` before answering, replay on start | Survives a restart. At the cut-off, rewrite the journal as a snapshot of every client (balance 0) and tag each confirmed cut-off with a sequence number, so a crash after the file but before the rewrite does not replay movements already stored. |
| Persistent id sequence (for example a database sequence) | Ids unique across restarts. |
| Key clients by `(country, document_number)` | Two countries can issue the same number. |
| Swap-and-merge cut-off | Swap the balances out under the lock and write the file outside it, so movements do not wait for the disk. On failure, merge back by adding. Needed if the file goes to slow or remote storage. |
| Precision per currency | The 2 decimals are a simple convention. |
| Authentication, metrics, more than one instance | Out of scope here. |
