fn main() -> std::io::Result<()> {
    // rebuild rust definitions if base changed
    println!("cargo:rerun-if-changed=./proto/**/*.proto");

    // find all protocol files
    let files = glob::glob("./proto/**/*.proto")
        .expect("failed to find protocol files")
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to find protocol files");

    // and compile them
    prost_build::compile_protos(&files, &["proto/"])?;
    Ok(())
}
