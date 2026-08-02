//! Thread-local timer registry for setTimeout / setInterval / clearTimeout / clearInterval.
//! Also manages the microtask queue for Promise integration.
//!
//! JsFunction is not Send, so we use thread_local storage. Boa's Context always
//! runs on the same thread, so this is safe.

use boa_engine::{Context, JsValue};
use boa_engine::object::builtins::JsFunction;
use std::cell::RefCell;
use std::collections::BTreeMap;
use tracing::warn;

/// A single pending timer entry.
struct TimerEntry {
    /// The JS callback to invoke.
    func: JsFunction,
    /// Arguments to pass to the callback.
    args: Vec<JsValue>,
    /// If Some(interval_ms), re-schedule after firing. None for one-shot setTimeout.
    interval_ms: Option<u64>,
}

thread_local! {
    /// Ordered timer map: (deadline_ms, timer_id) → TimerEntry.
    /// Using BTreeMap guarantees:
    ///   1. Timers are iterated in deadline order (earliest first).
    ///   2. For equal deadlines, lower timer_id fires first (insertion order).
    static PENDING_TIMERS: RefCell<BTreeMap<(u64, u32), TimerEntry>> =
        RefCell::new(BTreeMap::new());

    /// Monotonically increasing timer ID counter (starts at 1, never 0).
    static TIMER_ID_COUNTER: RefCell<u32> = RefCell::new(1);

    /// Pending userland microtask functions (queueMicrotask / polyfill Promise).
    static MICROTASK_QUEUE: RefCell<Vec<JsFunction>> = RefCell::new(Vec::new());
}

/// Returns milliseconds since the Unix epoch (wall clock).
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn next_timer_id() -> u32 {
    TIMER_ID_COUNTER.with(|c| {
        let id = *c.borrow();
        *c.borrow_mut() = id.wrapping_add(1).max(1);
        id
    })
}

/// Register a one-shot timeout. Returns the timer ID.
pub fn set_timeout(func: JsFunction, args: Vec<JsValue>, delay_ms: u64) -> u32 {
    let id = next_timer_id();
    let deadline = now_ms() + delay_ms;
    PENDING_TIMERS.with(|m| {
        m.borrow_mut().insert((deadline, id), TimerEntry {
            func,
            args,
            interval_ms: None,
        });
    });
    id
}

/// Register a repeating interval. Returns the timer ID.
pub fn set_interval(func: JsFunction, args: Vec<JsValue>, interval_ms: u64) -> u32 {
    let id = next_timer_id();
    let deadline = now_ms() + interval_ms;
    PENDING_TIMERS.with(|m| {
        m.borrow_mut().insert((deadline, id), TimerEntry {
            func,
            args,
            interval_ms: Some(interval_ms),
        });
    });
    id
}

/// Cancel a timer/interval by ID.
pub fn clear_timer(id: u32) {
    PENDING_TIMERS.with(|m| {
        // The key includes deadline which we don't know; scan all entries.
        let key = m.borrow().keys().find(|(_, tid)| *tid == id).cloned();
        if let Some(k) = key {
            m.borrow_mut().remove(&k);
        }
    });
}

/// Enqueue a JS function as a userland microtask.
pub fn enqueue_microtask(func: JsFunction) {
    MICROTASK_QUEUE.with(|q| q.borrow_mut().push(func));
}

/// Fire any timers whose deadline has passed.
/// Also drains Boa's internal job queue (native Promise microtasks) after each callback.
/// Returns the number of callbacks fired this tick.
pub fn tick_timers(ctx: &mut Context) -> usize {
    let now = now_ms();
    let mut fired = 0;

    // Collect expired keys. The BTreeMap is ordered by (deadline, id), so
    // we iterate in the correct FIFO order automatically.
    let expired_keys: Vec<(u64, u32)> = PENDING_TIMERS.with(|m| {
        m.borrow()
            .range(..=(now, u32::MAX))
            .map(|(k, _)| *k)
            .collect()
    });

    for key in expired_keys {
        let entry = PENDING_TIMERS.with(|m| m.borrow_mut().remove(&key));
        if let Some(entry) = entry {
            let args: Vec<JsValue> = entry.args.iter().cloned().collect();
            if let Err(e) = entry.func.call(&JsValue::undefined(), &args, ctx) {
                warn!("[Timer id={}] callback threw: {e}", key.1);
            }
            // Flush Boa's internal Promise/microtask job queue after each callback.
            ctx.run_jobs();
            fired += 1;

            // Re-register intervals.
            if let Some(interval_ms) = entry.interval_ms {
                let new_deadline = now + interval_ms;
                let func_clone = entry.func.clone();
                PENDING_TIMERS.with(|m| {
                    m.borrow_mut().insert((new_deadline, key.1), TimerEntry {
                        func: func_clone,
                        args: entry.args,
                        interval_ms: Some(interval_ms),
                    });
                });
            }
        }
    }

    fired
}

/// Drain the userland microtask queue (queueMicrotask / Promise polyfill callbacks).
/// Also calls `ctx.run_jobs()` to flush Boa's native Promise internals.
/// Loops until both queues are empty.
pub fn drain_microtasks(ctx: &mut Context) -> usize {
    let mut count = 0;
    loop {
        // 1. Flush Boa's internal job queue (native Promise).
        ctx.run_jobs();

        // 2. Drain our userland microtask queue.
        let funcs: Vec<JsFunction> = MICROTASK_QUEUE.with(|q| q.borrow_mut().drain(..).collect());
        if funcs.is_empty() { break; }

        for func in funcs {
            if let Err(e) = func.call(&JsValue::undefined(), &[], ctx) {
                warn!("[Microtask] threw: {e}");
            }
            count += 1;
            // Each callback might enqueue more microtasks — flush Boa's queue again.
            ctx.run_jobs();
        }
    }
    // Final Boa job flush in case the loop exited with pending native jobs.
    ctx.run_jobs();
    count
}

/// Returns true if there are any pending timers or microtasks.
pub fn has_pending_work() -> bool {
    let timers = PENDING_TIMERS.with(|m| !m.borrow().is_empty());
    let microtasks = MICROTASK_QUEUE.with(|q| !q.borrow().is_empty());
    timers || microtasks
}

/// Clear all pending timers and microtasks (e.g. on page navigation).
pub fn clear_all_timers() {
    PENDING_TIMERS.with(|m| m.borrow_mut().clear());
    MICROTASK_QUEUE.with(|q| q.borrow_mut().clear());
}
