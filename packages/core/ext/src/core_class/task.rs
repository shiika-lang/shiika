use crate::runtime::task;
use shiika_ffi::core_class::{SkBool, SkClass, SkFn, SkInt};
use shiika_ffi_macro::{async_shiika_method, shiika_method};

#[shiika_method("Meta:Task::Scope#_create")]
pub extern "C" fn meta_task_scope_create(_receiver: SkClass) -> SkInt {
    task::create_scope().into()
}

#[shiika_method("Meta:Task::Scope#_spawn")]
pub extern "C" fn meta_task_scope_spawn(_receiver: SkClass, scope_id: SkInt, f: SkFn) -> SkBool {
    task::spawn(scope_id.val(), f).into()
}

#[async_shiika_method("Meta:Task::Scope#_join")]
async fn meta_task_scope_join(_receiver: SkClass, scope_id: SkInt) {
    task::join_scope(scope_id.val()).await;
}
