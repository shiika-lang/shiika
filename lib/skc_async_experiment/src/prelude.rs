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
        requirement chiika_env_push_frame(env: ENV, n: Int) -> Null
        requirement chiika_env_set(env: ENV, idx: Int, obj: ANY, type_id: Int) -> Null
        requirement chiika_env_pop_frame(env: ENV, expected_len: Int) -> ANY
        requirement chiika_env_get(env: ENV, idx: Int, expected_type_id: Int) -> ANY
        requirement chiika_spawn(f: Fn2<ENV,Fn2<ENV,Null,FUTURE>,FUTURE>) -> Null
        requirement chiika_start_tokio(n: Int) -> Int
        def self.chiika_start_user(env: ENV, cont: Fn2<ENV,Int,FUTURE>) -> FUTURE
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
