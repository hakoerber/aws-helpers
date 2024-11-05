//! Provides an opinionated interface to the AWS API

extern crate self as aws_lib;

use std::{
    fmt::{self, Debug},
    net,
    time::Duration,
};

use aws_config::retry::RetryConfig;
use aws_sdk_ec2::client::Waiters;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod error;
pub use error::Error;

pub mod tags;
use tags::{ParseTagValueError, RawTag, RawTagValue, Tag, TagKey, TagList};

pub mod export;

macro_rules! wrap_aws_enum {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name(aws_sdk_ec2::types::$name);

        impl $name {
            pub const fn new(from: aws_sdk_ec2::types::$name) -> Self {
                Self(from)
            }

            pub const fn inner(&self) -> &aws_sdk_ec2::types::$name {
                &self.0
            }

            pub fn into_inner(self) -> aws_sdk_ec2::types::$name {
                self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.inner().as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(Self(String::deserialize(deserializer)?.as_str().into()))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.inner().to_string())
            }
        }
    };
}

wrap_aws_enum!(InstanceStateName);
wrap_aws_enum!(InstanceType);

#[derive(Debug)]
pub struct Instance {
    tags: TagList,
    instance_type: InstanceType,
    state: InstanceStateName,
    instance_id: InstanceId,
    image_id: AmiId,
    subnet_id: SubnetId,
    public_ip_address: Option<Ip>,
}

impl Instance {
    pub fn try_from_aws(instance: aws_sdk_ec2::types::Instance) -> Result<Self, Error> {
        macro_rules! extract {
            ($instance:ident, $field:ident) => {
                $instance
                    .$field
                    .clone() // not ideal
                    .ok_or_else(|| Error::UnexpectedNoneValue {
                        entity: stringify!($field).to_owned(),
                    })
            };
        }

        Ok(Self {
            tags: extract!(instance, tags)?.try_into()?,
            instance_type: InstanceType(extract!(instance, instance_type)?),
            state: InstanceStateName(extract!(instance, state)?.name.ok_or_else(|| {
                Error::UnexpectedNoneValue {
                    entity: "state.name".to_owned(),
                }
            })?),
            instance_id: InstanceId(extract!(instance, instance_id)?),
            image_id: AmiId(extract!(instance, image_id)?),
            subnet_id: SubnetId(extract!(instance, subnet_id)?),
            public_ip_address: instance
                .public_ip_address
                .map(|s| -> Result<_, Error> { Ok(Ip(s.parse()?)) })
                .transpose()?,
        })
    }

    pub fn get_tag(&self, key: TagKey) -> Option<&RawTag> {
        self.tags.get(key)
    }

    pub const fn tags(&self) -> &TagList {
        &self.tags
    }

    pub const fn instance_type(&self) -> &InstanceType {
        &self.instance_type
    }

    pub const fn state(&self) -> &InstanceStateName {
        &self.state
    }

    pub const fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    pub const fn image_id(&self) -> &AmiId {
        &self.image_id
    }

    pub const fn subnet_id(&self) -> &SubnetId {
        &self.subnet_id
    }

    pub const fn public_ip_address(&self) -> Option<&Ip> {
        self.public_ip_address.as_ref()
    }

    pub async fn stop(&self, client: &RegionClient) -> Result<(), Error> {
        let _state_change_info = client
            .main
            .ec2
            .stop_instances()
            .instance_ids(self.instance_id().as_str())
            .send()
            .await?;

        Ok(())
    }

    pub async fn wait_for_stop(
        &self,
        client: &RegionClient,
        max_wait: Duration,
    ) -> Result<(), Error> {
        match client
            .main
            .ec2
            .wait_until_instance_stopped()
            .instance_ids(self.instance_id().as_str())
            .wait(max_wait)
            .await
        {
            Ok(_final_response) => Ok(()),
            Err(e) => match e {
                aws_sdk_ec2::waiters::instance_stopped::WaitUntilInstanceStoppedError::ExceededMaxWait(_) => Err(Error::InstanceStopExceededMaxWait { max_wait, instance: self.instance_id().clone()}),
                _ => Err(e.into())
            },
        }?;

        Ok(())
    }

