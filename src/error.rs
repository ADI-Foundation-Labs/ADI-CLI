pub use eyre::{Result, WrapErr};

// Re-export eyre's Result as our default Result type
// This allows us to use eyre's error chaining and context features
// while maintaining a consistent error type throughout the application
