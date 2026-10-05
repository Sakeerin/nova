//! One scenario, one process: with nothing set, the runtime reports the OS's
//! own argv, the route a built executable takes. Its own test binary because
//! the first read fixes the list for the rest of the process.

use nova_runtime::process::{args, nova_rt_process_arg_count};

#[test]
fn with_nothing_set_the_list_is_the_os_argv() {
    let list = args();
    let os: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(list, os.as_slice());
    assert_eq!(nova_rt_process_arg_count(), list.len() as i64);
    let exe = std::env::current_exe().expect("current_exe");
    let stem = exe
        .file_stem()
        .expect("an executable has a file name")
        .to_string_lossy()
        .into_owned();
    assert!(
        list[0].contains(&stem),
        "argv[0] {:?} should name this test binary, {stem:?}",
        list[0]
    );
}
