use desktop_rpsc::build_router_with_bindings;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // create a path to the bindings file
    let bindings = args
        .get(1)
        .map(|s| {
            let path = std::env::current_dir().expect("failed to get current directory");
            path.join(s)
        })
        .expect("missing bindings file argument");

    // build a router with the given bindings file
    build_router_with_bindings(bindings);
}
