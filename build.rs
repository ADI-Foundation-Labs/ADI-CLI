fn main() {
    if let Err(_error) = built::write_built_file() {
        // Just continue if build info generation fails
        // This ensures the build doesn't fail due to git info issues
    }
}
