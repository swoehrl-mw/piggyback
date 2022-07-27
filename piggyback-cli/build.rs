fn main() {
    let tag = match std::env::var("GITHUB_REF") {
        Ok(github_ref) => {
            github_ref.replace("refs/tags/v", "")
        },
        Err(_) => "dev".to_string()
    };
    println!("cargo:rustc-env=GIT_TAG={}", tag);
}
