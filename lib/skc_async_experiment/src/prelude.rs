/// Returns the functions needed to run the Milika program.
pub fn prelude_funcs(main_is_async: bool) -> String {
    let main_sig = if main_is_async {
        "requirement __internal__chiika_main(ENV env, FN((ENV, Int)->FUTURE) cont) -> FUTURE"
    } else {
        "requirement __internal__chiika_main() -> Int"
    };
    let call_user_main = if main_is_async {
        "return chiika_main(env, cont)"
    } else {
        "return cont(env, chiika_main())"
    };
    String::new()
        + "class Main\n"
        + main_sig
        + "
        requirement chiika_env_push_frame(ENV env, Int n) -> Null
        requirement chiika_env_set(ENV env, Int idx, ANY obj, Int type_id) -> Null
        requirement chiika_env_pop_frame(ENV env, Int expected_len) -> ANY
        requirement chiika_env_ref(ENV env, Int idx, Int expected_type_id) -> Int
        requirement chiika_spawn(FN((ENV,FN((ENV,Null)->FUTURE))->FUTURE) f) -> Null
        requirement chiika_start_tokio(Int n) -> Int
        def self.chiika_start_user(ENV env, FN((ENV,Int)->FUTURE) cont) -> FUTURE
    " + call_user_main
        + "
        end
        def self.main() -> Int
          chiika_start_tokio(0)
          return 0
        end
    end
    "
}
