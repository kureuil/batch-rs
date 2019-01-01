//! Phony batch jobs for use in tests.

use batch::job;

/// Standard batch job using default configuration.
#[job(name = "batch-test.standard")]
pub fn standard() {
	println!("Just a happy little job");
}
