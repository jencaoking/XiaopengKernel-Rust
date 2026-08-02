//! Timer infrastructure for setTimeout / setInterval / clearTimeout / clearInterval.
//!
//! Design: since JsFunction is not Send, we store pending timers in a thread_local.
//! BrowserEngine calls `tick_timers(ctx, now_ms)` once per event-loop turn,
//! which fires any callbacks whose deadline has passed.

use boa_engine::{Context, JsValue};
use boa_engine::object::builtins::JsFunction;
use std::cell::RefCell;
use std::collections::HashMap;
use tracing::{info, warn};

/// A single pending timer entry.
struct TimerEntry {
    /// Absolute deadline in milliseconds (from engine epoch).
    deadline_ms: u64,
    /// The JS callback to invoke.
    func: JsFunction,
    /// Arguments to pass to the callback (usually empty for timers).
    args: Vec<JsValue>,
    /// If Some(interval_ms), re-schedule after firing. None for one-shot setTimeout.
    interval_ms: Option<u64>,
}

thread_local! {
    /// timer_id → TimerEntry
    static PENDING_TIMERS: RefCell<HashMap<u32, TimerEntry>> = RefCell::new(HashMap::new());

    /// Monotonically increasing timer ID counter.
    static TIMER_ID_COUNTER: RefCell<u32> = RefCell::new(1);

    /// Engine epoch: timestamp (ms) at which the first timer was created, used
    /// to convert relative delays into absolute deadlines. We store Option so we
    /// can lazy-init it on the first timer registration.
    static ENGINE_START_MS: RefCell<Option<u64>> = RefCell::new(None);

    /// Pending microtask functions (for Promise integration).
    static MICROTASK_QUEUE: RefCell<Vec<JsFunction>> = RefCell::new(Vec::new());
}

/// Returns milliseconds since some fixed epoch (wall clock).
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
        *c.borrow_mut() = id.wrapping_add(1).max(1); // never return 0
        id
    })
}

/// Register a new timer. Returns its timer ID.
pub fn set_timeout(func: JsFunction, args: Vec<JsValue>, delay_ms: u64) -> u32 {
    let id = next_timer_id();
    let deadline = now_ms() + delay_ms;
    PENDING_TIMERS.with(|m| {
        m.borrow_mut().insert(id, TimerEntry {
            deadline_ms: deadline,
            func,
            args,
            interval_ms: None,
        });
    });
    id
}

/// Register a repeating interval. Returns its timer ID.
pub fn set_interval(func: JsFunction, args: Vec<JsValue>, interval_ms: u64) -> u32 {
    let id = next_timer_id();
    let deadline = now_ms() + interval_ms;
    PENDING_TIMERS.with(|m| {
        m.borrow_mut().insert(id, TimerEntry {
            deadline_ms: deadline,
            func,
            args,
            interval_ms: Some(interval_ms),
        });
    });
    id
}

/// Cancel a timer/interval by ID.
pub fn clear_timer(id: u32) {
    PENDING_TIMERS.with(|m| { m.borrow_mut().remove(&id); });
}

/// Enqueue a JS function as a microtask (called from the Promise polyfill).
pub fn enqueue_microtask(func: JsFunction) {
    MICROTASK_QUEUE.with(|q| q.borrow_mut().push(func));
}

/// Check for expired timers and invoke their callbacks.
/// Returns the number of callbacks that were fired this tick.
pub fn tick_timers(ctx: &mut Context) -> usize {
    let now = now_ms();
    let mut fired = 0;

    // Collect expired timer IDs without holding the borrow while calling into Boa.
    let expired: Vec<u32> = PENDING_TIMERS.with(|m| {
        m.borrow()
            .iter()
            .filter(|(_, t)| t.deadline_ms <= now)
            .map(|(id, _)| *id)
            .collect()
    });

    for id in expired {
        // Remove the entry (intervals will be re-inserted below).
        let entry = PENDING_TIMERS.with(|m| m.borrow_mut().remove(&id));
        if let Some(entry) = entry {
            // Invoke the callback.
            let args: Vec<JsValue> = entry.args.iter().cloned().collect();
            if let Err(e) = entry.func.call(&JsValue::undefined(), &args, ctx) {
                warn!("[Timer id={id}] callback threw: {e}");
            }
            fired += 1;

            // Re-register if this was an interval.
            if let Some(interval_ms) = entry.interval_ms {
                let new_deadline = now + interval_ms;
                // Try to clone the function (JsFunction is Clone in Boa 0.20).
                let new_func = entry.func.clone();
                PENDING_TIMERS.with(|m| {
                    m.borrow_mut().insert(id, TimerEntry {
                        deadline_ms: new_deadline,
                        func: new_func,
                        args: entry.args,
                        interval_ms: Some(interval_ms),
                    });
                });
            }
        }
    }

    fired
}

/// Drain the microtask queue, invoking all pending microtask functions.
/// Should be called after each macrotask (timer callback) to emulate
/// the WHATWG microtask checkpoint.
pub fn drain_microtasks(ctx: &mut Context) -> usize {
    let mut count = 0;
    loop {
        let funcs: Vec<JsFunction> = MICROTASK_QUEUE.with(|q| {
            let mut v = q.borrow_mut();
            let taken = v.drain(..).collect();
            taken
        });
        if funcs.is_empty() { break; }
        for func in funcs {
            if let Err(e) = func.call(&JsValue::undefined(), &[], ctx) {
                warn!("[Microtask] threw: {e}");
            }
            count += 1;
        }
        // Loop: microtasks can enqueue more microtasks.
    }
    count
}

/// Returns true if there are any pending timers or microtasks.
pub fn has_pending_work() -> bool {
    let timers = PENDING_TIMERS.with(|m| !m.borrow().is_empty());
    let microtasks = MICROTASK_QUEUE.with(|q| !q.borrow().is_empty());
    timers || microtasks
}

/// Clear all pending timers (called when a page navigates away / engine resets).
pub fn clear_all_timers() {
    PENDING_TIMERS.with(|m| m.borrow_mut().clear());
    MICROTASK_QUEUE.with(|q| q.borrow_mut().clear());
    info!("All pending timers and microtasks cleared");
}