    pub async fn add_tag<T>(&self, client: &RegionClient, tag: Tag<T>) -> Result<(), Error>
    where
        T: Debug + Clone + PartialEq + Eq + Into<String> + Send,
        T: tags::TagValue<T>,
    {
        let _output = client
            .main
            .ec2
            .create_tags()
            .resources(self.instance_id().as_str())
            .tags(tag.into())
            .send()
            .await?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Region {
    #[serde(rename = "eu-central-1")]
    EuCentral1,
    #[serde(rename = "us-east-1")]
    UsEast1,
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct ShieldPop(String);

impl ShieldPop {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Region {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EuCentral1 => "eu-central-1",
            Self::UsEast1 => "us-east-1",
        }
    }

    pub const fn all() -> [Self; 2] {
        [Self::EuCentral1, Self::UsEast1]
    }

    const fn name(self) -> &'static str {
        match self {
            Self::EuCentral1 => "eu-central-1",
            Self::UsEast1 => "us-east-1",
        }
    }

    pub fn cdn_shield_pop(self) -> ShieldPop {
        ShieldPop(
            match self {
                Self::EuCentral1 => "eu-central-1",
                Self::UsEast1 => "us-east-1",
            }
            .to_owned(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RegionClientMain {
    pub ec2: aws_sdk_ec2::Client,
    pub efs: aws_sdk_efs::Client,
    pub route53: aws_sdk_route53::Client,
}

#[derive(Debug, Clone)]
pub struct RegionClientCdn {
    pub cloudfront: aws_sdk_cloudfront::Client,
    pub cloudformation: aws_sdk_cloudformation::Client,
}

#[derive(Debug, Clone)]
pub struct RegionClient {
    pub region: Region,
    pub main: RegionClientMain,
    pub cdn: RegionClientCdn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceProfileName(String);

impl InstanceProfileName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceKeypairName(String);

impl InstanceKeypairName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGroupId(String);

impl SecurityGroupId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGroup {
    id: SecurityGroupId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubnetId(String);

impl SubnetId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub const fn from_string(value: String) -> Self {
        Self(value)
    }
}

impl PartialEq for SubnetId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for SubnetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! string_newtype {
    ($name:ident) => {
        #[Tag(translate = serde)]
        #[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
        pub struct $name(String);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

string_newtype!(AvailabilityZone);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subnet {
    pub id: SubnetId,
    pub availability_zone: AvailabilityZone,
}

impl TryFrom<aws_sdk_ec2::types::Subnet> for Subnet {
    type Error = Error;

    fn try_from(subnet: aws_sdk_ec2::types::Subnet) -> Result<Self, Self::Error> {
        macro_rules! extract {
            ($field:ident) => {
                subnet.$field.ok_or_else(|| Error::UnexpectedNoneValue {
                    entity: stringify!($field).to_owned(),
                })
            };
        }

        Ok(Self {
            id: SubnetId(extract!(subnet_id)?),
            availability_zone: AvailabilityZone(extract!(availability_zone)?),
        })
    }
}

string_newtype!(InstanceId);

impl InstanceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

string_newtype!(AmiId);

impl AmiId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ami {
    pub id: AmiId,
    pub tags: TagList,
    pub creation_date: Timestamp,
}

impl TryFrom<aws_sdk_ec2::types::Image> for Ami {
    type Error = Error;

    fn try_from(image: aws_sdk_ec2::types::Image) -> Result<Self, Self::Error> {
        macro_rules! extract {
            ($field:ident) => {
                image.$field.ok_or_else(|| Error::UnexpectedNoneValue {
                    entity: stringify!($field).to_owned(),
                })
            };
        }

        Ok(Self {
            id: AmiId(extract!(image_id)?),
            tags: extract!(tags)?.try_into()?,
            creation_date: RawImageCreationDate(extract!(creation_date)?).try_into()?,
        })
    }
}

#[Tag(translate = manual)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub const fn new(value: DateTime<Utc>) -> Self {
        Self(value)
    }

    pub const fn inner(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<RawTagValue> for Timestamp {
    type Error = ParseTagValueError;

    fn try_from(value: RawTagValue) -> Result<Self, Self::Error> {
        Ok(Self(
            chrono::NaiveDateTime::parse_from_str(value.as_str(), "%Y-%m-%dT%H:%M:%S")
                .map_err(|e| ParseTagValueError::InvalidValue {
                    value,
                    message: format!("failed parsing timestamp: {e}"),
                })
                .map(|timestamp| timestamp.and_utc())?,
        ))
    }
}

impl From<Timestamp> for RawTagValue {
    fn from(value: Timestamp) -> Self {
        Self::new(value.0.format("%Y-%m-%dT%H:%M:%S").to_string())
    }
}

struct RawImageCreationDate(String);

impl TryFrom<RawImageCreationDate> for Timestamp {
    type Error = Error;

    fn try_from(value: RawImageCreationDate) -> Result<Self, Self::Error> {
        Ok(Self(
            chrono::NaiveDateTime::parse_from_str(&value.0, "%Y-%m-%dT%H:%M:%S.%.3fZ")
                .map_err(|e| Error::InvalidTimestampError {
                    value: value.0,
                    message: format!("failed parsing timestamp: {e}"),
                })
                .map(|timestamp| timestamp.and_utc())?,
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ip(net::IpAddr);

impl Ip {
    pub const fn new(value: net::IpAddr) -> Self {
        Self(value)
    }

    pub fn into_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for Ip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

string_newtype!(EipAllocationId);

impl EipAllocationId {
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Eip {
    pub allocation_id: EipAllocationId,
    pub ip: Ip,
    pub associated_instance: Option<InstanceId>,
}

impl TryFrom<aws_sdk_ec2::types::Address> for Eip {
    type Error = Error;

    fn try_from(address: aws_sdk_ec2::types::Address) -> Result<Self, Self::Error> {
        macro_rules! extract {
            ($field:ident) => {
                address.$field.ok_or_else(|| Error::UnexpectedNoneValue {
                    entity: stringify!($field).to_owned(),
                })
            };
        }

        Ok(Self {
            ip: Ip(extract!(public_ip)?.parse()?),
            associated_instance: address.instance_id.map(InstanceId),
            allocation_id: EipAllocationId(extract!(allocation_id)?),
        })
    }
}

impl Eip {
    pub async fn attach_to_instance(
        &self,
        client: &RegionClient,
        new_instance: &Instance,
    ) -> Result<(), Error> {
        let _association_id = client
            .main
            .ec2
            .associate_address()
            .allocation_id(self.allocation_id.as_str())
            .instance_id(new_instance.instance_id().as_str())
            .send()
            .await?;

        Ok(())
    }

    pub async fn set_tags(&self, client: &RegionClient, tags: TagList) -> Result<(), Error> {
        let _output = client
            .main
            .ec2
            .delete_tags()
            .resources(self.allocation_id.as_str())
            .send()
            .await?;

        let _output = client
            .main
            .ec2
            .create_tags()
            .resources(self.allocation_id.as_str())
            .set_tags(Some(tags.into()))
            .send()
            .await?;

        Ok(())
    }
}

string_newtype!(CloudfrontDistributionId);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudfrontDistributionStatus {
    Deployed,
    Other(String),
}

impl fmt::Display for CloudfrontDistributionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Deployed => "deployed",
                Self::Other(ref s) => &s,
            }
        )
    }
}

impl From<String> for CloudfrontDistributionStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Deployed" => Self::Deployed,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfsId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Efs {
    id: EfsId,
    region: Region,
}

impl Efs {
    pub fn fs_dns_name(&self) -> String {
        format!("{}.efs.{}.amazonaws.com", self.id.0, self.region.as_str())
    }
}

impl TryFrom<(aws_sdk_efs::types::FileSystemDescription, Region)> for Efs {
    type Error = Error;

    fn try_from(
        (efs, region): (aws_sdk_efs::types::FileSystemDescription, Region),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: EfsId(efs.file_system_id),
            region,
        })
    }
}

string_newtype!(CloudfrontDistributionDomain);

impl From<String> for CloudfrontDistributionDomain {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudfrontOrigin {
    id: CloudfrontOriginId,
    domain: CloudfrontOriginDomain,
}

impl CloudfrontOrigin {
    pub const fn id(&self) -> &CloudfrontOriginId {
        &self.id
    }
}

string_newtype!(CloudfrontOriginId);
string_newtype!(CloudfrontOriginDomain);

impl PartialEq<str> for CloudfrontOriginId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl CloudfrontOriginDomain {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CloudfrontOriginDomain {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<aws_sdk_cloudfront::types::Origin> for CloudfrontOrigin {
    fn from(value: aws_sdk_cloudfront::types::Origin) -> Self {
        Self {
            id: CloudfrontOriginId(value.id),
            domain: value.domain_name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudfrontDistribution {
    pub id: CloudfrontDistributionId,
    pub status: CloudfrontDistributionStatus,
    pub domain: CloudfrontDistributionDomain,
    pub origins: Vec<CloudfrontOrigin>,
}

impl TryFrom<aws_sdk_cloudfront::types::DistributionSummary> for CloudfrontDistribution {
    type Error = Error;

    fn try_from(
        distribution: aws_sdk_cloudfront::types::DistributionSummary,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: CloudfrontDistributionId(distribution.id),
            status: distribution.status.into(),
            domain: distribution.domain_name.into(),
            origins: distribution.origins.map_or_else(Vec::new, |origins| {
                origins.items.into_iter().map(Into::into).collect()
            }),
        })
    }
}

impl CloudfrontDistribution {
    pub fn origins(&self) -> &[CloudfrontOrigin] {
        &self.origins
    }

    pub const fn domain(&self) -> &CloudfrontDistributionDomain {
        &self.domain
    }

    pub const fn status(&self) -> &CloudfrontDistributionStatus {
        &self.status
    }
}

#[derive(Clone)]
pub struct ProfileName(String);

impl ProfileName {
    pub const fn new(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone)]
pub struct ProfileConfig {
    pub profile_name_main: ProfileName,
    pub profile_name_cdn: ProfileName,
}

pub async fn load_sdk_clients<const C: usize>(
    regions: [Region; C],
    profile_config: ProfileConfig,
) -> Vec<RegionClient> {
    let mut region_clients = vec![];

    for region in regions {
        let base_config = || {
            aws_config::ConfigLoader::default()
                .retry_config(RetryConfig::standard())
                .stalled_stream_protection(
                    aws_sdk_ec2::config::StalledStreamProtectionConfig::enabled()
                        .grace_period(Duration::from_secs(5))
                        .build(),
                )
                .behavior_version(aws_config::BehaviorVersion::latest())
        };

        let config = base_config()
            .profile_name(&profile_config.profile_name_main.0)
            .region(region.name())
            .load()
            .await;

        let config_cdn = base_config()
            .profile_name(&profile_config.profile_name_cdn.0)
            .region(region.name())
            .load()
            .await;

        // Cloudformation needs always be run in us-east-1
        let config_cloudformation = base_config()
            .profile_name(&profile_config.profile_name_cdn.0)
            .region(Region::UsEast1.as_str())
            .load()
            .await;

        let ec2_client = aws_sdk_ec2::Client::new(&config);
        let cloudfront_client = aws_sdk_cloudfront::Client::new(&config_cdn);
        let efs_client = aws_sdk_efs::Client::new(&config);
        let route53_client = aws_sdk_route53::Client::new(&config);
        let cloudformation_client = aws_sdk_cloudformation::Client::new(&config_cloudformation);

        region_clients.push(RegionClient {
            region,
            main: RegionClientMain {
                ec2: ec2_client,
                efs: efs_client,
                route53: route53_client,
            },
            cdn: RegionClientCdn {
                cloudfront: cloudfront_client,
                cloudformation: cloudformation_client,
            },
        });
    }

    region_clients
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    id: String,
}

impl Account {
    pub const fn new(id: String) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostedZoneId(String);

impl HostedZoneId {
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route53Zone {
    hosted_zone_id: HostedZoneId,
    name: String,
}

impl Route53Zone {
    pub async fn find_by_name(client: &RegionClient, name: &str) -> Result<Option<Self>, Error> {
        Ok(client
            .main
            .route53
            .list_hosted_zones()
            .into_paginator()
            .items()
            .send()
            .try_collect()
            .await?
            .into_iter()
            .filter(|zone| zone.name == name)
            .map(Into::into)
            .next())
    }

    pub const fn new(name: String, hosted_zone_id: HostedZoneId) -> Self {
        Self {
            hosted_zone_id,
            name,
        }
    }
}

impl From<aws_sdk_route53::types::HostedZone> for Route53Zone {
    fn from(zone: aws_sdk_route53::types::HostedZone) -> Self {
        Self {
            hosted_zone_id: HostedZoneId(zone.id),
            name: zone.name,
        }
    }
}

pub struct NewEc2Config<'a> {
    pub ami: &'a Ami,
    pub instance_type: &'a InstanceType,
    pub security_group: &'a SecurityGroup,
    pub instance_profile_name: &'a InstanceProfileName,
    pub instance_keypair_name: &'a InstanceKeypairName,
    pub subnet_id: &'a SubnetId,
    pub user_data: &'a str,
    pub tags: &'a TagList,
}

pub async fn start_ec2_instance<'a>(
    client: &RegionClient,
    ami: &'a Ami,
    instance_type: &'a InstanceType,
    security_group: &'a SecurityGroup,
    instance_profile_name: &'a InstanceProfileName,
    instance_keypair_name: &'a InstanceKeypairName,
    subnet_id: &'a SubnetId,
    user_data: &'a str,
    tags: &'a TagList,
) -> Result<Instance, Error> {
    Instance::try_from_aws(
        client
            .main
            .ec2
            .run_instances()
            .image_id(ami.id.as_str())
            .instance_type(instance_type.clone().into_inner())
            .key_name(instance_keypair_name.as_str())
            .min_count(1)
            .max_count(1)
            .security_group_ids(security_group.id.as_str())
            .subnet_id(subnet_id.as_str())
            .user_data(user_data)
            .tag_specifications(
                aws_sdk_ec2::types::TagSpecification::builder()
                    .resource_type(aws_sdk_ec2::types::ResourceType::Instance)
                    .set_tags(Some(tags.clone().into()))
                    .build(),
            )
            .metadata_options(
                aws_sdk_ec2::types::InstanceMetadataOptionsRequest::builder()
                    .http_tokens(aws_sdk_ec2::types::HttpTokensState::Optional)
                    .http_endpoint(aws_sdk_ec2::types::InstanceMetadataEndpointState::Enabled)
                    .instance_metadata_tags(aws_sdk_ec2::types::InstanceMetadataTagsState::Enabled)
                    .build(),
            )
            .disable_api_termination(true)
            .iam_instance_profile(
                aws_sdk_ec2::types::IamInstanceProfileSpecification::builder()
                    .name(instance_profile_name.as_str())
                    .build(),
            )
            .send()
            .await?
            .instances
            .ok_or(Error::UnexpectedNoneValue {
                entity: "RunInstancesOutput.instances".to_owned(),
            })?
            .pop()
            .ok_or(Error::RunInstancesEmptyResponse)?,
    )
}

pub async fn create_cloudformation_stack(
    client: &RegionClient,
    name: &str,
    template: &str,
    parameters: &CloudformationParameters,
    tags: &TagList,
) -> Result<(), Error> {
    let _create_stack_output = client
        .cdn
        .cloudformation
        .create_stack()
        .stack_name(name)
        .template_body(template)
        .set_parameters(Some(
            parameters
                .0
                .iter()
                .map(|param| {
                    aws_sdk_cloudformation::types::Parameter::builder()
                        .parameter_key(param.key.as_str())
                        .parameter_value(param.value.as_str())
                        .build()
                })
                .collect(),
        ))
        .disable_rollback(true)
        .capabilities(aws_sdk_cloudformation::types::Capability::CapabilityAutoExpand)
        .set_tags(Some(tags.clone().into()))
        .send()
        .await?;

    Ok(())
}

pub struct CloudformationParameter {
    key: String,
    value: String,
}

impl CloudformationParameter {
    pub const fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

pub struct CloudformationParameters(Vec<CloudformationParameter>);

impl CloudformationParameters {
    pub const fn new(value: Vec<CloudformationParameter>) -> Self {
        Self(value)
    }
}

#[expect(
    clippy::missing_panics_doc,
    reason = "only expect() on builder instances"
)]
pub async fn create_route53_record(
    client: &RegionClient,
    eip: &Eip,
    route53_zone: &Route53Zone,
    fqdn: &str,
) -> Result<(), Error> {
    let _change_info = client
        .main
        .route53
        .change_resource_record_sets()
        .hosted_zone_id(route53_zone.hosted_zone_id.as_str())
        .change_batch(
            aws_sdk_route53::types::ChangeBatch::builder()
                .changes(
                    aws_sdk_route53::types::Change::builder()
                        .action(aws_sdk_route53::types::ChangeAction::Create)
                        .resource_record_set(
                            aws_sdk_route53::types::ResourceRecordSet::builder()
                                .name(fqdn)
                                .r#type(aws_sdk_route53::types::RrType::A)
                                .ttl(600)
                                .resource_records(
                                    aws_sdk_route53::types::ResourceRecord::builder()
                                        .value(eip.ip.to_string())
                                        .build()
                                        .expect("builder has missing fields"),
                                )
                                .build()
                                .expect("builder has missing fields"),
                        )
                        .build()
                        .expect("builder has missing fields"),
                )
                .build()
                .expect("builder has missing fields"),
        )
        .send()
        .await?;

    Ok(())
}

pub async fn find_efs(client: &RegionClient, tag: &RawTag) -> Result<Option<Efs>, Error> {
    let mut found = client
        .main
        .efs
        .describe_file_systems()
        .into_paginator()
        .items()
        .send()
        .try_collect()
        .await?
        .into_iter()
        .filter(|fs| fs.tags.iter().any(|t| t == tag))
        .map(|fs| (fs, client.region).try_into())
        .collect::<Result<Vec<Efs>, Error>>()?;

    match (found.len(), found.pop()) {
        (0, _) => Ok(None),
        (1, Some(found)) => Ok(Some(found)),
        _ => Err(Error::MultipleMatches {
            entity: "efs".to_owned(),
        }),
    }
}
