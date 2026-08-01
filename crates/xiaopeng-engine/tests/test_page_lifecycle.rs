use xiaopeng_engine::browsing_context::{BrowsingContext, PageLifecycleState};

#[test]
fn test_lifecycle_transitions() {
    let mut context = BrowsingContext::new();
    assert_eq!(context.lifecycle_state, PageLifecycleState::Active);

    context.transition_lifecycle(PageLifecycleState::Hidden);
    assert_eq!(context.lifecycle_state, PageLifecycleState::Hidden);

    context.transition_lifecycle(PageLifecycleState::Frozen);
    assert_eq!(context.lifecycle_state, PageLifecycleState::Frozen);

    context.transition_lifecycle(PageLifecycleState::Terminated);
    assert_eq!(context.lifecycle_state, PageLifecycleState::Terminated);
}
