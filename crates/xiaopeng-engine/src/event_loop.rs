use std::collections::VecDeque;

pub type Task = Box<dyn FnOnce()>;

/// WHATWG Event Loop implementation
pub struct EventLoop {
    /// Macrotasks (e.g., setTimeout, DOM events, networking callbacks)
    task_queue: VecDeque<Task>,
    /// Microtasks (e.g., Promise callbacks, MutationObserver)
    microtask_queue: VecDeque<Task>,
    /// Control flag to stop the event loop
    running: bool,
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
            microtask_queue: VecDeque::new(),
            running: false,
        }
    }

    /// Enqueue a macrotask
    pub fn queue_task<F>(&mut self, task: F)
    where
        F: FnOnce() + 'static,
    {
        self.task_queue.push_back(Box::new(task));
    }

    /// Enqueue a microtask
    pub fn queue_microtask<F>(&mut self, task: F)
    where
        F: FnOnce() + 'static,
    {
        self.microtask_queue.push_back(Box::new(task));
    }

    /// Execute the microtask checkpoint.
    /// Runs all microtasks, including those queued during the execution of other microtasks.
    pub fn perform_microtask_checkpoint(&mut self) {
        while let Some(microtask) = self.microtask_queue.pop_front() {
            microtask();
        }
    }

    /// Execute a single turn (tick) of the event loop
    pub fn step(&mut self, render_callback: &mut dyn FnMut()) {
        // 1. Task Phase: Pick and run the oldest task
        if let Some(task) = self.task_queue.pop_front() {
            task();
        }

        // 2. Microtask Checkpoint Phase: Run all microtasks
        self.perform_microtask_checkpoint();

        // 3. Update the Rendering Phase
        // In a real browser, this evaluates if rendering is needed (e.g. via requestAnimationFrame and 60fps pacing).
        // Here, we invoke the rendering callback on each step to ensure the screen is updated.
        render_callback();
    }

    /// Run the event loop continuously until stopped or empty
    pub fn run(&mut self, mut render_callback: impl FnMut()) {
        self.running = true;
        while self.running {
            if self.task_queue.is_empty() && self.microtask_queue.is_empty() {
                // In a production engine, this would wait/sleep on a condition variable
                // until an external thread (like IO/Network/Input) pushes a new task.
                break;
            }
            self.step(&mut render_callback);
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;

    #[test]
    fn test_event_loop_execution_order() {
        let mut event_loop = EventLoop::new();
        let execution_order = Rc::new(RefCell::new(Vec::new()));

        let order_clone = execution_order.clone();
        event_loop.queue_task(move || {
            order_clone.borrow_mut().push("task 1");
        });

        let order_clone2 = execution_order.clone();
        let el_ptr = &mut event_loop as *mut EventLoop;
        
        event_loop.queue_task(move || {
            order_clone2.borrow_mut().push("task 2");
            // unsafe is just for testing the queueing inside a task without arc/mutex overhead
            unsafe {
                (*el_ptr).queue_microtask({
                    let c = order_clone2.clone();
                    move || c.borrow_mut().push("microtask from task 2")
                });
            }
        });

        let order_clone3 = execution_order.clone();
        event_loop.queue_microtask(move || {
            order_clone3.borrow_mut().push("microtask 1");
        });

        let order_clone4 = execution_order.clone();
        event_loop.queue_microtask(move || {
            order_clone4.borrow_mut().push("microtask 2");
        });

        let render_calls = Rc::new(RefCell::new(0));
        let rc_clone = render_calls.clone();
        
        event_loop.run(move || {
            *rc_clone.borrow_mut() += 1;
        });

        assert_eq!(
            *execution_order.borrow(),
            vec![
                "task 1",
                "microtask 1", 
                "microtask 2",
                "task 2",
                "microtask from task 2"
            ]
        );
        assert_eq!(*render_calls.borrow(), 2);
    }
}
