//! One scenario, one process: the list `set_args` stores is the list the
//! runtime reports, and a second `set_args` changes nothing. Its own test
//! binary because the list is set at most once per process.

use nova_runtime::process::{args, nova_rt_process_arg_count, set_args};

#[test]
fn set_args_fixes_the_list_once() {
    let first = vec![
        "prog.nova".to_string(),
        "one".to_string(),
        "two words".to_string(),
        String::new(),
    ];
    assert!(set_args(first.clone()), "the first set_args takes");
    assert_eq!(args(), first.as_slice());
    assert_eq!(nova_rt_process_arg_count(), 4);
    assert!(
        !set_args(vec!["other".to_string()]),
        "a second set_args is refused"
    );
    assert_eq!(args(), first.as_slice(), "and changes nothing");
}
