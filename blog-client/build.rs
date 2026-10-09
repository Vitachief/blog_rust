fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../blog-server/proto/blog.proto"],
            &["../blog-server/proto"],
        )?;
    println!("cargo:rerun-if-changed=../blog-server/proto/blog.proto");
    Ok(())
}