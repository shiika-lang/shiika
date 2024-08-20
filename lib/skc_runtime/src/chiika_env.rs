use crate::{ChiikaCont, ContFuture};
type ChiikaValue = u64;
type TypeId = u64;
type EnvItem = (ChiikaValue, TypeId);
enum EnvFrame {
    NormalFrame(Vec<Option<EnvItem>>),
    RustFrame(ContFuture),
}

#[repr(C)]
pub struct ChiikaEnv {
    stack: Vec<EnvFrame>,
    pub cont: Option<ChiikaCont>,
}

impl ChiikaEnv {
    pub fn new() -> ChiikaEnv {
        ChiikaEnv {
            stack: vec![],
            cont: None,
        }
    }

    fn current_frame(&mut self) -> &EnvFrame {
        match self.stack.last() {
            Some(v) => v,
            None => panic!("[BUG;ChiikaEnv::current_frame] Stack underflow: no frame there"),
        }
    }

    fn current_frame_mut(&mut self) -> &mut EnvFrame {
        match self.stack.last_mut() {
            Some(v) => v,
            None => panic!("[BUG;ChiikaEnv::current_frame_mut] Stack underflow: no frame there"),
        }
    }

    pub fn push_rust_frame(&mut self, future: ContFuture) {
        self.stack.push(EnvFrame::RustFrame(future));
    }

    pub fn pop_rust_frame(&mut self) -> Option<ContFuture> {
        match self.stack.pop() {
            Some(EnvFrame::RustFrame(future)) => Some(future),
            _ => None,
        }
    }
}

/// Push a frame to the stack.
#[no_mangle]
pub extern "C" fn chiika_env_push_frame(env: *mut ChiikaEnv, size: u64) {
    unsafe {
        let v = std::iter::repeat(None).take(size as usize).collect();
        (*env).stack.push(EnvFrame::NormalFrame(v));
    }
}

/// Push an item to the current frame.
#[no_mangle]
pub extern "C" fn chiika_env_set(env: *mut ChiikaEnv, n: u64, value: ChiikaValue, type_id: TypeId) {
    let frame_ = unsafe { (*env).current_frame_mut() };
    let EnvFrame::NormalFrame(frame) = frame_ else {
        panic!("[BUG;chiika_env_set] Rust frame is on the top");
    };
    if n > (frame.len() as u64) - 1 {
        panic!(
            "[BUG;chiika_env_set] Index out of bounds: n={}, frame_size={}",
            n,
            frame.len()
        );
    }
    frame[n as usize] = Some((value, type_id));
}

/// Pop last frame from the stack and returns its first item.
/// Panics if the frame size is not as expected.
#[no_mangle]
pub extern "C" fn chiika_env_pop_frame(env: *mut ChiikaEnv, expected_len: u64) -> u64 {
    let frame = unsafe { (*env).stack.pop() };
    match frame {
        Some(EnvFrame::NormalFrame(v)) => {
            if v.len() != expected_len as usize {
                panic!(
                    "[BUG;chiika_env_pop_frame] Frame size mismatch: expected size={}, but got size={}",
                    expected_len,
                    v.len()
                );
            }
            v.first().unwrap().unwrap().0
        }
        Some(EnvFrame::RustFrame(_)) => {
            panic!("[BUG;chiika_env_pop_frame] Rust frame is on the top");
        }
        None => panic!("[BUG;chiika_env_pop_frame] Stack underflow: no frame to pop"),
    }
}

/// Peek the n-th last item in the current frame.
#[no_mangle]
pub extern "C" fn chiika_env_ref(env: *mut ChiikaEnv, n: u64, expected_type_id: TypeId) -> u64 {
    let frame_ = unsafe { (*env).current_frame() };
    let EnvFrame::NormalFrame(frame) = frame_ else {
        panic!("[BUG;chiika_env_ref] Rust frame is on the top");
    };
    if n > (frame.len() as u64) - 1 {
        panic!(
            "[BUG;chiika_env_ref] Index out of bounds: n={}, frame_size={}",
            n,
            frame.len()
        );
    }
    let Some((value, type_id)) = frame[n as usize] else {
        panic!("[BUG;chiika_env_ref] value not set at index {n}");
    };
    if type_id != expected_type_id {
        panic!(
            "[BUG;chiika_env_ref] Type mismatch: expected type_id={} for index {n} but got type_id={}",
            expected_type_id, type_id
        );
    }
    value
}
