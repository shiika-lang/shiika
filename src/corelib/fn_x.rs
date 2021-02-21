use crate::corelib::*;

macro_rules! fn_item {
    ($i:expr) => {{
        let mut typarams = (1..=$i).map(|i| format!("S{}", i)).collect::<Vec<_>>();
        typarams.push("T".to_string());

        (
            format!("Fn{}", $i),
            Some(class_fullname("Fn")),
            vec![],
            vec![],
            ivars(),
            typarams,
        )
    }};
}

fn ivars() -> HashMap<String, SkIVar> {
    let mut ivars = HashMap::new();
    ivars.insert(
        "@func".to_string(),
        SkIVar {
            name: "@func".to_string(),
            idx: 0,
            ty: ty::raw("Shiika::Internal::Ptr"),
            readonly: true,
        },
    );
    ivars.insert(
        "@the_self".to_string(),
        SkIVar {
            name: "@the_self".to_string(),
            idx: 1,
            ty: ty::raw("Object"),
            readonly: true,
        },
    );
    ivars.insert(
        "@captures".to_string(),
        SkIVar {
            name: "@captures".to_string(),
            idx: 2,
            ty: ty::ary(ty::raw("Shiika::Internal::Ptr")),
            readonly: true,
        },
    );
    ivars.insert(
        "@exit_status".to_string(),
        SkIVar {
            name: "@exit_status".to_string(),
            idx: 3,
            ty: ty::raw("Int"),
            readonly: false,
        },
    );
    ivars
}

pub fn fn_items() -> Vec<ClassItem> {
    vec![
        fn_item!(0),
        fn_item!(1),
        fn_item!(2),
        fn_item!(3),
        fn_item!(4),
        fn_item!(5),
        fn_item!(6),
        fn_item!(7),
        fn_item!(8),
        fn_item!(9),
    ]
}
