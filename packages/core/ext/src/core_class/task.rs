use crate::runtime::task;
use shiika_ffi::core_class::{SkClass, SkFn, SkInt};
use shiika_ffi_macro::{async_shiika_method, shiika_method};

#[shiika_method("Meta:Task::Scope#_create")]
pub extern "C" fn meta_task_scope_create(_receiver: SkClass) -> SkInt {
    task::create_scope().into()
}

/// Returns the id of a new task, or 0 if the scope is already closed.
#[shiika_method("Meta:Task::Scope#_register_task")]
pub extern "C" fn meta_task_scope_register_task(_receiver: SkClass, scope_id: SkInt) -> SkInt {
    task::register_task(scope_id.val()).unwrap_or(0).into()
}

#[shiika_method("Meta:Task::Scope#_start_task")]
pub extern "C" fn meta_task_scope_start_task(
    _receiver: SkClass,
    scope_id: SkInt,
    task_id: SkInt,
    f: SkFn,
) {
    task::start_task(scope_id.val(), task_id.val(), f);
}

#[async_shiika_method("Meta:Task::Scope#_join")]
async fn meta_task_scope_join(_receiver: SkClass, scope_id: SkInt) {
    task::join_scope(scope_id.val()).await;
}

#[async_shiika_method("Meta:Task#_wait")]
async fn meta_task_wait(_receiver: SkClass, task_id: SkInt) {
    task::wait_task(task_id.val()).await;
}
