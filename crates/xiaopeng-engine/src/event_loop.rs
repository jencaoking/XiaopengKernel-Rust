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
