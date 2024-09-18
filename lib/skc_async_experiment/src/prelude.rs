/// Returns the functions needed to run the Milika program.
pub fn prelude_funcs(main_is_async: bool) -> String {
    let main_sig = if main_is_async {
        "requirement __internal__chiika_main(env: ENV, cont: Fn2<ENV, Int, FUTURE>) -> FUTURE"
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
        requirement shiika_malloc(n: Shiika::Internal::Int64) -> ANY
        requirement chiika_env_push_frame(env: ENV, n: Shiika::Internal::Int64) -> Void
        requirement chiika_env_set(env: ENV, idx: Shiika::Internal::Int64, obj: ANY, type_id: Shiika::Internal::Int64) -> Void
        requirement chiika_env_pop_frame(env: ENV, expected_len: Shiika::Internal::Int64) -> ANY
        requirement chiika_env_get(env: ENV, idx: Shiika::Internal::Int64, expected_type_id: Shiika::Internal::Int64) -> ANY
        requirement chiika_spawn(f: Fn2<ENV,Fn2<ENV,Void,FUTURE>,FUTURE>) -> Void
        requirement chiika_start_tokio() -> Void
        def self.chiika_start_user(env: ENV, cont: Fn2<ENV,Int,FUTURE>) -> FUTURE
    " + call_user_main
        + "
        end
        def self.main() -> Int
          chiika_start_tokio()
          return 0
        end
    end
    "
}
