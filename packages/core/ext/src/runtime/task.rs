//! Runtime support for `Task.scope` and `Task::Scope#spawn`.
//!
//! Rust values which own resources must not be stored in Shiika objects
//! (they are allocated by GC and never dropped), so the states of scopes are
//! kept in a table and Shiika objects only have the id.
use super::{chiika_finish, make_poller};
use shiika_ffi::async_::{ChiikaCont, ChiikaEnv, ChiikaValue, ContFuture};
use shiika_ffi::core_class::SkFn;
use shiika_ffi::SkValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// Exit status of the process when a task panicked (same as that of Rust).
const EXIT_STATUS_PANIC: i32 = 101;

/// Lowered signature of a lambda function which takes no arguments.
#[allow(improper_ctypes_definitions)]
type Fn0Func =
    unsafe extern "C" fn(env: *mut ChiikaEnv, fn_obj: ChiikaValue, cont: ChiikaCont) -> ContFuture;

pub type ScopeId = i64;

struct Scope {
    /// Number of tasks which are spawned in this scope and not finished yet
    n_running: usize,
    /// Notified every time a task of this scope finishes
    task_finished: Arc<Notify>,
}

struct Table {
    last_id: ScopeId,
    scopes: HashMap<ScopeId, Scope>,
}

// Note: do not hold this lock across an await point.
static TABLE: Mutex<Option<Table>> = Mutex::new(None);

fn with_table<T>(f: impl FnOnce(&mut Table) -> T) -> T {
    let mut guard = TABLE.lock().unwrap();
    let table = guard.get_or_insert_with(|| Table {
        last_id: 0,
        scopes: HashMap::new(),
    });
    f(table)
}

/// Creates a new scope and returns its id.
pub fn create_scope() -> ScopeId {
    with_table(|table| {
        table.last_id += 1;
        let id = table.last_id;
        table.scopes.insert(
            id,
            Scope {
                n_running: 0,
                task_finished: Arc::new(Notify::new()),
            },
        );
        id
    })
}

/// Starts a new task which runs `f` (a `Fn0`) in the scope.
/// Returns false if the scope is already closed.
pub fn spawn(scope_id: ScopeId, f: SkFn) -> bool {
    let registered = with_table(|table| match table.scopes.get_mut(&scope_id) {
        Some(scope) => {
            scope.n_running += 1;
            true
        }
        None => false,
    });
    if !registered {
        return false;
    }
    let func: Fn0Func = unsafe { std::mem::transmute(f.func_ptr()) };
    let fn_obj = f.as_raw_u64();
    let poller = make_poller(move |env| unsafe { func(env, fn_obj, chiika_finish) });
    tokio::spawn(async move {
        // A panic in Shiika is not something to be caught. tokio catches a
        // panic in a task, so make sure the process exits in that case.
        let _guard = ExitOnPanic;
        poller.await;
        finish_task(scope_id);
    });
    true
}

fn finish_task(scope_id: ScopeId) {
    with_table(|table| {
        let scope = table
            .scopes
            .get_mut(&scope_id)
            .expect("[BUG] scope of a running task not found");
        scope.n_running -= 1;
        // `notify_one` stores a permit when `join_scope` is not waiting yet.
        scope.task_finished.notify_one();
    });
}

/// Waits until all the tasks in the scope are finished and then closes the
/// scope.
///
/// A task may spawn another task in the same scope while it is being joined;
/// such tasks are waited too.
pub async fn join_scope(scope_id: ScopeId) {
    loop {
        let task_finished = with_table(|table| {
            let scope = table
                .scopes
                .get(&scope_id)
                .expect("[BUG] scope to join not found");
            if scope.n_running == 0 {
                // Close the scope
                table.scopes.remove(&scope_id);
                None
            } else {
                Some(scope.task_finished.clone())
            }
        });
        match task_finished {
            Some(notify) => notify.notified().await,
            None => return,
        }
    }
}

struct ExitOnPanic;

impl Drop for ExitOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // The panic message is already printed by the panic hook.
            std::process::exit(EXIT_STATUS_PANIC);
        }
    }
}
