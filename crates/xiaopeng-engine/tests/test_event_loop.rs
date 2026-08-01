use std::rc::Rc;
use std::cell::RefCell;
use xiaopeng_engine::EventLoop;

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
