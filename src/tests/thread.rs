use std::panic::catch_unwind;

use {Error, Function, Lua, Thread, ThreadStatus};

#[test]
fn test_thread() {
    let lua = Lua::new();
    let thread = lua.create_thread(lua.eval::<Function>(
        r#"
                function (s)
                    local sum = s
                    for i = 1,4 do
                        sum = sum + coroutine.yield(sum)
                    end
                    return sum
                end
            "#,
        None,
    ).unwrap())
        .unwrap();

    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(0).unwrap(), 0);
    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(1).unwrap(), 1);
    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(2).unwrap(), 3);
    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(3).unwrap(), 6);
    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(4).unwrap(), 10);
    assert_eq!(thread.status(), ThreadStatus::Unresumable);

    let accumulate = lua.create_thread(lua.eval::<Function>(
        r#"
                function (sum)
                    while true do
                        sum = sum + coroutine.yield(sum)
                    end
                end
            "#,
        None,
    ).unwrap())
        .unwrap();

    for i in 0..4 {
        accumulate.resume::<_, ()>(i).unwrap();
    }
    assert_eq!(accumulate.resume::<_, i64>(4).unwrap(), 10);
    assert_eq!(accumulate.status(), ThreadStatus::Resumable);
    assert!(accumulate.resume::<_, ()>("error").is_err());
    assert_eq!(accumulate.status(), ThreadStatus::Error);

    let thread = lua.eval::<Thread>(
        r#"
                coroutine.create(function ()
                    while true do
                        coroutine.yield(42)
                    end
                end)
            "#,
        None,
    ).unwrap();
    assert_eq!(thread.status(), ThreadStatus::Resumable);
    assert_eq!(thread.resume::<_, i64>(()).unwrap(), 42);

    let thread: Thread = lua.eval(
        r#"
                coroutine.create(function(arg)
                    assert(arg == 42)
                    local yieldarg = coroutine.yield(123)
                    assert(yieldarg == 43)
                    return 987
                end)
            "#,
        None,
    ).unwrap();

    assert_eq!(thread.resume::<_, u32>(42).unwrap(), 123);
    assert_eq!(thread.resume::<_, u32>(43).unwrap(), 987);

    match thread.resume::<_, u32>(()) {
        Err(Error::CoroutineInactive) => {}
        Err(_) => panic!("resuming dead coroutine error is not CoroutineInactive kind"),
        _ => panic!("resuming dead coroutine did not return error"),
    }
}

#[test]
fn coroutine_from_closure() {
    let lua = Lua::new();
    let thrd_main = lua.create_function(|_, ()| Ok(())).unwrap();
    lua.globals().set("main", thrd_main).unwrap();
    let thrd: Thread = lua.eval("coroutine.create(main)", None).unwrap();
    thrd.resume::<_, ()>(()).unwrap();
}

#[test]
fn coroutine_panic() {
    // check that coroutines propagate panics correctly
    let lua = Lua::new();
    let thrd_main = lua.create_function(|lua, ()| {
        // whoops, 'main' has a wrong type
        let _coro: u32 = lua.globals().get("main").unwrap();
        Ok(())
    }).unwrap();
    lua.globals().set("main", thrd_main.clone()).unwrap();
    let thrd: Thread = lua.create_thread(thrd_main).unwrap();

    match catch_unwind(|| thrd.resume::<_, ()>(())) {
        Ok(r) => panic!("coroutine panic not propagated, instead returned {:?}", r),
        Err(_) => {}
    }
}
