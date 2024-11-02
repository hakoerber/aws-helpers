//! Re-exports selected elements of the EC2 SDK

pub mod ec2 {
    #![expect(clippy::module_name_repetitions, reason = "error prefix is necessary")]
    pub mod error {
        pub use aws_sdk_ec2::error::SdkError;
    }
    pub use aws_sdk_ec2::types::{Filter, InstanceStateName, InstanceType, Tag};
}
